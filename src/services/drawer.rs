use std::path::PathBuf;
use eframe::egui;
use infer;
use crate::models::FileAction;

pub fn draw_item(
    ui: &mut egui::Ui, 
    path: &PathBuf,
    search_query: &mut String,
    visible_dirty: &mut bool,
    zoom_factor: f32,
    is_cut: bool,
) -> Option<FileAction>{
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let is_dir = path.is_dir();
    let is_pic = infer::get_from_path(path)
            .map(|opt| opt.map_or(false, |info| info.mime_type().starts_with("image")))
            .unwrap_or(false);
    let icon = if is_dir { "📁" } else if is_pic { "🖼" } else { "📄" };
    let mut clicked_action = None;
    ui.scope(|ui|{
        ui.style_mut().spacing.button_padding *= zoom_factor;

        let clickable_file = ui.add_enabled_ui(!is_cut, |ui| {
            ui.selectable_label(false, format!("{} {}", icon, filename)).on_hover_cursor(egui::CursorIcon::PointingHand)
        }).inner;

        if clickable_file.clicked() {
            if !search_query.is_empty(){
                search_query.clear();
            }
            clicked_action = Some(FileAction::Open(path.to_path_buf()));
            *visible_dirty = true;
        }

        clickable_file.clone().context_menu(|ui| {
            ui.spacing_mut().interact_size = egui::vec2(100.0, 26.0);
            ui.set_min_width(80.0);

            if ui.button("Copy").clicked(){
                clicked_action = Some(FileAction::Copy(path.to_path_buf()));
                ui.close()
            }
            if ui.button("Cut").clicked(){
                clicked_action = Some(FileAction::Cut(path.to_path_buf()));
                ui.close()
            }
            if ui.button("Rename").clicked(){
                clicked_action = Some(FileAction::Rename(path.to_path_buf()));
                ui.close();
            }
            ui.separator();
            if ui.button("Delete").clicked(){
                clicked_action = Some(FileAction::Delete(path.to_path_buf()));
                ui.close()
            }
        });
    });
    clicked_action
}