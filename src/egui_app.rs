use crate::app::AppState;
use crate::app::ContentLookup;
use crate::app::DisplayType;
use crate::app::NoteLookup;
use crate::app::as_note;
use crate::font_icons::phosphor;
use crate::note::Note;
use crate::note::SCRATCH_PAD_NAME;
use crate::storage::Directory;
use crate::storage::ObjectId;
use crate::storage::as_dir;
use crate::util::chrono::to_local_date_time;
use crate::util::egui::item_spacing;
use egui::CollapsingHeader;
use egui::Color32;
use egui::Popup;
use rust_i18n::t;

use std::collections::VecDeque;
use std::sync::{Arc, LazyLock};

use chrono::DateTime;
use chrono::Utc;

use egui::{self, Label, RichText, ScrollArea, TextEdit, TextStyle, Ui, panel::Side};
use egui::{
    Align, Button, Context, FontData, FontDefinitions, FontFamily, Frame, Layout, Margin,
    TopBottomPanel, Widget, Window,
};

fn gen_sample_text(lines_count: i32) -> String {
    (0..lines_count)
        .map(|number| format!("Sample text line. The line number is {number}\n"))
        .collect()
}

pub struct NotesApp {
    state: AppState,
    command_queue: VecDeque<UiCommand>,
    ui_state: UiState,
}

pub struct UiState {
    pub explorer_layout: ExplorerLayout,
    pub explorer: bool,
    pub content_is_scrolled: bool,
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
            content_is_scrolled: Default::default(),
            egui_settings: Default::default(),
            explorer_layout: Default::default(),
            trash: Default::default(),
        }
    }
}

#[derive(Debug)]
enum UiCommand {
    FolderTarget(FolderAction),
    NoteTarget(NoteAction),
}

#[derive(Debug)]
enum FolderAction {
    CreateSubFolder(ObjectId),
    CreateNote(ObjectId),
    CreateNoteThenSelect(ObjectId),
    Delete(ObjectId),
}

#[derive(Debug)]
enum NoteAction {
    Select(ObjectId),
    Delete(String, ObjectId),
}

/// Create demo instance
impl NotesApp {
    pub fn demo() -> Self {
        let mut state = AppState::initial();
        state.scratch_pad_mut().text = gen_sample_text(100);
        Self {
            state,
            command_queue: Default::default(),
            ui_state: Default::default(),
        }
    }
}

