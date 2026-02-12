use crate::app::ApplicationConfig;
use crate::app::MemoryCellState;
use crate::app::NonBlockingApplication;
use crate::data::DataNode;
use crate::data::DirEntry;
use crate::data::Directory;
use crate::util::chrono::to_local_date_time;
use egui::CollapsingHeader;
use egui::Popup;
use notes::DEFAULT_ICON;
use notes::Note;
use notes::SCRATCH_PAD_ICON;
use notes::SCRATCH_PAD_NAME;
use phosphor_icons;
use rust_i18n::t;

use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use chrono::DateTime;
use chrono::Utc;

use egui::{self, Label, RichText, ScrollArea, TextEdit, TextStyle, Ui, panel::Side};
use egui::{
    Align, Button, Context, FontData, FontDefinitions, FontFamily, Frame, Layout, Margin,
    TopBottomPanel, Widget, Window,
};

#[derive(Debug)]
pub enum Command {
    ReadAndSelectNote(PathBuf),
    ReadDir(PathBuf),
    CreateNote(PathBuf),
    DeleteNote(PathBuf),
    DeleteDir(PathBuf),
    CreateNoteThenSelect(PathBuf),
    CreateSubDir(PathBuf),
    MarkChanged(PathBuf),
    SaveNote(PathBuf),
}

pub struct NotesApp {
    app: NonBlockingApplication,
    command_queue: VecDeque<Command>,
    ui_state: UiState,
}

pub struct UiState {
    pub explorer_layout: ExplorerLayout,
    pub explorer: bool,
    pub egui_settings: bool,
    pub trash: bool,
}

#[derive(Default)]
pub enum ExplorerLayout {
    Windowed,
    #[default]
    SideBar,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            explorer: true,
            egui_settings: Default::default(),
            explorer_layout: Default::default(),
            trash: Default::default(),
        }
    }
}

/// Create demo instance
impl NotesApp {
    pub fn init() -> Self {
        let app = NonBlockingApplication::init(ApplicationConfig::default()).unwrap();
        Self {
            app,
            command_queue: Default::default(),
            ui_state: Default::default(),
        }
    }
}

