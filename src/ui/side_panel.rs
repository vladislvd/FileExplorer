use std::path::PathBuf;
use eframe::egui;
use egui_material_icons::icons;
use crate::app::FileExplorer;
use crate::ui::extensions::PointHandTrait;


pub fn draw_side_panel(
    app: &mut FileExplorer,
    ctx: &egui::Context
){
    let screen_width = ctx.content_rect().width();
    let mut selected_disk: Option<PathBuf> = None;
    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(200.0)
        .width_range(60.0..=screen_width - 50.0)
        .show(ctx, |ui| {
            ui.set_min_width(ui.available_width());
            selected_disk = draw_disks(ui, app);
        });

    app.handle_disk_action(selected_disk);
}

fn draw_disks(
    ui: &mut egui::Ui,
    app: &mut FileExplorer,
) -> Option<PathBuf> {
    let mut result: Option<PathBuf> = None;

    ui.vertical(|ui|{
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        ui.label(egui::RichText::new("Disks: ").size(15.0));

        for disk in &app.all_disks {
            let new_path = disk.mount_point.clone();
            if ui
                .selectable_label(
                    false,
                    format!("{} {} ({})",
                            icons::ICON_STORAGE,
                            disk.name,
                            disk.mount_point_str
                    )
                )
                .on_hover_text(
                    format!("Total space: {} Gb\nAvailable space: {} Gb",
                            disk.total_gb,
                            disk.available_gb
                    )
                )
                .hand_cursor()
                .clicked()
            {
                result = Some(new_path);
            }
        }
    });
    result
}

