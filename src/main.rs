use eframe::egui;
use walkdir::WalkDir;
use std::{path::PathBuf, sync::mpsc::{Receiver, channel}, thread};


#[derive(Default)]
struct FileExplorer {
    current_path: PathBuf,
    path_history: Vec<PathBuf>,
    search_query: String,
    search_result: Vec<PathBuf>,
    search_rx: Option<Receiver<PathBuf>>,
}

impl FileExplorer{
    fn new(_cc: &eframe::CreationContext<'_>) -> Self{
        Self { 
            current_path: dirs::download_dir().unwrap_or_else(|| {std::env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"))}),
            path_history: Vec::new(),
            search_query: String::new(),
            search_result: Vec::new(),
            search_rx: None,
        }
    }

    fn search(&mut self) {
        let (tx, rx) = channel();
        let query = self.search_query.to_lowercase();
        let root_to_scan = self.current_path.clone(); //PathBuf::from("C://"); // dirs::home_dir().unwrap_or_else(|| self.current_path.clone()); - должна быть опция
        self.search_result.clear();
        self.search_rx = Some(rx);
        thread::spawn(move || {
            let walker = WalkDir::new(&root_to_scan)
                .into_iter()
                .filter_entry(move |e| {
                    if e.path() == root_to_scan { return true; }
                    let name = e.file_name().to_string_lossy();
                    let hide = hf::is_hidden(e.path()).unwrap_or(false); //должна быть опция
                    let first_is_dot = name.starts_with("."); //должна быть опция
                    let venv = name == "venv"; //должна быть опция
                    !(hide || first_is_dot || venv)
                });
            for file in walker.filter_map(|e| e.ok()){
                if !hf::is_hidden(file.path()).unwrap_or(false){
                    let name = file.file_name().to_string_lossy().to_lowercase();
                    if name.contains(&query){ //contains - поиск подстроки
                        if tx.send(file.path().to_path_buf()).is_err(){
                            break;
                        }
                    }
                }
                // if let Some(parent) = file.path().parent(){
                //     println!("Идёт поиск....{}", parent.to_string_lossy());
                // }
            }
        });
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
    if ui.selectable_label(false, format!("{} {}", icon, filename)).clicked() {
        let cur_path = app.current_path.clone();
        if is_dir {
            app.path_history.push(cur_path);
            app.current_path = path.clone();
            app.search_query.clear();
            app.search_rx = None
        } else {
            let _ = opener::open(path);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
        }
    }
}


impl eframe::App for FileExplorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref rx) = self.search_rx{
            while let Ok(path) = rx.try_recv() {
                self.search_result.push(path);
            }
        }
        let screen_width = ctx.content_rect().width();
        egui::TopBottomPanel::top("main_top_bar").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui|{
                ui.group(|ui|{ //группа управления путём
                        if ui.button("^").clicked() {  //изменить ширину
                            if let Some(parent) = self.current_path.parent() {
                                self.path_history.push(self.current_path.clone());
                                self.current_path = parent.to_path_buf();
                            }
                        }
                    if ui.button("<--").clicked(){
                        println!("{:?}", self.path_history);
                        if let Some(future_path) = self.path_history.pop(){
                            self.current_path = future_path
                        }
                    }
                    ui.label(format!("Текущий путь: {}", self.current_path.to_string_lossy())); //можно поменять to_string_lossy на display??
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::default()), |ui|{
                    ui.group(|ui|{
                        let search_bar = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Поиск (Enter для глубокого)...")
                            .desired_width(200.0)
                        );
                        if search_bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)){ //response.lost_focus() - закончил ли пользователь взаимодействовать с search_bar
                            self.search();
                        }
                        if !self.search_query.is_empty(){
                            if ui.button("❌").clicked(){
                                self.search_query.clear();
                            }
                        }
                        if self.search_query.is_empty(){
                            if ui.button("🔎").clicked(){
                                self.search_query.clear();
                            }
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
                //ui.columns(n, |columns| { ... });
                ui.allocate_space(ui.available_size());
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_width(screen_width)
                .show(ui, |ui|{
                    if !self.search_query.is_empty() && self.search_rx.is_some() {
                        ui.label("Результаты глубокого поиска:");
                        for path in self.search_result.clone() {
                            draw_item(ui, &path, self, ctx);
                        }
                    } else {
                    if let Ok(files) = std::fs::read_dir(&self.current_path){
                        for file in files.flatten(){ //flatten - убирает лишние циклы, выводы с Result
                            let path = file.path();
                            let filename = path.file_name().unwrap_or_default().to_string_lossy();
                            if !self.search_query.is_empty() && !filename.to_lowercase().contains(&self.search_query.to_lowercase()){ //contains - поиск подстроки
                                continue;
                            }
                            draw_item(ui, &path, self, ctx);
                        }
                    }
                }
            });
        });
    }
}