impl eframe::App for NotesApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.app.poll_background_tasks();
        // if ctx.input(|i| i.viewport().fullscreen.is_none_or(|fullscreen| !fullscreen)) {
        //     TopBottomPanel::top("native_title_bar_padding_panel")
        //         .frame(Frame::side_top_panel(&ctx.style()).inner_margin(0.))
        //         .exact_height(22.)
        //         .show(ctx, |_ui| {});
        // }

        // Bottom Bar
        TopBottomPanel::bottom("bottom_bar")
            .frame(
                Frame::side_top_panel(&ctx.style()).inner_margin(Margin::symmetric(
                    ctx.style().spacing.item_spacing.x as i8,
                    ctx.style().spacing.item_spacing.y as i8 * 2,
                )),
            )
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    if (ui.input(|i| i.modifiers.ctrl) || self.ui_state.egui_settings)
                        && Button::selectable(self.ui_state.egui_settings, phosphor_icons::WRENCH)
                            .ui(ui)
                            .on_hover_text("Egui Tweaks")
                            .clicked()
                    {
                        self.ui_state.egui_settings = !self.ui_state.egui_settings;
                    }
                    if Button::selectable(self.ui_state.explorer, phosphor_icons::LIST_DASHES)
                        .ui(ui)
                        .on_hover_text(t!("explorer"))
                        .clicked()
                    {
                        self.ui_state.explorer = !self.ui_state.explorer
                    }
                    if Button::selectable(self.ui_state.trash, phosphor_icons::TRASH)
                        .ui(ui)
                        .on_hover_text(t!("trash"))
                        .clicked()
                    {
                        self.ui_state.trash = !self.ui_state.trash;
                    }
                });
            });

        Window::new("Egui Settings")
            .collapsible(true)
            .vscroll(true)
            .open(&mut self.ui_state.egui_settings)
            .show(ctx, |ui| ctx.settings_ui(ui));

        self.trash_ui_windowed(ctx);

        // Draw Explorer
        if self.ui_state.explorer {
            match self.ui_state.explorer_layout {
                ExplorerLayout::Windowed => {
                    egui::Window::new("Explorer Window")
                        .title_bar(false)
                        .resizable(true)
                        .collapsible(false)
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                let default_item_spacing = ui.spacing_mut().item_spacing.x;
                                ui.spacing_mut().item_spacing.x = 0.;
                                if Button::new(phosphor_icons::X)
                                    .ui(ui)
                                    .on_hover_text("Hide")
                                    .clicked()
                                {
                                    self.ui_state.explorer = false;
                                }
                                ui.add_space(default_item_spacing / 2.);
                                ui.spacing_mut().item_spacing.x = default_item_spacing;

                                if Button::new(phosphor_icons::SIDEBAR)
                                    .ui(ui)
                                    .on_hover_text("To side bar")
                                    .clicked()
                                {
                                    self.ui_state.explorer_layout = ExplorerLayout::SideBar;
                                }
                                ui.add(
                                    Label::new(format!(
                                        "{} {}",
                                        phosphor_icons::LIST_DASHES,
                                        t!("explorer")
                                    ))
                                    .selectable(false),
                                );
                            });
                            Self::explorer_ui(&mut self.app, &mut self.command_queue, ui)
                        });
                }
                ExplorerLayout::SideBar => {
                    egui::SidePanel::new(Side::Left, "explorer_side_bar").show(ctx, |ui| {
                        ui.add_space(ui.spacing().icon_spacing);
                        if Button::new(phosphor_icons::CARDS)
                            .ui(ui)
                            .on_hover_text("To window")
                            .clicked()
                        {
                            self.ui_state.explorer_layout = ExplorerLayout::Windowed
                        }
                        Self::explorer_ui(&mut self.app, &mut self.command_queue, ui);
                    });
                }
            }
        }

        // Note View/Edit Panel
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0))
            .show(ctx, |ui| {
                // Draw Status Bar
                {
                    egui::TopBottomPanel::top("status_bar_panel")
                        .frame(
                            Frame::side_top_panel(&ctx.style()).inner_margin(Margin::symmetric(
                                ctx.style().spacing.item_spacing.x as i8,
                                ctx.style().spacing.item_spacing.y as i8 * 2,
                            )),
                        )
                        .show_inside(ui, |ui| Self::path_bar_ui(&self.app, ui));
                }

                // Draw Title and Editor
                {
                    egui::CentralPanel::default()
                        .frame(
                            Frame::central_panel(&ctx.style())
                                .inner_margin(Margin::symmetric(50, 0)),
                        )
                        .show_inside(ui, |ui| self.note_content_ui(ui))
                }
            });

        while let Some(command) = self.command_queue.pop_front() {
            handle_command(&mut self.app, command);
        }
    }
}

