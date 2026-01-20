//! Arbor GUI - Impact-first code analysis interface
//!
//! A minimal, focused GUI for answering: "What breaks if I change this?"

mod app;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Arbor - Impact Analysis"),
        ..Default::default()
    };

    eframe::run_native(
        "Arbor",
        options,
        Box::new(|cc| Ok(Box::new(app::ArborApp::new(cc)))),
    )
}
