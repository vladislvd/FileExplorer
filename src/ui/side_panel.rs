use eframe::egui;
use crate::app::FileExplorer;

pub fn draw_side_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    let screen_width = ctx.content_rect().width();
    let mut selected_disk = None;

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .width_range(50.0..=screen_width - 50.0)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::default()), |ui|{
                ui.vertical(|ui|{
                    ui.label(egui::RichText::new("Disks: ").size(15.0));

                    for disk in &app.all_disks {
                        let new_path = disk.mount_point.clone();
                        let clicked = ui
                            .selectable_label(
                                false,
                                format!("{} ({})", disk.name, disk.mount_point_str)
                            )
                            .on_hover_text(
                                format!("Total space:{} Gb\nAvailable space: {} Gb",
                                        disk.total_gb,
                                        disk.available_gb
                                )
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked();
                        if clicked {
                            selected_disk = Some(new_path);
                        }
                    }
                });
            });
        });
    if let Some(path) = selected_disk {
        app.current_disk = path.clone();
        app.current_path = path;
        app.update_index();
        app.visible_dirty = true;
    }
}