/// UI drawing
impl NotesApp {
    /// Setup default font and icon font
    pub fn setup_fonts(ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        // Regular
        let ibm_plex = "IBM Plex Sans";
        fonts.font_data.insert(
            ibm_plex.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/IBMPlexSans-VariableFont_wdth,wght.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, ibm_plex.to_owned());

        // Mono
        let jet_brains_mono = "JetBrains Mono";
        fonts.font_data.insert(
            jet_brains_mono.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/JetBrainsMono-VariableFont_wght.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, jet_brains_mono.to_owned());

        // Icons
        let phosphor = "Phosphor";
        fonts.font_data.insert(
            phosphor.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/fonts/Phosphor.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(1, phosphor.to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(1, phosphor.to_owned());

        ctx.set_fonts(fonts);
    }

    fn explorer_ui(
        app: &NonBlockingApplication,
        command_queue: &mut VecDeque<Command>,
        ui: &mut Ui,
    ) {
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            ScrollArea::vertical()
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    {
                        let selected = app.is_selected(&app.scratch_pad_path());
                        if ui.add(scratch_pad_label(selected)).clicked() {
                            command_queue.push_back(Command::ReadAndSelectNote(
                                app.scratch_pad_path().to_path_buf(),
                            ));
                        }
                    }

                    ui.separator();
                    let dir_action = create_action_buttons_ui(ui, app.base_dir_path());
                    // in root switch to created note
                    let dir_action = dir_action.map(|action| match action {
                        Command::CreateNote(dir) => Command::CreateNoteThenSelect(dir),
                        _ => action,
                    });
                    if let Some(action) = dir_action {
                        command_queue.push_back(action);
                    }

                    let mut add_actions = VecDeque::new();
                    if let Some(root) = app.base_dir() {
                        explorer_folder_content_ui(&app, ui, root, &mut add_actions, false);
                        command_queue.extend(add_actions);
                    } else {
                        command_queue
                            .push_back(Command::ReadDir(app.base_dir_path().to_path_buf()));
                    }
                });
        });
    }

    fn path_bar_ui(app: &NonBlockingApplication, ui: &mut Ui) {
        let layout = Layout::left_to_right(Align::TOP).with_main_align(Align::LEFT);
        ui.with_layout(layout, |ui| {
            Label::new(RichText::new(app.current_note_path().to_string_lossy()))
                .selectable(false)
                .ui(ui);
        });
    }

    // fn title_ui(&mut self, ui: &mut Ui) {
    //     let layout = Layout::top_down_justified(Align::LEFT);
    //     ui.with_layout(layout, |ui| {
    //         let current_note_id = self.state.current_note_id;
    //         let node = self.state.lookup_current_note().node;
    //         let note = &node.data;
    //
    //         self.state.get_item_path_str(current_note_id).map(|path| {
    //             ui.weak(path);
    //             ui.add_space(item_spacing(ui.ctx(), &layout));
    //         });
    //
    //         let mod_date = format_date_time(&node.modification_time);
    //         ui.weak(format!("{} {}", t!("modified"), mod_date));
    //
    //         ui.add_space(item_spacing(ui.ctx(), &layout));
    //
    //         let icon_label = Label::new(RichText::new(note.icon()).heading());
    //         let title_text = match self.state.lookup_current_note_content() {
    //             MaybeMutRef::Mut(content) => &mut content.title,
    //             MaybeMutRef::Immut(content) => &mut content.title.clone(),
    //         };
    //         let title_text_edit = TextEdit::multiline(title_text)
    //             .desired_rows(1)
    //             .clip_text(false)
    //             .font(TextStyle::Heading)
    //             .frame(false)
    //             .clip_text(true)
    //             .return_key(None)
    //             .background_color(ui.visuals().panel_fill);
    //
    //         ui.add(icon_label);
    //
    //         let title_edit_response = ui.add(title_text_edit);
    //         if title_edit_response.changed() {
    //             self.state.touch_current_note();
    //         }
    //
    //         let switch_focus = title_edit_response
    //             .ctx
    //             .input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Tab));
    //         if switch_focus {
    //             title_edit_response.surrender_focus();
    //         }
    //         ui.add_space(item_spacing(ui.ctx(), &layout));
    //     });
    // }

    fn note_content_ui(&mut self, ui: &mut Ui) {
        let note_path = self.app.current_note_path().to_owned();
        let note_state = self.app.note_state(&note_path);
        match note_state {
            Some(MemoryCellState::Ready) => {
                let current_note = self.app.get_note_mut(&note_path).unwrap();
                let _scroll_area = ScrollArea::both().stick_to_bottom(false).show(ui, |ui| {
                    ui.add_space(ui.spacing().item_spacing.y);

                    if TextEdit::multiline(&mut current_note.data.text)
                        .desired_width(f32::INFINITY)
                        .font(TextStyle::Body)
                        .background_color(ui.visuals().panel_fill)
                        .lock_focus(true)
                        .desired_rows(5)
                        .clip_text(false)
                        .frame(false)
                        .ui(ui)
                        .changed()
                    {
                        self.command_queue
                            .push_back(Command::MarkChanged(note_path.to_path_buf()));
                        self.command_queue
                            .push_back(Command::SaveNote(note_path.to_path_buf()));
                    }
                });
            }
            Some(MemoryCellState::PendingRead) => {
                ui.label("Loading...");
            }
            Some(MemoryCellState::Error) => {
                todo!("Display IO Error")
            }
            None => {
                self.command_queue
                    .push_back(Command::ReadAndSelectNote(note_path.to_path_buf()));
            }
        }
    }

    fn trash_ui_windowed(&mut self, ctx: &Context) {
        Window::new(t!("trash"))
            .collapsible(true)
            .vscroll(true)
            .open(&mut self.ui_state.trash)
            .show(ctx, |ui| { /*TODO*/ });
    }
}

