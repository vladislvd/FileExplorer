use eframe::egui;
use core::{f32, time};
use std::{path::PathBuf, sync::atomic, thread, time::SystemTime, os::windows::fs::MetadataExt};
use std::sync::{Arc, RwLock, atomic::AtomicBool};
use rayon::prelude::*;
use egui_extras::{TableBuilder, Column};
use chrono;
use smol_str::SmolStr;

#[derive(Default, PartialEq, Clone)]
pub enum SortBy {
    #[default]
    Date,
    Name,
    Type,
}


struct FileExplorer {
    current_path: PathBuf,
    path_history: Vec<PathBuf>,
    search_query: String,
    zoom_factor: f32,
    static_index: Arc<RwLock<Vec<FileInfo>>>,
    is_indexing: Arc<AtomicBool>,
    show_err: bool,
    text_err: String,
    sort_by: SortBy,
    sort_ascending: bool, // true = А-Я, false = Я-А
    show_hidden: bool,
    search_hidden: bool,
    search_venv: bool,
    search_whole_word: bool,
    match_case: bool,
    search_everywhere: bool,
    index_time: Arc<RwLock<std::time::Duration>>,
    index_count: Arc<std::sync::atomic::AtomicUsize>,
    index_all: bool,
}

struct FileInfo {
    path: PathBuf,
    name: SmolStr,
    is_dir: bool,
    created_at: SystemTime,
    is_hidden: bool,
    is_venv: bool,
}

impl FileExplorer{
    fn new(_cc: &eframe::CreationContext<'_>) -> Self{
        let app = Self {  
            current_path: dirs::download_dir().unwrap_or_else(|| {std::env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"))}),
            path_history: Vec::new(),
            search_query: String::new(),
            zoom_factor: 1.5,
            static_index: Arc::new(RwLock::new(Vec::new())),
            is_indexing: Arc::new(AtomicBool::new(false)),
            show_err: false,
            text_err: String::new(),
            sort_by: SortBy::default(),
            sort_ascending: true,
            show_hidden: false,
            search_hidden: false,
            search_venv: false,
            search_whole_word: true,
            match_case: true, //=учитывать регистр
            search_everywhere: true,
            index_time: Arc::new(RwLock::new(time::Duration::new(0, 0))),
            index_count: Arc::new(atomic::AtomicUsize::new(0)),
            index_all: false
        };

        app.update_index();
        app
    }

    fn update_index(&self){
        let index_prt = Arc::clone(&self.static_index);
        let is_indexing = Arc::clone(&self.is_indexing);
        let index_time = Arc::clone(&self.index_time);
        let index_count = Arc::clone(&self.index_count);
        let root = PathBuf::from("C://");
        let index_all = self.index_all.clone();
        // let root = PathBuf::clone(&self.current_path);

        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            is_indexing.store(true, atomic::Ordering::SeqCst);
            let mut builder = ignore::WalkBuilder::new(root);
            builder.hidden(false)
                .follow_links(false)
                .threads(num_cpus::get());
            if !index_all {
                builder.filter_entry(|e|{
                    let path = e.path();
                    if let Some(ext) = path.extension() {
                        if ext == "log" {
                            return false;
                        }
                    }
                    let name = e.file_name();
                    name != "Windows" && 
                    name != "$Recycle.Bin" && 
                    name != "$SysReset" && 
                    name != "hp" &&
                    name != "System.sav" &&
                    name != "AppData" &&
                    name != "Default" && 
                    name != "Recovery"
                });
            }
            let walker = builder.build_parallel();
            let all_files = Arc::new(parking_lot::Mutex::new(Vec::with_capacity(800000)));

            walker.run(|| {
                let shared_ptr = Arc::clone(&all_files);
                let mut buffer = Vec::with_capacity(2048);
                Box::new(move |result| {
                    let buffer = &mut buffer;
                    if let Ok(entry) = result{
                        let file_name_os = entry.file_name();
                        let name = file_name_os.to_string_lossy();
                        let meta = entry.metadata().ok();
                        let mut is_hidden = name.starts_with(".");
                        #[cfg(windows)]
                        if !is_hidden {
                            is_hidden = meta.map(|m| m.file_attributes() & 0x2 != 0).unwrap_or(false);
                        }

                        buffer.push(FileInfo{
                            is_hidden: is_hidden,
                            is_venv: name == "venv",
                            name: SmolStr::new(&name),
                            is_dir: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
                            created_at: entry.metadata().ok().and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::now()),
                            path: entry.into_path(),
                        });
                        if buffer.len() >= 2048 {
                            let mut global_vec = shared_ptr.lock();
                            global_vec.append(buffer);
                        }
                    }
                    ignore::WalkState::Continue
                })
            });
            let data = std::mem::take(&mut *all_files.lock());
            index_count.store(data.len(), atomic::Ordering::SeqCst);
            if let Ok(mut lock) = index_prt.write() {
                *lock = data;
            }
            if let Ok(mut t_lock) = index_time.write() {
                *t_lock = start_time.elapsed();
            }
            is_indexing.store(false, atomic::Ordering::SeqCst);
        });

    }

    fn files_sorting<'a>(&'a self, index: &'a [FileInfo]) -> Vec<&'a FileInfo> {
        let current_path = &self.current_path;
        let search_hidden = self.search_hidden;
        let query = self.search_query.as_str();
        let search_venv = self.search_venv;
        let search_everywhere = self.search_everywhere;
        let search_whole_word = self.search_whole_word;
        let match_case = self.match_case;
        let show_hidden = self.show_hidden;
        let sort_ascending = self.sort_ascending;
        let sort_by = SortBy::clone(&self.sort_by);

        let mut filtred_files: Vec<&'a FileInfo> = if query.is_empty(){
            index.par_iter().filter(|file|{
                if !show_hidden && file.is_hidden { return false; }
                file.path.parent().map_or(false, |p| p == current_path)
            }).collect()
        } else {
            index.par_iter().filter(|file|{
                if !search_hidden && file.is_hidden { return false; }
                if !search_venv && file.is_venv { return false; }
                if !search_everywhere && !file.path.starts_with(&current_path) { return false; }
            if match_case {
                if search_whole_word {
                    file.name == query
                } else {
                    file.name.contains(query)
                }
            } else {
                if search_whole_word {
                    file.name.eq_ignore_ascii_case(query)
                } else {
                    if query.is_empty() { return true; }
                    file.name.as_bytes()
                        .windows(query.len())
                        .any(|window| {
                            window.eq_ignore_ascii_case(query.as_bytes())
                        })
                }
            }
            }).take_any(500)
            .collect()
        };

        filtred_files.par_sort_by(|a, b|{
            let a_low_name = a.name.to_lowercase();
            let b_low_name = b.name.to_lowercase();
            let result =match sort_by {
                SortBy::Date => a.created_at.cmp(&b.created_at),
                SortBy::Name => a_low_name.cmp(&b_low_name),
                SortBy::Type => {
                    if a.is_dir != b.is_dir {
                        b.is_dir.cmp(&a.is_dir)
                    } else {
                        a_low_name.cmp(&b_low_name)
                    }
                }
            };
            if sort_ascending { result } else { result.reverse() }
        });
        filtred_files
    }
}



