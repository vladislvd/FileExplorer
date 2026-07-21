use std::sync::atomic;
use eframe::egui;
use eframe::egui::CursorIcon;
use crate::app::FileExplorer;
use crate::models::SortBy;
use crate::services::paste_operation;


pub fn draw_top_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    egui::TopBottomPanel::top("main_top_bar").show(ctx, |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui|{
            ui.with_layout(egui::Layout::left_to_right(egui::Align::default()), |ui|{
                ui.group(|ui|{

                    let alt_down_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowDown)));
                    if ui.button("🏠").on_hover_text("To the home directory (ALT + ArrowDown").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || alt_down_arrow_pressed {
                        if let Some(home_dir) = dirs::home_dir(){
                            app.path_history.push(app.current_path.clone());
                            app.current_path = home_dir;
                            app.visible_dirty = true;
                        }
                    }

                    let alt_up_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowUp)));
                    if ui.button("^").on_hover_text("To the parent directory (ALT + ArrowUp").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || alt_up_arrow_pressed {
                        if let Some(parent) = app.current_path.parent() {
                            app.path_history.push(app.current_path.clone());
                            app.current_path = parent.to_path_buf();
                            app.visible_dirty = true;
                        }
                    }

                    let alt_left_arrow_pressed = ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::ArrowLeft)));
                    if ui.button("<--").on_hover_text("To the previous directory (ALT + ArrowLeft)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || alt_left_arrow_pressed {
                        if let Some(future_path) = app.path_history.pop(){
                            app.current_path = future_path;
                            app.visible_dirty = true;
                        }
                    }

                    ui.label(format!("{}", app.current_path.to_string_lossy()));
                        if !app.is_indexing.clone().load(atomic::Ordering::Relaxed){
                            let f5_pressed = ui.input(|i| i.key_pressed(egui::Key::F5));

                            if ui.button("🔄").on_hover_text("Update cache(F5)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() || f5_pressed {
                                app.update_index();
                            }
                            let time = if let Ok(t) = app.index_time.read() { *t } else { std::time::Duration::ZERO };

                            ui.label(format!("{:.2?}", time));
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
                                if ui.checkbox(&mut app.show_hidden, "Show hidden").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                                ui.separator();
                                ui.label("Sort by:");
                                if ui.radio_value(&mut app.sort_by, SortBy::Date, "Date").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                                if ui.radio_value(&mut app.sort_by, SortBy::Name, "Name").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                                if ui.radio_value(&mut app.sort_by, SortBy::Type, "Type").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                                ui.separator();
                                if ui.radio_value(&mut app.sort_ascending, true, "⬆ Ascending (A-Z)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                                if ui.radio_value(&mut app.sort_ascending, false, "⬇ Descending (Z-A)").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                    app.visible_dirty = true;
                                }
                            });
                            ui.separator();
                            if ui.checkbox(&mut app.index_all, "Indexing all").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                                app.update_index();
                            }
                        });
                    });

                    let mut is_clipboard_empty = true;
                    if app.clipboard.is_some() {
                        is_clipboard_empty = false;
                    }

                    let paste_button = ui.add_enabled_ui(!is_clipboard_empty, |ui| {
                        ui.button("📋").on_hover_text("Paste into this directory").on_hover_cursor(CursorIcon::PointingHand)
                    }).inner;

                    if paste_button.clicked(){
                        paste_operation(app);
                    }

                    let clear_clipboard_button = ui.add_enabled_ui(!is_clipboard_empty, |ui| {
                        ui.button("🧹").on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Clear the clipboard")
                    }).inner;

                    if clear_clipboard_button.clicked() {
                        app.clipboard = None
                    }
                });
                ui.add_space(50.0);
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::default()), |ui|{
                ui.group(|ui|{

                    let menu_res = ui.menu_button("🔽", |ui|{
                        ui.set_min_width(150.0);
                        if ui.checkbox(&mut app.search_hidden, "Search hidden").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            app.visible_dirty = true;
                        }
                        if ui.checkbox(&mut app.search_venv, "Search venv").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            app.visible_dirty = true;
                        }
                        if ui.checkbox(&mut app.search_everywhere, "Search anywhere").on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("or start from this directory").clicked(){
                            app.visible_dirty = true;
                        }
                        if ui.checkbox(&mut app.search_whole_word, "Search the whole world").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            app.visible_dirty = true;
                        }
                        if ui.checkbox(&mut app.match_case, "Keep the case").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            app.visible_dirty = true;
                        }
                    });
                    menu_res.response.on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Search settings");

                    let search_bar = ui.add(
                        egui::TextEdit::singleline(&mut app.search_query)
                            .hint_text("Search...")
                            .desired_width(200.0)
                    );
                    if search_bar.changed(){
                        app.visible_dirty = true;
                    }
                    if search_bar.hovered(){
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }

                    if !app.search_query.is_empty(){
                        if ui.button("❌").on_hover_text("Clear the search bar").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(){
                            app.search_query.clear();
                            app.visible_dirty = true;
                        }
                    }
                    if app.search_query.is_empty(){
                        ui.add_enabled(false, egui::Button::new("🔎"));
                    }

                });
            });
        });
        ui.add_space(10.0);
    });
}