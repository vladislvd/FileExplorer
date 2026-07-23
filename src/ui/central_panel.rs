use std::sync::atomic::Ordering;
use eframe::egui;
use crate::app::FileExplorer;
use crate::services::{draw_item, rename_operation_window};
use crate::models::{FileAction, ClipboardOperation};

//todo: при нажатии по свободной части меню должна быть(новая папка, новый файл, вставить)
pub fn draw_central_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.visible_dirty {
            app.rebuild_visible();
        }

        if let Some(action) = draw_files(ui, app) {
            app.handle_file_action(action, ui);
        }

        if let Some(old_path) = app.source_rename.clone() {
            rename_operation_window(&ctx, app, old_path);
        }
    });
}

fn draw_files(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
) -> Option<FileAction> {
    let row_height = 20.0 * app.zoom_factor;
    let mut pending_action = None;

    egui::ScrollArea::vertical().show_rows(ui, row_height, app.visible_files.len(), |ui, rows|{
        if app.visible_files.is_empty() && !app.is_indexing.load(Ordering::Relaxed){
            ui.label("Nothing was found");
        } else {
            for i in rows{
                let file = &app.visible_files[i];
                ui.horizontal(|ui|{
                    ui.set_min_height(row_height);
                    let (rect1, _) = ui.allocate_exact_size(egui::vec2(300.0, row_height), egui::Sense::hover());
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect1), |ui|{
                        let mut is_cut = false;
                        if let Some(clipboard) = &app.clipboard {
                            is_cut = file.path == clipboard.source_path && clipboard.operation == ClipboardOperation::Cut;
                        }
                        if let Some(p) = draw_item(ui, &file.path, &mut app.search_query ,&mut app.visible_dirty, app.zoom_factor, is_cut) {
                            pending_action = Some(p);
                        }
                    });

                    let (rect2, _) = ui.allocate_exact_size(egui::vec2(300.0, row_height), egui::Sense::hover());
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect2), |ui|{
                        ui.label(file.path.to_string_lossy())
                    });

                    let (rect3, _) = ui.allocate_exact_size(egui::vec2(300.0, row_height), egui::Sense::hover());
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect3), |ui| {
                        let dt: chrono::DateTime<chrono::Local> = file.created_at.into();
                        ui.label(dt.format("%d.%m.%y %H:%M").to_string());
                    });
                });
            }
        }
    });
    pending_action
}
