use std::sync::atomic::Ordering;
use eframe::egui;
use std::fs;
use eframe::egui::Sense;
use crate::app::FileExplorer;
use crate::services::draw_item;
use crate::models::{FileAction, AppClipboard, ClipboardOperation};

pub fn draw_central_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.visible_dirty {
            app.rebuild_visible();
        }
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
                        let (rect1, _) = ui.allocate_exact_size(egui::vec2(300.0, row_height), Sense::hover());
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect1), |ui|{
                            if let Some(p) = draw_item(ui, &file.path, &mut app.search_query ,&mut app.visible_dirty, app.zoom_factor) {
                                pending_action = Some(p);
                            }
                        });

                        let (rect2, _) = ui.allocate_exact_size(egui::vec2(300.0, row_height), Sense::hover());
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
        if let Some(action) = pending_action {
            match action {
                FileAction::Open(path) => {
                    if path.exists(){
                        if path.is_dir() {
                            app.path_history.push(app.current_path.clone());
                            app.current_path = path;
                            app.visible_dirty = true;
                        } else {
                            let _ = opener::open(path);
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                        }
                    } else {
                        app.text_err = String::from("File not found.");
                        app.show_err = true;
                    }
                }
                FileAction::Copy(path) => {
                    app.clipboard = Some(AppClipboard{
                        source_path: path,
                        operation: ClipboardOperation::Copy,
                    })
                },
                FileAction::Cut(path) => {
                    app.clipboard = Some(AppClipboard {
                        source_path: path,
                        operation: ClipboardOperation::Cut
                    })
                },
                //TODO: добавить переименование.
                FileAction::Delete(path) => {
                    let mut successfully_deleted = false;
                    if path.is_dir(){
                        match fs::remove_dir_all(&path) {
                            Ok(_) => successfully_deleted = true,
                            Err(e) => {
                                app.text_err = format!("Couldn`t delete dir {}", e);
                                app.show_err = true;
                            }
                        }
                    } else {
                        match fs::remove_file(&path) {
                            Ok(_) => successfully_deleted = true,
                            Err(e) => {
                            app.text_err = format!("Couldn`t delete file {}", e);
                            app.show_err = true;
                            }
                        }
                    }
                    if successfully_deleted {
                        if let Ok(mut lock) = app.static_index.write() {
                            lock.retain(|file| path != file.path)
                        }
                        app.visible_files.retain(|file| path != file.path)
                    }
                }
            }
        }
    });
}