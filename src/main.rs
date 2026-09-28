// Release builds on Windows open as a plain GUI app, without a console window behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use text_templater::{app::NoteTemplaterApp, storage};

#[cfg(not(target_arch = "wasm32"))]
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

/// Browser entry point: draws into the `<canvas id="app">` in `index.html`.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("app"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("index.html has no <canvas id=\"app\">");
        let paths = storage::resolve_paths().expect("browser paths are static");

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_cc| Ok(Box::new(NoteTemplaterApp::new(paths)))),
            )
            .await;

        // Replace the loading message with what went wrong, e.g. no WebGL.
        if let Some(status) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("status"))
        {
            match result {
                Ok(()) => status.remove(),
                Err(e) => status.set_text_content(Some(&format!("Could not start: {e:?}"))),
            }
        }
    });
}
