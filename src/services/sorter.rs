use std::{
    path::PathBuf, 
    sync::RwLockReadGuard
};
use rayon::prelude::*;
use crate::models::FileInfo;
use crate::models::SortBy;
use crate::services::searching_engine;

pub fn sorting(
    current_path: &PathBuf,
    index: RwLockReadGuard<Vec<FileInfo>>,
    query: &str,
    search_hidden: bool,
    show_hidden: bool,
    search_venv: bool,
    search_everywhere: bool,
    search_whole_word: bool,
    match_case: bool,
    sort_ascending: bool,
    visible_files: &mut Vec<FileInfo>,
    visible_dirty: &mut bool,
    sort_by: SortBy,
    
) {
    let mut filtered: Vec<FileInfo> = if query.is_empty() {
        sort_hidden_files(
            index,
            show_hidden,
            current_path,
        )
    } else {
        searching_engine(
            index,
            query,
            current_path,
            search_hidden,
            search_venv,
            search_everywhere,
            search_whole_word,
            match_case
        )
    };

    deep_sorting(&mut filtered, sort_by, sort_ascending);

    *visible_files = filtered;
    *visible_dirty = false;
}

fn sort_hidden_files(
    index: RwLockReadGuard<Vec<FileInfo>>,
    show_hidden: bool,
    current_path: &PathBuf,
) -> Vec<FileInfo>{
    index.par_iter()
        .filter(|file| {
            if !show_hidden && file.is_hidden { return false; }
            file.path.parent().map_or(false, |p| p == current_path)
        })
        .cloned()
        .collect()
}

pub fn deep_sorting(
    files: &mut Vec<FileInfo>,  
    sort_by: SortBy,
    sort_ascending: bool,
){
    files.par_sort_by(move |a, b| {
        let result = match sort_by {
            SortBy::Date => a.created_at.cmp(&b.created_at),
            SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortBy::Type => b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)),
        };
        if sort_ascending { result } else { result.reverse() }
    });
}