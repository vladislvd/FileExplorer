use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use eframe::egui::Context;
use eframe::Frame;
use sysinfo::Disks;
use crate::models::{DiskInfo, FileInfo, SortBy, AppClipboard};
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
}

impl eframe::App for FileExplorer{
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        check_end_of_indexing(self);
        
        get_new_disks(self);
        
        ctx.set_pixels_per_point(self.zoom_factor);
        
        if self.show_err{
            show_error_window(self, &ctx);
        }

        //todo: переделать структуру функций отрисовки панелей
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