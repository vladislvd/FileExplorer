use eframe::egui;
use crate::app::FileExplorer;

pub fn show_error_window(
    app: &mut FileExplorer,
    ctx: &egui::Context,
){
    let window_frame = egui::Frame::window(&ctx.style()).stroke(egui::Stroke::new(2.0, egui::Color32::RED));

    egui::Window::new("Error")
        .fixed_size([120.0,25.0])
        .frame(window_frame)
        .collapsible(false)
        .show(ctx, |ui|{
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.label(egui::RichText::new(&app.text_err).heading());
                if ui.add_sized([50.0, 25.0], egui::Button::new("Ok")).clicked(){
                    app.show_err = false;
                }
            });
        });
}