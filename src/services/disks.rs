use std::sync::mpsc::{channel, Receiver};
use sysinfo::Disks;
use std::thread;
use std::time::Duration;
use crate::app::FileExplorer;
use crate::models::DiskInfo;

pub fn start_disks_monitoring() -> Receiver<Disks>{
    let (tx, rx) = channel();
    thread::spawn(move || {
        loop {
            let disks = Disks::new_with_refreshed_list();
            if tx.send(disks).is_err() { break; }
            thread::sleep(Duration::from_secs(1));
        }
    });
    rx
}

pub fn get_new_disks(app: &mut FileExplorer){
    if let Ok(new_disks) = app.disk_receiver.try_recv(){
        app.all_disks.clear();
        for disk in &new_disks{
            let name = disk.name().to_string_lossy().into_owned();
            let info = DiskInfo {
                name: if !name.is_empty() { name } else { "Storage device".to_string() },
                mount_point: disk.mount_point().to_path_buf(),
                mount_point_str: disk.mount_point().to_string_lossy().into_owned(),
                total_gb: format!("{:.2}",disk.total_space() as f64 / 1_000_000_000.0).to_string(),
                available_gb: format!("{:.2}",disk.available_space() as f64 / 1_000_000_000.0).to_string(),
            };
            app.all_disks.push(info);
        }
    }
}