impl eframe::App for NotesApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
                        && Button::selectable(self.ui_state.egui_settings, phosphor::WRENCH)
                            .ui(ui)
                            .on_hover_text("Egui Tweaks")
                            .clicked()
                    {
                        self.ui_state.egui_settings = !self.ui_state.egui_settings;
                    }
                    if Button::selectable(self.ui_state.explorer, phosphor::LIST_DASHES)
                        .ui(ui)
                        .on_hover_text(t!("explorer"))
                        .clicked()
                    {
                        self.ui_state.explorer = !self.ui_state.explorer
                    }
                    if Button::selectable(self.ui_state.trash, phosphor::TRASH)
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
                                if Button::new(phosphor::X)
                                    .ui(ui)
                                    .on_hover_text("Hide")
                                    .clicked()
                                {
                                    self.ui_state.explorer = false;
                                }
                                ui.add_space(default_item_spacing / 2.);
                                ui.spacing_mut().item_spacing.x = default_item_spacing;

                                if Button::new(phosphor::SIDEBAR)
                                    .ui(ui)
                                    .on_hover_text("To side bar")
                                    .clicked()
                                {
                                    self.ui_state.explorer_layout = ExplorerLayout::SideBar;
                                }
                                ui.add(
                                    Label::new(format!(
                                        "{} {}",
                                        phosphor::LIST_DASHES,
                                        t!("explorer")
                                    ))
                                    .selectable(false),
                                );
                            });
                            self.explorer_ui(ui)
                        });
                }
                ExplorerLayout::SideBar => {
                    egui::SidePanel::new(Side::Left, "explorer_side_bar").show(ctx, |ui| {
                        ui.add_space(ui.spacing().icon_spacing);
                        if Button::new(phosphor::CARDS)
                            .ui(ui)
                            .on_hover_text("To window")
                            .clicked()
                        {
                            self.ui_state.explorer_layout = ExplorerLayout::Windowed
                        }
                        self.explorer_ui(ui);
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
                    let status_bar_visible = self.ui_state.content_is_scrolled;
                    let opacity_anim =
                        ctx.animate_bool("status_bar_opacity".into(), status_bar_visible);

                    let default_item_spacing =
                        std::mem::replace(&mut ui.spacing_mut().item_spacing.y, 0.);
                    ui.scope(|ui| {
                        ui.set_opacity(opacity_anim);
                        egui::TopBottomPanel::top("status_bar_panel")
                            .frame(Frame::side_top_panel(&ctx.style()).inner_margin(
                                Margin::symmetric(
                                    ctx.style().spacing.item_spacing.x as i8,
                                    ctx.style().spacing.item_spacing.y as i8 * 2,
                                ),
                            ))
                            .show_inside(ui, |ui| self.status_bar_ui(ui));
                    });
                    ui.spacing_mut().item_spacing.y = default_item_spacing;
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
            self.handle_explorer_action(command);
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

    fn explorer_ui(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            ScrollArea::vertical()
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    {
                        let (note_id, note) =
                            (self.state.scratch_pad_id, &self.state.scratch_pad());
                        let selected = self.state.current_note_id == note_id;
                        if ui
                            .add(scratch_pad_label(selected, SCRATCH_PAD_NAME, note))
                            .clicked()
                        {
                            self.command_queue
                                .push_back(UiCommand::NoteTarget(NoteAction::Select(note_id)));
                        }
                    }

                    ui.separator();
                    let folder_action =
                        create_action_buttons_ui(ui, self.state.storage.root_dir_id());
                    // in root switch to created note
                    let folder_action = folder_action.map(|action| match action {
                        FolderAction::CreateNote(folder) => {
                            FolderAction::CreateNoteThenSelect(folder)
                        }
                        _ => action,
                    });
                    if let Some(action) = folder_action {
                        self.command_queue
                            .push_back(UiCommand::FolderTarget(action));
                    }

                    let mut commands = VecDeque::new();
                    self.explorer_folder_content_ui(
                        ui,
                        self.state.storage.root_dir_id(),
                        self.state.storage.root_dir(),
                        &mut commands,
                    );
                    self.command_queue.extend(commands);
                });
        });
    }

    fn handle_explorer_action(&mut self, action: UiCommand) {
        match action {
            UiCommand::NoteTarget(NoteAction::Select(note_id)) => {
                self.state.current_note_id = note_id
            }
            UiCommand::NoteTarget(NoteAction::Delete(item_name, item_id)) => {
                self.state.delete_object(item_id, item_name)
            }
            UiCommand::FolderTarget(FolderAction::CreateSubFolder(parent_folder_id)) => {
                self.state.new_folder(parent_folder_id)
            }
            UiCommand::FolderTarget(FolderAction::CreateNote(parent_folder_id)) => {
                self.state.new_note(parent_folder_id)
            }
            UiCommand::FolderTarget(FolderAction::CreateNoteThenSelect(folder_id)) => {
                self.state.new_note_then_switch(folder_id)
            }
            UiCommand::FolderTarget(FolderAction::Delete(folder_id)) => {
                self.state.delete_dir(folder_id);
            }
        }
    }

    fn explorer_folder_ui(
        &self,
        ui: &mut Ui,
        folder_id: ObjectId,
        folder: &Directory,
        command_queue: &mut VecDeque<UiCommand>,
    ) {
        ui.horizontal(|ui| {
            let collapsing =
                CollapsingHeader::new(&folder.name)
                    .id_salt(folder_id)
                    .show(ui, |ui| {
                        self.explorer_folder_content_ui(ui, folder_id, folder, command_queue);
                    });
            [Some(collapsing.header_response), collapsing.body_response]
                .iter()
                .flatten()
                .for_each(|response| {
                    Popup::context_menu(response).show(|ui| {
                        if let Some(folder_action) = folder_action_buttons_ui(ui, folder_id) {
                            command_queue.push_back(folder_action);
                        }
                    });
                });
        });
    }

    fn explorer_folder_content_ui(
        &self,
        ui: &mut Ui,
        folder_id: ObjectId,
        folder: &Directory,
        command_queue: &mut VecDeque<UiCommand>,
    ) {
        let show_deleted = folder_id == self.state.trash_dir_id;
        let mut notes = folder
            .entries
            .iter()
            .map(|(name, &id)| (name.as_str(), id))
            .collect::<Vec<(&str, ObjectId)>>();
        notes.sort();
        notes
            .into_iter()
            .map(|(note_name, note_id)| {
                (
                    note_id,
                    note_name,
                    self.state.storage.get_object(note_id).unwrap(),
                )
            })
            .filter(|(_, _, node)| node.is_deleted() == show_deleted)
            .map(|(id, name, node)| (id, name, as_note(&node.data).expect("Must be note")))
            .for_each(|(note_id, note_name, node)| {
                let was_selected = self.state.current_note_id == note_id;
                let mut selected = was_selected;
                let mut remove = false;
                explorer_note_label_ui(ui, &mut selected, &mut remove, note_name, node);
                if selected && !was_selected {
                    command_queue.push_back(UiCommand::NoteTarget(NoteAction::Select(note_id)));
                }
                if remove {
                    command_queue.push_back(UiCommand::NoteTarget(NoteAction::Delete(
                        note_name.to_owned(),
                        note_id,
                    )));
                }
            });
        let mut sub_folders = self.state.storage.get_sub_directories(folder_id).unwrap();
        sub_folders
            .sort_by_key(|folder| as_dir(&folder.data).expect("Must be dir").name.to_owned());
        sub_folders.into_iter().for_each(|sub_folder| {
            self.explorer_folder_ui(
                ui,
                sub_folder.id(),
                as_dir(&sub_folder.data).expect("Must be dir"),
                command_queue,
            );
        });
    }

    fn status_bar_ui(&self, ui: &mut Ui) {
        let layout = Layout::left_to_right(Align::TOP).with_main_align(Align::LEFT);
        let current_note_id = self.state.current_note_id;
        ui.with_layout(layout, |ui| {
            let current = self.state.lookup_current_note();
            let note = &current.note;
            match current.display_type {
                DisplayType::Default => {
                    let path = self
                        .state
                        .get_item_path_str(current_note_id)
                        .expect("Path for note in tree must be present");
                    let text = format!("{}/ {} {}", path, &note.icon(), &note.title);
                    Label::new(RichText::new(text)).selectable(false).ui(ui);
                }
                DisplayType::ScratchPad => {
                    let text = format!("{} {}", &note.icon(), &note.title);
                    Label::new(RichText::new(text)).selectable(false).ui(ui);
                }
                DisplayType::Deleted => {
                    let text = format!(
                        "{} {} / {} {}",
                        phosphor::TRASH,
                        t!("trash"),
                        &note.icon(),
                        &note.title
                    );
                    Label::new(RichText::new(text).color(Color32::RED))
                        .selectable(false)
                        .ui(ui);
                }
            }
        });
    }

    fn title_ui(&mut self, ui: &mut Ui) {
        let layout = Layout::top_down_justified(Align::LEFT);
        ui.with_layout(layout, |ui| {
            let current_note_id = self.state.current_note_id;
            let node = self
                .state
                .storage
                .get_object(current_note_id)
                .expect("Must be in storage");
            let note = as_note(&node.data).expect("Must be note");

            self.state.get_item_path_str(current_note_id).map(|path| {
                ui.weak(path);
                ui.add_space(item_spacing(ui.ctx(), &layout));
            });

            let mod_date = format_date_time(&node.modification_time);
            ui.weak(format!("{} {}", t!("modified"), mod_date));

            ui.add_space(item_spacing(ui.ctx(), &layout));

            let icon_label = Label::new(RichText::new(note.icon()).heading());
            let title_text = match self.state.lookup_current_note_content() {
                ContentLookup::Mut(content) => &mut content.title,
                ContentLookup::Immut(content) => &mut content.title.clone(),
            };
            let title_text_edit = TextEdit::singleline(title_text)
                .desired_rows(1)
                .clip_text(false)
                .font(TextStyle::Heading)
                .frame(false)
                .background_color(ui.visuals().panel_fill);

            ui.add(icon_label);

            let mut title_changed = false;
            ScrollArea::horizontal()
                .stick_to_right(false)
                .show(ui, |ui| {
                    if ui.add(title_text_edit).changed() {
                        title_changed = true;
                    }
                });
            if title_changed {
                self.state.touch_current_note();
            }

            ui.add_space(item_spacing(ui.ctx(), &layout));
        });
    }

    fn note_content_ui(&mut self, ui: &mut Ui) {
        let scroll_area = ScrollArea::both().stick_to_bottom(false).show(ui, |ui| {
            ui.add_space(ui.spacing().item_spacing.y);
            if let NoteLookup {
                node,
                note: _,
                display_type: DisplayType::Deleted,
            } = self.state.lookup_current_note()
            {
                let trash_put_time = format_date_time(
                    &node
                        .deletion_time
                        .expect("Deletion time must be present as node is deleted"),
                );
                ui.label(
                    RichText::new(format!(
                        "{} {} {}",
                        phosphor::INFO,
                        t!("note_added_to_trash"),
                        trash_put_time
                    ))
                    .color(Color32::RED),
                );
                if ui
                    .button(format!(
                        "{} {}",
                        phosphor::ARROW_CCW,
                        t!("restore_from_trash")
                    ))
                    .clicked()
                {
                    self.state.restore_object(node.id());
                }
            }

            let content = self.state.lookup_current_note_content();
            let mutable = content.is_mut();
            ui.add_enabled_ui(mutable, |ui| {
                self.title_ui(ui);
                ui.separator();

                let content = self.state.lookup_current_note_content();
                let note_text = match content {
                    ContentLookup::Mut(content) => &mut content.text,
                    ContentLookup::Immut(content) => &mut content.text.clone(),
                };

                if TextEdit::multiline(note_text)
                    .interactive(mutable)
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
                    self.state.touch_current_note();
                }
            })
        });
        let scrolled = scroll_area.state.offset.y > 0.0;
        self.ui_state.content_is_scrolled = scrolled;
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
    let label = ui.add(note_label(*selected, todo!("note_name_in_dir"), trashed));
    label.context_menu(|ui| {
        if ui
            .button(format!(
                "{} {}",
                phosphor::ARROW_CCW,
                t!("restore_from_trash")
            ))
            .clicked()
        {
            *restore = true;
            ui.close();
        }
    });
    if label.clicked() {
        *selected = true
    }
    label
}