fn main() -> eframe::Result<(), eframe::Error>{
    let options = eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_resizable(true)
            .with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "FileExplorer", 
        options, 
        Box::new(|cc| Ok(Box::new(FileExplorer::new(cc))))
    )
}

fn draw_item(ui: &mut egui::Ui, path: &PathBuf, zoom_factor: f32) -> Option<PathBuf>{
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let is_dir = path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };
    let mut clicked_path = None;
    ui.scope(|ui|{
        ui.style_mut().spacing.button_padding *= zoom_factor;
        if ui.selectable_label(false, format!("{} {}", icon, filename)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clicked_path = Some(path.to_path_buf());
        }
    });
    clicked_path
}


impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.zoom_factor);
        if self.show_err{
            egui::Window::new("Error")
            .fixed_size([105.0,25.0])
            .show(ctx, |ui|{
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    ui.label(&self.text_err);
                    if ui.add_sized([50.0, 25.0], egui::Button::new("Ok")).clicked(){
                        self.show_err = false;
                    }
                });
            });
        }
        let screen_width = ctx.content_rect().width();
        egui::TopBottomPanel::top("main_top_bar").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui|{
                ui.with_layout(egui::Layout::left_to_right(egui::Align::default()), |ui|{
                    ui.group(|ui|{ //группа управления путём
                        if ui.button("🏠").on_hover_text("To the home directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            if let Some(home_dir) = dirs::home_dir(){
                                self.path_history.push(self.current_path.clone());
                                self.current_path = home_dir;
                            }
                        }
                        if ui.button("^").on_hover_text("To the parent directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {  //изменить ширину
                            if let Some(parent) = self.current_path.parent() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = parent.to_path_buf();
                            }
                        }
                        if ui.button("<--").on_hover_text("To the previous directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            if let Some(future_path) = self.path_history.pop(){
                                self.current_path = future_path;
                            }
                        }
                        ui.label(format!("Текущий путь: {}", self.current_path.to_string_lossy())); //можно поменять to_string_lossy на display??
                        if !self.is_indexing.clone().load(atomic::Ordering::Relaxed){
                            if ui.button("🔄").on_hover_text("Update cache(F5)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                self.update_index();
                            }
                            let count = self.index_count.load(atomic::Ordering::Relaxed);
                            let time = if let Ok(t) = self.index_time.read() { *t } else { std::time::Duration::ZERO };

                            ui.label(format!(
                                "Найдено объектов: {} | Время индексации: {:.2?}", 
                                count, time
                            ));
                        } else {
                            ui.spinner();
                        }
                });
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.menu_button("⚙ View and sort 🔽", |ui|{
                        ui.set_width(150.0);
                        ui.horizontal(|ui|{
                            ui.vertical(|ui|{
                                ui.checkbox(&mut self.show_hidden, "Show hidden");
                                ui.separator();
                                ui.label("Sort by:");
                                ui.radio_value(&mut self.sort_by, SortBy::Date, "Date");
                                ui.radio_value(&mut self.sort_by, SortBy::Name, "Name");
                                ui.radio_value(&mut self.sort_by, SortBy::Type, "Type");
                                ui.separator();
                                ui.radio_value(&mut self.sort_ascending, true, "⬆ Ascending (A-Z)");
                                ui.radio_value(&mut self.sort_ascending, false, "⬇ Descending (Z-A)");
                            });
                            ui.separator();
                            if ui.checkbox(&mut self.index_all, "Indexing all").clicked(){
                                self.update_index();
                            }
                        });
                    });
                }); 
                ui.add_space(50.0);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::default()), |ui|{
                    ui.group(|ui|{
                        ui.menu_button("🔽", |ui|{
                            ui.set_min_width(150.0);
                            ui.checkbox(&mut self.search_hidden, "Search hidden");
                            ui.checkbox(&mut self.search_venv, "Search venv");
                            ui.checkbox(&mut self.search_everywhere, "Seacrh anywhere");
                            ui.checkbox(&mut self.search_whole_word, "Search the whole world");
                            ui.checkbox(&mut self.match_case, "Keep the case");
                        });
                        let search_bar = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Поиск (Enter для глубокого)...")
                            .desired_width(200.0)
                        );
                        if search_bar.hovered(){
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                        }
                        if !self.search_query.is_empty(){
                            if ui.button("❌").on_hover_text("Clear the search bar").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                self.search_query.clear();
                            }
                        }
                        if self.search_query.is_empty(){
                            ui.add_enabled(false, egui::Button::new("🔎"));
                        }
                    });
                });
            });
            ui.add_space(10.0);
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(100.0)
            .width_range(50.0..=screen_width - 50.0)
            .show(ctx, |ui| {
                ui.heading("Меню");
                ui.allocate_space(ui.available_size());
            });

        egui::CentralPanel::default().show(ctx, |ui| {
                    let mut action_path = None;
                    if let Ok(index) = self.static_index.read(){
                        let files = self.files_sorting(&index);
                        if files.is_empty() {
                            if !self.search_query.is_empty() {
                                ui.label("Nothing was found");
                            }
                        } else {
                            TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .column(Column::remainder().at_least(100.0))
                            .column(Column::remainder().at_least(100.0))
                            .column(Column::remainder().at_least(100.0))
                            .header(25.0, |mut header|{
                                header.col(|ui| { ui.strong("Name"); });
                                header.col(|ui| { ui.strong("Path"); });
                                header.col(|ui| { ui.strong("Created at"); });
                            })
                            .body(|body| {
                                body.rows(20.0 *self.zoom_factor, files.len(), |mut row| {
                                    let file = &files[row.index()];
                                    row.col(|ui|{
                                        if let Some(p) = draw_item(ui, &file.path, self.zoom_factor) {
                                            action_path = Some(p);
                                        }
                                    });
                                    row.col(|ui|{
                                        ui.add(egui::Label::new(file.path.to_string_lossy()).truncate());
                                    });
                                    row.col(|ui|{
                                        let dt: chrono::DateTime<chrono::Local> = file.created_at.into();
                                        ui.label(dt.format("%d.%m.%y %H:%M").to_string());
                                    });
                                });
                            });
                        }
                    }
                    if let Some(path) = action_path {
                        if path.exists(){
                            if path.is_dir() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = path;
                            } else {
                                let _ = opener::open(path); 
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                            }
                        } else {
                            self.text_err = String::from("Файл не найден.");
                            self.show_err = true;
                        }
                    }
                });
    }
}