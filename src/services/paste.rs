use std::fs;
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

        if target_path == clipboard.source_path{
            app.text_err = String::from("File already in this directory");
            app.show_err = true;
        }

        let mut successfully_paste = false;
        match clipboard.operation {
            ClipboardOperation::Copy => {
                if target_path.is_dir(){
                    match fs_extra::dir::copy(clipboard.source_path, &target_path, &CopyOptions::new()) {
                        Ok(_) => successfully_paste = true,
                        Err(e) => {
                            app.text_err = format!("Failed to copy directory, {}", e);
                            app.show_err = true;
                        }
                    }
                } else {
                    match fs::copy(clipboard.source_path, &target_path) {
                        Ok(_) => successfully_paste = true,
                        Err(e) => {
                            app.text_err = format!("Failed to copy file, {}", e);
                            app.show_err = true;
                        }
                    }
                }
            },
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