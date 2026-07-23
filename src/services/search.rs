use std::{
    path::PathBuf,
    sync::RwLockReadGuard
};
use rayon::prelude::*;
use crate::models::FileInfo;

pub fn searching_engine(
    index: RwLockReadGuard<Vec<FileInfo>>,
    query: &str,
    current_path: &PathBuf,
    search_hidden: bool,
    search_venv: bool,
    search_everywhere: bool,
    search_whole_word: bool,
    match_case: bool,
) -> Vec<FileInfo> {
    index.par_iter().filter(|file|{
        if !search_hidden && file.is_hidden { return false; }
        if !search_venv && file.is_venv { return false; }
        if !search_everywhere && !file.path.starts_with(&current_path) { return false; }
        if match_case {
            if search_whole_word {
                file.name == query
            } else {
                file.name.contains(query)
            }
        } else {
            if search_whole_word {
                file.name.eq_ignore_ascii_case(query)
            } else {
                if query.is_empty() { return true; }
                file.name.as_bytes()
                    .windows(query.len())
                    .any(|window| {
                        window.eq_ignore_ascii_case(query.as_bytes())
                    })
            }
        }
    })
    .take_any(500)
    .cloned()
    .collect()
}