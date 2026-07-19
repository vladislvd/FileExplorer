mod app;
mod models;
mod services;
mod ui;
use eframe::egui;
use app::FileExplorer;


fn main() -> eframe::Result<(), eframe::Error>{
    let options = eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_resizable(true)
            .with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "FileExplorer",
        options,
        Box::new(|cc| Ok(Box::new(FileExplorer::new(cc))))
    )
}