fn explorer_note_label_ui(
    ui: &mut Ui,
    selected: &mut bool,
    remove: &mut bool,
    note_name_in_dir: &str,
    note: &Note,
) -> egui::Response {
    let label = ui.add(note_label(*selected, note_name_in_dir, note));
    label.context_menu(|ui| {
        if ui
            .button(format!("{} {}", phosphor::TRASH, t!("trash_note")))
            .clicked()
        {
            *remove = true;
            ui.close();
        }
    });
    if label.clicked() {
        *selected = true
    }
    label
}

fn note_label<'x>(selected: bool, note_name_in_dir: &str, note: &'x Note) -> Button<'x> {
    let mut label_text = RichText::new(format!(
        "{} {} - {}",
        note.icon(),
        &note_name_in_dir,
        &note.title,
    ));

    if selected {
        label_text = label_text.strong();
    }

    Button::selectable(selected, label_text)
}

fn scratch_pad_label<'x>(selected: bool, note_name_in_dir: &str, note: &'x Note) -> Button<'x> {
    let mut label_text = RichText::new(format!("{} {}", note.icon(), &note_name_in_dir,));

    if selected {
        label_text = label_text.strong();
    }

    Button::selectable(selected, label_text)
}

fn create_action_buttons_ui(ui: &mut Ui, folder_id: u64) -> Option<FolderAction> {
    let mut action = None;
    if ui
        .button(format!("{} {}", phosphor::PLUS, t!("new_note")))
        .clicked()
    {
        action = FolderAction::CreateNote(folder_id).into();
    }
    if ui
        .button(format!("{} {}", phosphor::PLUS, t!("new_folder")))
        .clicked()
    {
        action = FolderAction::CreateSubFolder(folder_id).into()
    }
    action
}

fn folder_action_buttons_ui(ui: &mut Ui, folder_id: u64) -> Option<UiCommand> {
    let mut action = None;
    let create_action = create_action_buttons_ui(ui, folder_id);
    if let Some(create_action) = create_action {
        action = UiCommand::FolderTarget(create_action).into();
    }
    if ui
        .button(format!("{} {}", phosphor::TRASH, t!("trash_note")))
        .clicked()
    {
        action = UiCommand::FolderTarget(FolderAction::Delete(folder_id)).into();
    }
    action
}
