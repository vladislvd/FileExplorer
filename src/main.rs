use eframe::egui;
use walkdir::WalkDir;
use core::f32;
use std::{path::PathBuf, sync::mpsc::{Receiver, channel}, thread, time::SystemTime};
use regex;


#[derive(Default, PartialEq)]
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
    search_result: Vec<PathBuf>,
    search_rx: Option<Receiver<PathBuf>>,
    zoom_factor: f32,
    cached_files: Vec<FileInfo>,
    show_err: bool,
    text_err: String,
    sort_by: SortBy,
    sort_ascending: bool, // true = А-Я, false = Я-А
    show_hidden: bool,
    search_hidden: bool,
    search_venv: bool,
    search_whole_word: bool,
    match_case: bool,
}

struct FileInfo {
    path: PathBuf,
    filename: String,
    is_dir: bool,
    created_at: SystemTime,
}

impl FileExplorer{
    fn new(_cc: &eframe::CreationContext<'_>) -> Self{
        let mut app =Self {  
            current_path: dirs::download_dir().unwrap_or_else(|| {std::env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"))}),
            path_history: Vec::new(),
            search_query: String::new(),
            search_result: Vec::new(),
            search_rx: None,
            zoom_factor: 1.5,
            cached_files: Vec::new(),
            show_err: false,
            text_err: String::new(),
            sort_by: SortBy::default(),
            sort_ascending: true,
            show_hidden: false,
            search_hidden: false,
            search_venv: false,
            search_whole_word: true,
            match_case: true, //=учитывать регистр
        };
        app.refresh_cache();

        app
    }

    fn search(&mut self, ctx: &egui::Context) {
        if self.search_query.trim().is_empty() { return; }
        let (tx, rx) = channel();
        let query = self.search_query.clone();
        let root_to_scan = self.current_path.clone(); //PathBuf::from("C://"); // dirs::home_dir().unwrap_or_else(|| self.current_path.clone()); - должна быть опция
        self.search_result.clear();
        self.search_rx = Some(rx);
        let search_hidden = self.search_hidden;
        let search_venv = self.search_venv;
        let search_whole_word = self.search_whole_word;
        let match_case = self.match_case;
        let ctx_clone = ctx.clone();
        thread::spawn(move || {
            let walker = WalkDir::new(&root_to_scan)
                .into_iter()
                .filter_entry(move |e| {
                    if e.path() == root_to_scan { return true; }
                    let name = e.file_name().to_string_lossy();
                    let is_hidden = !search_hidden && name.starts_with(".") && hf::is_hidden(e.path()).unwrap_or(false);
                    let is_venv = !search_venv && name=="venv";
                    !(is_hidden || is_venv)
                });
            let escaped_query = regex::escape(&query);
            let pattern = if search_whole_word{
                format!(r"\b{}\b", escaped_query)
            } else {
                escaped_query
            };
            let re = regex::RegexBuilder::new(&pattern)
                .case_insensitive(!match_case)
                .build()
                .unwrap();
            for file in walker.filter_map(|e| e.ok()){
                let name = file.file_name().to_string_lossy();
                if re.is_match(&name){
                    if tx.send(file.path().to_path_buf()).is_err(){
                        break;
                    } else {
                        ctx_clone.request_repaint();
                    }                
                }
            }
        });
    }

    fn refresh_cache(&mut self){
        // println!("Обновление кэша для пути: {:?}", self.current_path);
        match std::fs::read_dir(&self.current_path) {
            Ok(files) => {
                let mut new_files: Vec<FileInfo> = files.
                filter_map(|e| {
                    let file = match e {
                        Ok(val ) => val,
                        Err(err) => {
                            self.text_err = String::from(format!("File read error: {}", err));
                            self.show_err = true;
                            return None;
                        }
                    };
                    let path = file.path();
                    if !self.show_hidden && hf::is_hidden(&path).unwrap_or(false){
                        return None;
                    }
                    let meta = file.metadata().ok();
                    Some(FileInfo {
                        filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Unknown".to_string()),
                        is_dir: path.is_dir(),
                        path: path,
                        created_at: meta.and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::now()),
                    })
                })
                .collect();
                new_files.sort_by(|a, b| {
                    let result = match self.sort_by {
                        SortBy::Name => a.filename.to_lowercase().cmp(&b.filename.to_lowercase()),
                        SortBy::Date => a.created_at.cmp(&b.created_at),
                        SortBy::Type => {
                            if a.is_dir != b.is_dir { 
                                b.is_dir.cmp(&a.is_dir)
                            } else {
                                a.filename.to_lowercase().cmp(&b.filename.to_lowercase())
                            }
                        },
                    };
                    if self.sort_ascending {
                        result 
                    } else {
                        result.reverse() 
                    }
                });
                self.cached_files = new_files;
            }
            Err(err) => {
                self.text_err = String::from(format!("Directory read error: {}", err));
                self.show_err = true;
            }
        }
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

fn draw_item(ui: &mut egui::Ui, path: &PathBuf, app: &mut FileExplorer, _ctx: &egui::Context) {
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let is_dir = path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };
    ui.scope(|ui|{
        ui.style_mut().spacing.button_padding *= app.zoom_factor;
        if ui.selectable_label(false, format!("{} {}", icon, filename)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            let cur_path = app.current_path.clone();
            if path.exists(){
                if is_dir {
                    app.path_history.push(cur_path);
                    app.current_path = path.clone();
                    app.search_query.clear();
                    app.search_rx = None;
                    app.refresh_cache();
                } else {
                    let _ = opener::open(path); 
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                }
            } else {
                app.text_err = String::from("Файл не найден.");
                app.show_err = true;
            }
        }
    });
}


impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.zoom_factor);
        if let Some(ref rx) = self.search_rx{
            while let Ok(path) = rx.try_recv() {
                self.search_result.push(path);
                ctx.request_repaint();
            }
        }
        if self.show_err{
            egui::Window::new("Error")
            .fixed_size([105.0,25.0])
            .show(ctx, |ui|{
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                    ui.label(&self.text_err);
                    if ui.add_sized([50.0, 25.0], egui::Button::new("Ok")).clicked(){
                        self.show_err = false;
                        self.refresh_cache();
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
                                self.refresh_cache();
                            }
                        }
                        if ui.button("^").on_hover_text("To the parent directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {  //изменить ширину
                            if let Some(parent) = self.current_path.parent() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = parent.to_path_buf();
                                self.refresh_cache();
                            }
                        }
                        if ui.button("<--").on_hover_text("To the previous directory").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            if let Some(future_path) = self.path_history.pop(){
                                self.current_path = future_path;
                                self.refresh_cache();
                            }
                        }
                        ui.label(format!("Текущий путь: {}", self.current_path.to_string_lossy())); //можно поменять to_string_lossy на display??
                        if ui.button("🔄").on_hover_text("Update this directory(F5)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            self.refresh_cache();
                        }
                });
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.menu_button("⚙ View and sort 🔽", |ui|{
                        ui.set_min_width(150.0);
                        if ui.checkbox(&mut self.show_hidden, "Show hidden").clicked(){
                            self.refresh_cache();
                        }
                        ui.separator();
                        ui.label("Sort by:");
                        if ui.radio_value(&mut self.sort_by, SortBy::Date, "Date").clicked() {
                            self.refresh_cache();
                        }
                        if ui.radio_value(&mut self.sort_by, SortBy::Name, "Name").clicked() {
                            self.refresh_cache();
                        }
                        if ui.radio_value(&mut self.sort_by, SortBy::Type, "Type").clicked() {
                            self.refresh_cache();
                        }
                        ui.separator();
                        if ui.radio_value(&mut self.sort_ascending, true, "⬆ Ascending (A-Z)").clicked(){
                            self.refresh_cache();
                        }
                        if ui.radio_value(&mut self.sort_ascending, false, "⬇ Descending (Z-A)").clicked(){
                            self.refresh_cache();
                        }
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
                        if search_bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)){ //response.lost_focus() - закончил ли пользователь взаимодействовать с search_bar
                            self.search(ctx);
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
                    if !self.search_query.is_empty() && self.search_rx.is_some() {
                        ui.label("Результаты глубокого поиска:");
                        let search_result: Vec<PathBuf> = self.search_result.iter().cloned().collect();
                        for path in search_result {
                            draw_item(ui, &path, self, ctx);
                        }
                    } else {
                        let files_to_draw: Vec<PathBuf> = self.cached_files.iter().filter(|info|{
                            if self.search_query.is_empty() {return true;}
                            info.filename.contains(&self.search_query.to_lowercase())
                        })
                        .map(|info| info.path.clone())
                        .collect();

                        for path in files_to_draw {
                            draw_item(ui, &path, self, ctx);
                        }
                    }
                });
        });
    }
}