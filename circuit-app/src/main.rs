mod app;
mod draw;
mod examples;
mod si;

use eframe::egui::ViewportBuilder;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Circuit Engine"),
        ..Default::default()
    };
    eframe::run_native(
        "circuit-engine",
        options,
        Box::new(|cc| Ok(Box::new(app::CircuitApp::new(cc)))),
    )
}
