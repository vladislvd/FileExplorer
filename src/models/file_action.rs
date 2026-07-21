use std::path::PathBuf;

#[derive(Clone)]
pub enum FileAction{
    Open(PathBuf),
    Copy(PathBuf),
    Cut(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
}