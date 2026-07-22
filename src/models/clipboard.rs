use std::path::PathBuf;

#[derive(PartialEq)]
pub enum ClipboardOperation{
    Copy,
    Cut,
}

#[derive(PartialEq)]
pub struct AppClipboard{
    pub source_path: PathBuf,
    pub operation: ClipboardOperation,
}