fn date_time_fmt() -> &'static str {
    static DATE_TIME_FMT: LazyLock<String> =
        std::sync::LazyLock::new(|| format!("%d.%m.%Y {} %H:%M", t!("at")));
    &DATE_TIME_FMT
}

fn format_date_time(date: &DateTime<Utc>) -> String {
    to_local_date_time(date).format(date_time_fmt()).to_string()
}

fn trash_label_ui(
    ui: &mut Ui,
    selected: &mut bool,
    restore: &mut bool,
    trashed: &Note,
) -> egui::Response {
    // let label = ui.add(note_label(*selected, todo!("note_name_in_dir"), trashed));
    // label.context_menu(|ui| {
    //     if ui
    //         .button(format!(
    //             "{} {}",
    //             phosphor::ARROW_CCW,
    //             t!("restore_from_trash")
    //         ))
    //         .clicked()
    //     {
    //         *restore = true;
    //         ui.close();
    //     }
    // });
    // if label.clicked() {
    //     *selected = true
    // }
    // label
    todo!()
}

fn explorer_note_label_ui(
    ui: &mut Ui,
    selected: bool,
    note_name_in_dir: &str,
    note_path: &Path,
) -> (egui::Response, VecDeque<Command>) {
    let mut commands = VecDeque::new();
    let label = ui.add(note_label(selected, note_name_in_dir));
    label.context_menu(|ui| {
        if ui
            .button(format!("{} {}", phosphor_icons::TRASH, t!("trash_note")))
            .clicked()
        {
            commands.push_back(Command::DeleteNote(note_path.to_path_buf()));
            ui.close();
        }
    });
    if label.clicked() && !selected {
        commands.push_back(Command::ReadAndSelectNote(note_path.to_path_buf()));
    }
    (label, commands)
}

fn note_label<'x>(selected: bool, note_name_in_dir: &str) -> Button<'x> {
    let mut label_text = RichText::new(format!("{} {}", DEFAULT_ICON, &note_name_in_dir,));

    if selected {
        label_text = label_text.strong();
    }

    Button::selectable(selected, label_text)
}

fn scratch_pad_label<'x>(selected: bool) -> Button<'x> {
    let mut label_text = RichText::new(format!("{} {}", SCRATCH_PAD_ICON, SCRATCH_PAD_NAME));

    if selected {
        label_text = label_text.strong();
    }

    Button::selectable(selected, label_text)
}

fn create_action_buttons_ui(ui: &mut Ui, dir_path: &Path) -> Option<Command> {
    let mut action = None;
    if ui
        .button(format!("{} {}", phosphor_icons::PLUS, t!("new_note")))
        .clicked()
    {
        action = Command::CreateNoteThenSelect(dir_path.to_path_buf()).into();
    }
    if ui
        .button(format!("{} {}", phosphor_icons::PLUS, t!("new_folder")))
        .clicked()
    {
        action = Command::CreateSubDir(dir_path.to_path_buf()).into()
    }
    action
}

