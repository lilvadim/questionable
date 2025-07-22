use crate::app::AppState;
use crate::app::ContentLookup;
use crate::app::NoteLookup;
use crate::font_icons::phosphor;
use crate::note::Note;
use crate::note_tree::NoteFolderTree;
use crate::trash::Trashed;
use crate::util::chrono::to_local_date_time;
use crate::util::egui::item_spacing;
use egui::CollapsingHeader;
use egui::Color32;
use rust_i18n::t;

use std::sync::{Arc, LazyLock};

use chrono::DateTime;
use chrono::Utc;

use egui::{self, Label, RichText, ScrollArea, TextEdit, TextStyle, Ui, panel::Side};
use egui::{
    Align, Button, Context, FontData, FontDefinitions, FontFamily, Frame, Layout, Margin,
    SelectableLabel, TopBottomPanel, Widget, Window,
};

fn gen_sample_notes(count: i32) -> Vec<String> {
    (0..count)
        .map(|number| format!("Note {}", number))
        .collect()
}

fn gen_sample_text(lines_count: i32) -> String {
    (0..lines_count)
        .map(|number| format!("Sample text line. The line number is {}\n", number))
        .collect()
}

pub struct NotesApp {
    pub state: AppState,
    pub ui_state: UiState,
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

/// Create demo instance
impl NotesApp {
    pub fn demo() -> Self {
        let notes: Vec<Note> = gen_sample_notes(10)
            .iter()
            .map(|name| Note::with_name(name.to_owned()))
            .collect();
        let notes = NoteFolderTree::with_items(notes);

        let mut note_with_text = Note::scratch_pad();
        let current_note_id = note_with_text.id;
        note_with_text.content.text = gen_sample_text(100);

        Self {
            state: AppState {
                current_note_id,
                scratch_pad: note_with_text,
                notes,
                trash: Default::default(),
            },
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
                    if ui.input(|i| i.modifiers.ctrl) || self.ui_state.egui_settings {
                        if SelectableLabel::new(self.ui_state.egui_settings, phosphor::WRENCH)
                            .ui(ui)
                            .on_hover_text("Egui Tweaks")
                            .clicked()
                        {
                            self.ui_state.egui_settings = !self.ui_state.egui_settings;
                        }
                    }
                    if SelectableLabel::new(self.ui_state.explorer, phosphor::LIST_DASHES)
                        .ui(ui)
                        .on_hover_text(t!("explorer"))
                        .clicked()
                    {
                        self.ui_state.explorer = !self.ui_state.explorer
                    }
                    if SelectableLabel::new(self.ui_state.trash, phosphor::TRASH)
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
                    ()
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
                    ()
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
            let mut note_id_to_remove = None;

            ScrollArea::vertical()
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    let note = &self.state.scratch_pad;
                    let selected = self.state.current_note_id == note.id;
                    if ui.add(note_label(selected, note)).clicked() {
                        self.state.current_note_id = note.id;
                    }

                    ui.separator();
                    if ui
                        .button(format!("{} {}", phosphor::PLUS, t!("new_note")))
                        .clicked()
                    {
                        self.state.new_note_then_switch();
                    }
                    if ui
                        .button(format!("{} {}", phosphor::PLUS, t!("new_folder")))
                        .clicked()
                    {
                        self.state.new_folder("New folder".to_owned());
                    }
                    self.state.notes.root_folder().iter().for_each(|note| {
                        let mut selected = self.state.current_note_id == note.id;
                        let mut remove = false;
                        explorer_note_label_ui(ui, &mut selected, &mut remove, note);
                        if selected {
                            self.state.current_note_id = note.id;
                        }
                        if remove {
                            note_id_to_remove = Some(note.id);
                        }
                    });
                    self.state
                        .notes
                        .get_sub_folders(&self.state.notes.root_folder().id)
                        .unwrap()
                        .for_each(|folder| {
                            // TODO: collapsible heading label
                            ui.horizontal(|ui| {
                                CollapsingHeader::new(&folder.name)
                                    .id_salt(&folder.id)
                                    .show(ui, |_ui| {});
                            });
                        });
                });

