use eframe::egui;
use core::f32;
use std::{path::PathBuf, sync::{atomic, mpsc::channel}, thread, time::SystemTime};
use std::sync::{Arc, RwLock, atomic::AtomicBool};
use rayon::prelude::*;


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
}

struct FileInfo {
    path: PathBuf,
    name: String,
    name_lower: String,
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
        };

        app.update_index();
        app
    }

    fn update_index(&self){
        let index_prt = Arc::clone(&self.static_index);
        let is_indexing = Arc::clone(&self.is_indexing);
        let root = self.current_path.clone();

        thread::spawn(move || {
            is_indexing.store(true, atomic::Ordering::SeqCst);
            let (tx, rx) = channel();
            let walker = ignore::WalkBuilder::new(root).threads(num_cpus::get()).build_parallel();
            walker.run(|| {
                let tx = tx.clone();
                Box::new(move |result| {
                    if let Ok(entry) = result{
                        let path = entry.path().to_path_buf();
                        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        let meta = path.metadata().ok();
                        let info = FileInfo{
                            is_hidden: name.starts_with(".") || hf::is_hidden(&path).unwrap_or(false),
                            name_lower: name.to_lowercase(),
                            name: name,
                            is_dir: path.is_dir(),
                            created_at: meta.and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::now()),
                            is_venv: path.components().any(|c| c.as_os_str() == "venv"),
                            path: path
                        };
                        let _ = tx.send(info);
                    }
                    ignore::WalkState::Continue
                })
            });
            drop(tx);
            let data: Vec<_> = rx.into_iter().collect();
            if let Ok(mut lock) = index_prt.write() {
                *lock = data;
            }
            is_indexing.store(false, atomic::Ordering::SeqCst);
        });

    }

    fn files_sorting<'a>(&'a self, index: &'a [FileInfo]) -> Vec<&'a FileInfo> {
        let current_path = &self.current_path;
        let query = if self.match_case { self.search_query.clone() } else { self.search_query.to_lowercase() };
        let search_hidden = self.search_hidden;
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
                let target = if match_case { &file.name } else { &file.name_lower };
                if !search_hidden && file.is_hidden { return false; }
                if !search_venv && file.is_venv { return false; }
                if !search_everywhere && !file.path.starts_with(&current_path) { return false; }
                if search_whole_word {
                    target == &query
                } else {
                    target.contains(&query)
                }
            }).take_any(500)
            .collect()
        };

        filtred_files.par_sort_by(|a, b|{
            let result =match sort_by {
                SortBy::Date => a.created_at.cmp(&b.created_at),
                SortBy::Name => a.name_lower.cmp(&b.name_lower),
                SortBy::Type => {
                    if a.is_dir != b.is_dir {
                        b.is_dir.cmp(&a.is_dir)
                    } else {
                        a.name_lower.cmp(&b.name_lower)
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
            // .with_inner_size([800.0, 300.0])
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
                        // self.refresh_cache();
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
                                self.update_index();
                            }
                        }
                        if ui.button("^").on_hover_text("To the parent directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {  //изменить ширину
                            if let Some(parent) = self.current_path.parent() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = parent.to_path_buf();
                                self.update_index();
                            }
                        }
                        if ui.button("<--").on_hover_text("To the previous directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            if let Some(future_path) = self.path_history.pop(){
                                self.current_path = future_path;
                                self.update_index();
                            }
                        }
                        ui.label(format!("Текущий путь: {}", self.current_path.to_string_lossy())); //можно поменять to_string_lossy на display??
                        if !self.is_indexing.clone().load(atomic::Ordering::Relaxed){
                            if ui.button("🔄").on_hover_text("Update this directory(F5)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                self.update_index();
                            }
                        } else {
                            ui.spinner();
                        }
                });
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.menu_button("⚙ View and sort 🔽", |ui|{
                        ui.set_min_width(150.0);
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
                }); 
                ui.add_space(50.0);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::default()), |ui|{
                    ui.group(|ui|{
                        ui.menu_button("🔽", |ui|{
                            ui.set_min_width(150.0);
                            ui.checkbox(&mut self.search_hidden, "Search hidden");
                            ui.checkbox(&mut self.search_venv, "Search venv");
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
            egui::ScrollArea::vertical()
                .max_width(f32::INFINITY)
                .auto_shrink([false; 2])
                .show(ui, |ui|{
                    ui.set_min_width(ui.available_width());
                    let mut action_path = None;
                    if let Ok(index) = self.static_index.read(){
                        let files = self.files_sorting(&index);
                        if files.is_empty() {
                            if !self.search_query.is_empty() {
                                ui.label("Nothing was found");
                            } else {
                                ui.label("Поиск...");
                            }
                        } else {
                            for file in files{
                                if let Some(p) = draw_item(ui, &file.path, self.zoom_factor) {
                                    action_path = Some(p);
                                }   
                            }
                        }
                    }
                    if let Some(path) = action_path {
                        if path.exists(){
                            if path.is_dir() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = path;
                                self.search_query.clear();
                                self.update_index();
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
        });
    }
}