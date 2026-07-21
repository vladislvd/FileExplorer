use std::fs;
use eframe::egui;
use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use fs_extra;
use fs_extra::dir::CopyOptions;
use smol_str::SmolStr;
use crate::app::FileExplorer;
use crate::models::{ClipboardOperation, FileInfo};
use crate::services::deep_sorting;

pub fn paste_operation(
    app: &mut FileExplorer,
){
    if let Some(clipboard) = app.clipboard.take(){
        let file_name = match clipboard.source_path.file_name() {
            Some(name) => name,
            None => return,
        };

        let target_path = app.current_path.join(file_name);

        //fixme: не работает почему-то проверка
        if target_path == clipboard.source_path{
            app.text_err = String::from("File already in this directory");
            app.show_err = true;
            return;
        }

        let mut successfully_paste = false;
        match clipboard.operation {
            //fixme: fs_extra::dir::copy копирует всё в папке, но в индексе и в visible_files остаются СТАРЫЕ файлы в СТАРОЙ папке и ВИЗУАЛЬНО не переносятся.
            ClipboardOperation::Copy => {
                if clipboard.source_path.is_dir(){
                    match fs_extra::dir::copy(&clipboard.source_path, &app.current_path, &CopyOptions::new()) {
                        Ok(_) => successfully_paste = true,
                        Err(e) => {
                            app.text_err = format!("Failed to copy directory, {}", e);
                            app.show_err = true;
                        }
                    }
                } else {
                    match fs::copy(&clipboard.source_path, &target_path) {
                        Ok(_) => successfully_paste = true,
                        Err(e) => {
                            app.text_err = format!("Failed to copy file, {}", e);
                            app.show_err = true;
                        }
                    }
                }
            },
            //fixme: в индексе и в visible files файлы в папках не обновляются и ВИЗУАЛЬНО не переносятся.
            ClipboardOperation::Cut => {
                match fs::rename(&clipboard.source_path, &target_path) {
                    Ok(_) => {
                        successfully_paste = true;
                        if let Ok(mut lock) = app.static_index.write() {
                            lock.retain(|file| file.path != clipboard.source_path)
                        }
                        app.visible_files.retain(|file| file.path != clipboard.source_path)
                    }
                    Err(e) => {
                        app.text_err = format!("Failed to move file, {}", e);
                        app.show_err = true;
                    }
                }
            }
        }
        if successfully_paste {
            let file_info = build_file_info_from_path(target_path);

            if let Ok(mut lock) = app.static_index.write() {
                lock.push(file_info.clone())
            }

            app.visible_files.push(file_info);
            
            deep_sorting(&mut app.visible_files, app.sort_by, app.sort_ascending)
        }
    }
}

pub fn rename_operation_window(
    ctx: &egui::Context,
    app: &mut FileExplorer,
    old_path: PathBuf,
){
    let mut close_window = false;

    //todo: ui нормальный сделать
    let window_res = egui::Window::new("Rename file")
        .resizable(false)
        .movable(true)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui|{
            ui.text_edit_singleline(&mut app.new_for_rename);

            ui.horizontal(|ui|{
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown).with_main_justify(false), |ui| {
                    if ui.add_sized([80.0, 12.0],egui::Button::new("Apply")).clicked() || enter_pressed {
                        if let Some(parent) = old_path.parent() {
                            let new_path = parent.join(&app.new_for_rename);
                            //fixme: в индексе и в visible files файлы в папках не обновляются и ВИЗУАЛЬНО не переносятся.
                            match fs::rename(&old_path, &new_path) {
                                Ok(_) => {
                                    if let Ok(mut lock) = app.static_index.write() {
                                        if let Some(file) = lock.iter_mut().find(|f| f.path == old_path) {
                                            file.path = new_path.clone();
                                            file.name = SmolStr::new(&app.new_for_rename);
                                        }
                                    }

                                    if let Some(file) = app.visible_files.iter_mut().find(|f| f.path == old_path) {
                                        file.path = new_path;
                                        file.name = SmolStr::new(&app.new_for_rename);
                                    }

                                    deep_sorting(&mut app.visible_files, app.sort_by, app.sort_ascending)
                                }
                                Err(e) => {
                                    app.text_err = format!("Failed to rename object, {}", e);
                                    app.show_err = true;
                                }
                            }
                        }
                        close_window = true;
                    }

                    if ui.add_sized([80.0, 12.0],egui::Button::new("Cancel")).clicked(){
                        close_window = true
                    }
                });
            });
        });
    if let Some(inner_window) = window_res {
        ctx.input(|i|
            if i.pointer.any_pressed() {
                if let Some(pos) = i.pointer.interact_pos() {
                    if !inner_window.response.rect.contains(pos) {
                        close_window = true;
                    }
                }
            }
        )
    }

    if close_window {
        app.source_rename = None;
        app.new_for_rename.clear();
    }
}

fn build_file_info_from_path(path: PathBuf) -> FileInfo{
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let meta = path.metadata().ok();

    let is_dir = path.is_dir();
    let mut is_hidden = name.starts_with(".");

    #[cfg(windows)]
    if !is_hidden {
        is_hidden = meta.clone().map(|m| m.file_attributes() & 0x2 != 0).unwrap_or(false);
    }

    FileInfo {
        is_hidden,
        is_venv: name == "venv",
        name: SmolStr::new( & name),
        is_dir,
        created_at: meta.and_then( | m| m.created().ok()).unwrap_or(std::time::SystemTime::now()),
        path
    }
}