            if let Some(note_id) = note_id_to_remove {
                self.state.trash_note(note_id);
            }
        });
    }

    fn status_bar_ui(&self, ui: &mut Ui) {
        let layout = Layout::left_to_right(Align::TOP).with_main_align(Align::LEFT);
        ui.with_layout(layout, |ui| match self.state.lookup_current_note() {
            NoteLookup::Default(note) | NoteLookup::ScratchPad(note) => {
                let text = format!("{} {}", &note.icon(), &note.content.name);
                Label::new(RichText::new(text)).selectable(false).ui(ui);
            }
            NoteLookup::Trashed(Trashed { item, .. }) => {
                let text = format!(
                    "{} {} / {} {}",
                    phosphor::TRASH,
                    t!("trash"),
                    &item.icon(),
                    &item.content.name
                );
                Label::new(RichText::new(text).color(Color32::RED))
                    .selectable(false)
                    .ui(ui);
            }
        });
    }

    fn title_ui(&mut self, ui: &mut Ui) {
        let layout = Layout::top_down_justified(Align::LEFT);
        ui.with_layout(layout, |ui| {
            let note = self.state.current_note();

            let mod_date = format_date_time(&note.modification_time);
            ui.weak(format!("{} {}", t!("modified"), mod_date));

            ui.add_space(item_spacing(ui.ctx(), &layout));

            let icon_label = Label::new(RichText::new(note.icon()).heading());
            let title_text = match self.state.lookup_current_note_content() {
                ContentLookup::Mut(content) => &mut content.name,
                ContentLookup::Immut(content) => &mut content.name.clone(),
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
            if let NoteLookup::Trashed(trashed) = self.state.lookup_current_note() {
                let trash_put_time = format_date_time(&trashed.put_time);
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
                    self.state.restore_note(trashed.item.id);
                }
            }

            let mutable = self.state.lookup_current_note_content().is_mut();
            ui.add_enabled_ui(mutable, |ui| {
                self.title_ui(ui);
                ui.separator();

                let note_text = match self.state.lookup_current_note_content() {
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
            .show(ctx, |ui| {
                let mut note_id_to_restore = None;

                self.state.trash.values().into_iter().for_each(|trashed| {
                    let mut selected = self.state.current_note_id == trashed.item.id;
                    let mut restore = false;
                    trash_label_ui(ui, &mut selected, &mut restore, trashed);
                    if selected {
                        self.state.current_note_id = trashed.item.id;
                    }
                    if restore {
                        note_id_to_restore = Some(trashed.item.id);
                    }
                });
                if let Some(note_id) = note_id_to_restore {
                    self.state.restore_note(note_id);
                }
            });
    }
}

fn date_time_fmt() -> &'static str {
    static DATE_TIME_FMT: LazyLock<String> =
        std::sync::LazyLock::new(|| format!("%d.%m.%Y {} %H:%M", t!("at")));
    &*DATE_TIME_FMT
}

fn format_date_time(date: &DateTime<Utc>) -> String {
    to_local_date_time(date)
        .format(&date_time_fmt())
        .to_string()
}

fn trash_label_ui(
    ui: &mut Ui,
    selected: &mut bool,
    restore: &mut bool,
    trashed: &Trashed<Note>,
) -> egui::Response {
    let label = ui.add(note_label(*selected, &trashed.item));
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
            ui.close_menu();
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
    note: &Note,
) -> egui::Response {
    let label = ui.add(note_label(*selected, note));
    label.context_menu(|ui| {
        if ui
            .button(format!("{} {}", phosphor::TRASH, t!("trash_note")))
            .clicked()
        {
            *remove = true;
            ui.close_menu();
        }
    });
    if label.clicked() {
        *selected = true
    }
    label
}

fn note_label(selected: bool, note: &Note) -> SelectableLabel {
    let mut label_text = RichText::new(format!("{} {}", note.icon(), &note.content.name));

    if selected {
        label_text = label_text.strong();
    }

    SelectableLabel::new(selected, label_text)
}
