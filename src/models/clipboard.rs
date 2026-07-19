use std::path::PathBuf;

pub enum ClipboardOperation{
    Copy,
    Cut,
}

pub struct AppClipboard{
    pub source_path: PathBuf,
    pub operation: ClipboardOperation,
}