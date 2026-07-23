use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock, 
           atomic::{AtomicBool, Ordering},
           mpsc::Receiver,
    },
    time::Duration,
};
use eframe::{egui, Frame};
use sysinfo::Disks;
use crate::models::{DiskInfo, FileInfo, SortBy, AppClipboard, FileAction, ClipboardOperation};
use crate::services::{start_indexing, sorting, start_disks_monitoring, get_new_disks};
use crate::ui::{draw_central_panel, draw_side_panel, draw_top_panel};
use crate::ui::show_error_window;

pub struct FileExplorer {
    pub current_path: PathBuf,
    pub path_history: Vec<PathBuf>,
    pub current_disk: PathBuf,
    pub static_index: Arc<RwLock<Vec<FileInfo>>>,
    pub is_indexing: Arc<AtomicBool>,
    pub was_indexing: Arc<AtomicBool>,
    cancel_indexing: Arc<AtomicBool>,
    pub index_time: Arc<RwLock<Duration>>,
    pub index_all: bool,
    pub visible_files: Vec<FileInfo>,
    pub visible_dirty: bool,
    pub search_query: String,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
    pub show_hidden: bool,
    pub search_hidden: bool,
    pub search_venv: bool,
    pub search_whole_word: bool,
    pub match_case: bool,
    pub search_everywhere: bool,
    pub zoom_factor: f32,
    pub show_err: bool,
    pub text_err: String,
    pub disk_receiver: Receiver<Disks>,
    pub all_disks: Vec<DiskInfo>,
    pub clipboard: Option<AppClipboard>,
    pub source_rename: Option<PathBuf>,
    pub new_for_rename: String,
}

impl FileExplorer {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let rx = start_disks_monitoring();

        let mut app = Self {
            current_path: PathBuf::from("C:\\"),
            path_history: Vec::new(),
            current_disk: PathBuf::from("C:\\"),
            static_index: Arc::new(RwLock::new(Vec::new())),
            is_indexing: Arc::new(AtomicBool::new(false)),
            was_indexing: Arc::new(AtomicBool::new(false)),
            cancel_indexing: Arc::new(AtomicBool::new(false)),
            index_time: Arc::new(RwLock::new(Duration::ZERO)),
            index_all: false,
            visible_files: Vec::new(),
            visible_dirty: false,
            search_query: String::new(),
            sort_by: SortBy::Date,
            sort_ascending: false,
            show_hidden: false,
            search_hidden: false,
            search_venv: false,
            search_whole_word: false,
            match_case: false,
            search_everywhere: true,
            zoom_factor: 1.5,
            show_err: false,
            text_err: String::new(),
            disk_receiver: rx,
            all_disks: Vec::new(),
            clipboard: None,
            source_rename: None,
            new_for_rename: String::new(),
        };

        app.update_index();
        app
    }

    pub fn update_index(&mut self) {
        self.cancel_indexing.store(true, Ordering::SeqCst);
        self.cancel_indexing = Arc::new(AtomicBool::new(false));

        start_indexing(
            self.current_disk.clone(),
            self.index_all,
            Arc::clone(&self.static_index),
            Arc::clone(&self.is_indexing),
            Arc::clone(&self.cancel_indexing),
            Arc::clone(&self.index_time),
        );
    }

    pub fn rebuild_visible(&mut self){
        sorting(
          &self.current_path,
          self.static_index.read().unwrap(),
          self.search_query.as_str(),
          self.search_hidden,
          self.show_hidden,
          self.search_venv,
          self.search_everywhere,
          self.search_whole_word,
          self.match_case,
          self.sort_ascending,
          &mut self.visible_files,
          &mut self.visible_dirty,
          self.sort_by.clone(),
        );
    }

    pub fn handle_file_action(
        &mut self,
        action: FileAction,
        ui: &mut egui::Ui,
    ){
        match action {
            FileAction::Open(path) => {
                if path.exists(){
                    if path.is_dir() {
                        self.path_history.push(self.current_path.clone());
                        self.current_path = path;
                        self.visible_dirty = true;
                    } else {
                        let _ = opener::open(path);
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                    }
                } else {
                    self.text_err = String::from("File not found.");
                    self.show_err = true;
                }
            }
            FileAction::Copy(path) => {
                self.clipboard = Some(AppClipboard {
                    source_path: path,
                    operation: ClipboardOperation::Copy,
                })
            },
            FileAction::Cut(path) => {
                self.clipboard = Some(AppClipboard {
                    source_path: path,
                    operation: ClipboardOperation::Cut
                })
            },
            FileAction::Rename(path) => {
                if let Some(filename) = path.clone().file_name(){
                    self.source_rename = Some(path);
                    self.new_for_rename = filename.to_string_lossy().into_owned();
                }
            }
            FileAction::Delete(path) => {
                let mut successfully_deleted = false;
                if path.is_dir(){
                    match fs::remove_dir_all(&path) {
                        Ok(_) => successfully_deleted = true,
                        Err(e) => {
                            self.text_err = format!("Couldn`t delete dir {}", e);
                            self.show_err = true;
                        }
                    }
                } else {
                    match fs::remove_file(&path) {
                        Ok(_) => successfully_deleted = true,
                        Err(e) => {
                            self.text_err = format!("Couldn`t delete file {}", e);
                            self.show_err = true;
                        }
                    }
                }
                if successfully_deleted {
                    if let Ok(mut lock) = self.static_index.write() {
                        lock.retain(|file| path != file.path)
                    }
                    self.visible_files.retain(|file| path != file.path)
                }
            }
        }
    }

    pub fn handle_disk_action(
        &mut self,
        selected_disk: Option<PathBuf>,
    ){
        if let Some(path) = selected_disk {
            self.current_disk = path.clone();
            self.current_path = path;
            self.update_index();
            self.visible_dirty = true;
        }
    }
}

impl eframe::App for FileExplorer{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        check_end_of_indexing(self);
        
        get_new_disks(self);
        
        ctx.set_pixels_per_point(self.zoom_factor);
        
        if self.show_err{
            show_error_window(self, &ctx);
        }

        draw_side_panel(self, &ctx);
        draw_top_panel(self, &ctx);
        draw_central_panel(self, &ctx);

        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

fn check_end_of_indexing(app: &mut FileExplorer){
    let current_indexing = app.is_indexing.load(Ordering::Relaxed);
    if app.was_indexing.swap(current_indexing, Ordering::Relaxed) && !current_indexing {
        app.visible_dirty = true;
    }
}