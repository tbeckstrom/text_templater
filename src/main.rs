// Release builds on Windows open as a plain GUI app, without a console window behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use text_templater::{app::NoteTemplaterApp, storage};

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
