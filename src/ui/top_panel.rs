use std::sync::atomic;
use eframe::egui;
use crate::app::FileExplorer;
use crate::models::SortBy;
use crate::services::paste_operation;
use crate::ui::extensions::{PointHandTrait, TextHandTrait};


pub fn draw_top_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    egui::TopBottomPanel::top("main_top_bar").show(ctx, |ui| {
        ui.add_space(10.0);

        ui.horizontal(|ui|{
            ui.group(|ui|{
                home_button(ui, app);
                parent_dir_button(ui, app);
                previous_button(ui, app);
                ui.label(format!("{}", app.current_path.to_string_lossy()));
                update_index_indicator_button(ui, app);
            });

            ui.add_space(10.0);

            sort_and_view_settings(ui, app);
            clipboard_buttons(ui, app);

            ui.add_space(50.0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::default()), |ui|{
                ui.group(|ui|{
                    search_settings(ui, app);
                    search_bar(ui, app);
                });
            });
        });

        ui.add_space(10.0);
    });
}

fn home_button(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,

){
    let alt_down_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowDown)));
    if ui.button("🏠").on_hover_text("To the home directory (ALT + ArrowDown").hand_cursor().clicked() || alt_down_arrow_pressed {
        if let Some(home_dir) = dirs::home_dir(){
            app.path_history.push(app.current_path.clone());
            app.current_path = home_dir;
            app.visible_dirty = true;
        }
    }
}

fn parent_dir_button(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    let alt_up_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowUp)));
    if ui.button("^").on_hover_text("To the parent directory (ALT + ArrowUp").hand_cursor().clicked() || alt_up_arrow_pressed {
        if let Some(parent) = app.current_path.parent() {
            app.path_history.push(app.current_path.clone());
            app.current_path = parent.to_path_buf();
            app.visible_dirty = true;
        }
    }
}

fn previous_button(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    let alt_left_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowLeft)));
    if ui.button("<--").on_hover_text("To the previous directory (ALT + ArrowLeft)").hand_cursor().clicked() || alt_left_arrow_pressed {
        if let Some(future_path) = app.path_history.pop(){
            app.current_path = future_path;
            app.visible_dirty = true;
        }
    }
}


fn update_index_indicator_button(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    if !app.is_indexing.clone().load(atomic::Ordering::Relaxed){
        let f5_pressed = ui.input(|i| i.key_pressed(egui::Key::F5));

        if ui.button("🔄").on_hover_text("Update cache(F5)").hand_cursor().clicked() || f5_pressed {
            app.update_index();
        }
        let time = if let Ok(t) = app.index_time.read() { *t } else { std::time::Duration::ZERO };

        ui.label(format!("{:.2?}", time));
    } else {
        ui.spinner();
    }
}

fn sort_and_view_settings(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    ui.menu_button("⚙ View and sort 🔽", |ui|{
        ui.set_width(150.0);

        if ui.checkbox(&mut app.show_hidden, "Show hidden").hand_cursor().clicked(){
            app.visible_dirty = true;
        }

        ui.separator();

        ui.label("Sort by:");
        if ui.radio_value(&mut app.sort_by, SortBy::Date, "Date").hand_cursor().clicked()
            || ui.radio_value(&mut app.sort_by, SortBy::Name, "Name").hand_cursor().clicked()
            || ui.radio_value(&mut app.sort_by, SortBy::Type, "Type").hand_cursor().clicked()
        {
            app.visible_dirty = true;
        }

        ui.separator();

        if ui.radio_value(&mut app.sort_ascending, true, "⬆ Ascending (A-Z)").hand_cursor().clicked()
            || ui.radio_value(&mut app.sort_ascending, false, "⬇ Descending (Z-A)").hand_cursor().clicked()
        {
            app.visible_dirty = true;
        }

        ui.separator();

        if ui.checkbox(&mut app.index_all, "Indexing all").hand_cursor().clicked(){
            app.update_index();
        }
    }).response.hand_cursor();
}

fn clipboard_buttons(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    let is_clipboard_empty =  app.clipboard.is_some();

    ui.add_enabled_ui(!is_clipboard_empty, |ui| {
        if ui.button("📋").on_hover_text("Paste into this directory").hand_cursor().clicked() {
            paste_operation(app);
        }
        if ui.button("🧹").on_hover_text("Clear the clipboard").hand_cursor().clicked() {
            app.clipboard = None
        }
    });
}

fn search_settings(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    ui.menu_button("🔽", |ui|{
        ui.set_min_width(150.0);

        if ui.checkbox(&mut app.search_hidden, "Search hidden").hand_cursor().clicked()
            || ui.checkbox(&mut app.search_venv, "Search venv").hand_cursor().clicked()
            || ui.checkbox(&mut app.search_everywhere, "Search the entire disk").hand_cursor().clicked()
            || ui.checkbox(&mut app.search_whole_word, "Search the whole word").hand_cursor().clicked()
            || ui.checkbox(&mut app.match_case, "Keep the case").hand_cursor().clicked()
        {
            app.visible_dirty = true;
        }
    }).response.hand_cursor().on_hover_text("Search settings");
}

fn search_bar(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
){
    if ui.add(
        egui::TextEdit::singleline(&mut app.search_query)
            .hint_text("Search...")
            .desired_width(200.0)
    ).text_cursor().changed() {
        app.visible_dirty = true;
    }

    if !app.search_query.is_empty(){
        if ui.button("❌").on_hover_text("Clear the search bar").hand_cursor().clicked(){
            app.search_query.clear();
            app.visible_dirty = true;
        }
    }
}