fn dir_action_buttons_ui(ui: &mut Ui, dir_path: &Path) -> Option<Command> {
    let mut action = create_action_buttons_ui(ui, dir_path);
    if ui
        .button(format!("{} {}", phosphor_icons::TRASH, t!("trash_note")))
        .clicked()
    {
        action = Some(Command::DeleteDir(dir_path.to_path_buf()));
    }
    action
}

fn explorer_folder_ui(
    app: &NonBlockingApplication,
    ui: &mut Ui,
    dir_name: &str,
    dir_path: &Path,
    command_queue: &mut VecDeque<Command>,
) {
    ui.horizontal(|ui| {
        let collapsing = CollapsingHeader::new(dir_name)
            .id_salt(&dir_path)
            .show(ui, |ui| {
                if let Some(dir) = app.get_dir(dir_path) {
                    explorer_folder_content_ui(app, ui, dir, command_queue, false);
                }
            });
        if collapsing.header_response.clicked() {
            command_queue.push_back(Command::ReadDir(dir_path.to_path_buf()));
        }
        [Some(collapsing.header_response), collapsing.body_response]
            .iter()
            .flatten()
            .for_each(|response| {
                Popup::context_menu(response).show(|ui| {
                    if let Some(dir_action) = dir_action_buttons_ui(ui, &dir_path) {
                        command_queue.push_back(dir_action);
                    }
                });
            });
    });
}

fn explorer_folder_content_ui(
    app: &NonBlockingApplication,
    ui: &mut Ui,
    dir: &DataNode<Directory>,
    command_queue: &mut VecDeque<Command>,
    show_hidden: bool,
) {
    let mut notes = dir
        .data
        .entries
        .iter()
        .filter_map(|(name, ent)| {
            if let DirEntry::File(path) = ent
                && (show_hidden || !path.file_name().unwrap().to_string_lossy().starts_with("."))
            {
                Some((name.as_str(), path.as_path()))
            } else {
                None
            }
        })
        .collect::<Vec<(&str, &Path)>>();
    notes.sort_by_key(|(name, _)| name.to_owned());
    notes.into_iter().for_each(|(note_name, note_path)| {
        let selected = app.is_selected(note_path);
        let (_, commands) = explorer_note_label_ui(ui, selected, note_name, note_path);
        command_queue.extend(commands);
    });
    let mut sub_folders = dir
        .data
        .entries
        .iter()
        .filter_map(|(name, ent)| {
            if let DirEntry::Dir(path) = ent {
                Some((name.as_str(), path.as_path()))
            } else {
                None
            }
        })
        .collect::<Vec<(&str, &Path)>>();
    sub_folders.sort_by_key(|(name, _path)| name.to_owned());
    sub_folders.iter().for_each(|(name, path)| {
        explorer_folder_ui(app, ui, name, path, command_queue);
    });
}

fn handle_command(app: &mut NonBlockingApplication, command: Command) {
    match command {
        Command::ReadAndSelectNote(path_buf) => {
            app.read_note_in_background(&path_buf);
            app.set_current_note_path(path_buf);
        }
        Command::ReadDir(path_buf) => {
            if !app.dir_in_memory(&path_buf) {
                app.read_dir_in_background(&path_buf);
            }
        }
        Command::CreateNote(path_buf) => todo!(),
        Command::DeleteNote(path_buf) => todo!(),
        Command::DeleteDir(path_buf) => todo!(),
        Command::CreateNoteThenSelect(path_buf) => todo!(),
        Command::CreateSubDir(path_buf) => todo!(),
        Command::MarkChanged(path_buf) => {
            app.set_dirty(&path_buf);
        }
        Command::SaveNote(path_buf) => {
            if app.note_is_dirty(&path_buf) {
                app.save_note_in_background(&path_buf);
            }
        }
    }
}
