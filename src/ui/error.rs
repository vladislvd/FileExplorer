use eframe::egui;
use crate::app::FileExplorer;

pub fn show_error_window(
    app: &mut FileExplorer,
    ctx: &egui::Context,
){
    egui::Window::new("Error")
        .fixed_size([105.0,25.0])
        .show(ctx, |ui|{
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.label(&app.text_err);
                if ui.add_sized([50.0, 25.0], egui::Button::new("Ok")).clicked(){
                    app.show_err = false;
                }
            });
        });
}