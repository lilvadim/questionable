mod app;
mod data;
mod egui_app;
mod font_icons;
mod id_gen;
mod note;
mod thread_pool;
mod util;

use eframe::Renderer;
use egui_app::NotesApp;

rust_i18n::i18n!("locales", fallback = "en");

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        // viewport: egui::ViewportBuilder::default()
        //     .with_titlebar_shown(false)
        //     .with_title_shown(false)
        //     .with_fullsize_content_view(true),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Questionable.",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            NotesApp::setup_fonts(&cc.egui_ctx);
            Ok(Box::new(NotesApp::init()))
        }),
    )
}
