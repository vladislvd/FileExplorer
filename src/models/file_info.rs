use std::{
    path::PathBuf,
    time::SystemTime,
};
use smol_str::SmolStr;

#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: SmolStr,
    pub is_dir: bool,
    pub created_at: SystemTime,
    pub is_hidden: bool,
    pub is_venv: bool,
}