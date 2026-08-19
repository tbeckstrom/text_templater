mod app;
mod drafts;
mod field;
mod formatting;
mod history;
mod render;
mod storage;
mod template;

use app::NoteTemplaterApp;

fn main() -> eframe::Result {
    let paths = storage::resolve_paths().expect("failed to resolve app data directory");
    let app = NoteTemplaterApp::new(paths);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Note Templater",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
