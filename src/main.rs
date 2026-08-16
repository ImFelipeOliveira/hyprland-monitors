mod adapters;
mod application;
mod domain;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 660.0])
            .with_min_inner_size([720.0, 480.0])
            .with_app_id("hyprland-monitors"),
        ..Default::default()
    };
    eframe::run_native(
        "Hyprland Monitors",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
