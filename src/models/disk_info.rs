use std::path::PathBuf;

pub struct DiskInfo {
    pub name: String,
    pub mount_point: PathBuf,
    pub mount_point_str: String,
    pub total_gb: String,
    pub available_gb: String,
}