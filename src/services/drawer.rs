use std::path::PathBuf;
use eframe::egui;

pub fn draw_item(
    ui: &mut egui::Ui, 
    path: &PathBuf,
    visible_dirty: &mut bool,
    zoom_factor: f32
) -> Option<PathBuf>{
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let is_dir = path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };
    let mut clicked_path = None;
    ui.scope(|ui|{
        ui.style_mut().spacing.button_padding *= zoom_factor;
        if ui.selectable_label(false, format!("{} {}", icon, filename)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clicked_path = Some(path.to_path_buf());
            *visible_dirty = true;
        }
    });
    clicked_path
}