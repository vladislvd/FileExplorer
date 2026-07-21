use std::{
    cmp,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
        RwLock,
    },
    thread,
    time::{Duration, Instant},
};
use parking_lot::Mutex;
use smol_str::SmolStr;
use winapi::um::{
    processthreadsapi::{GetCurrentThread, SetThreadPriority},
    winbase::THREAD_PRIORITY_BELOW_NORMAL,
};

use crate::models::FileInfo;


pub fn start_indexing(
    mut root: PathBuf,
    index_all: bool,

    static_index: Arc<RwLock<Vec<FileInfo>>>,
    is_indexing: Arc<AtomicBool>,
    cancel_indexing: Arc<AtomicBool>,

    index_time: Arc<RwLock<Duration>>,

) {
    normalize_root(&mut root);

    thread::spawn(move || {
        is_indexing.store(true, Ordering::Relaxed);
        
        lower_thread_priority();
        thread::sleep(Duration::from_micros(50));

        let start_time = Instant::now();

        let files = collect_files(
            root,
            index_all,
            Arc::clone(&cancel_indexing),
        );

        if cancel_indexing.load(Ordering::Relaxed) {
            is_indexing.store(false, Ordering::Relaxed);
            return;
        }

        if let Ok(mut lock) = static_index.write() {
            *lock = files;
        }

        if let Ok(mut lock) = index_time.write() {
            *lock = start_time.elapsed();
        }

        is_indexing.store(false, Ordering::Relaxed);
    });
}

fn collect_files(
    root: PathBuf,
    index_all: bool,
    cancel: Arc<AtomicBool>,
) -> Vec<FileInfo> {
    let mut builder = ignore::WalkBuilder::new(root);

    builder
        .hidden(false)
        .follow_links(false)
        .threads(cmp::max(2, num_cpus::get()/2));

    if !index_all {
        builder.filter_entry(filter_entry);
    }

    let walker = builder.build_parallel();

    let shared = Arc::new(Mutex::new(Vec::with_capacity(800_000)));

    walker.run(|| {
        let cancel = Arc::clone(&cancel);
        let shared = Arc::clone(&shared);

        Box::new(move |result| {
            if cancel.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }
            if let Ok(entry) = result{
                if let Some(info) = build_file_info(entry){
                    shared.lock().push(info);
                }
            }
            ignore::WalkState::Continue
        })
    });

    std::mem::take(&mut *shared.lock())
}

fn build_file_info(entry: ignore::DirEntry) -> Option<FileInfo>{
    use std::os::windows::fs::MetadataExt;

    let name = entry.file_name().to_string_lossy();
    let meta = entry.metadata().ok();
    let mut is_hidden = name.starts_with(".");

    #[cfg(windows)]
    if !is_hidden {
        is_hidden = meta.map(|m| m.file_attributes() & 0x2 != 0).unwrap_or(false);
    }

    Some(FileInfo{
        is_hidden,
        is_venv: name == "venv",
        name: SmolStr::new(&name),
        is_dir: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
        created_at: entry.metadata().ok().and_then(|m| m.created().ok()).unwrap_or(std::time::SystemTime::now()),
        path: entry.into_path(),
    })
}

fn filter_entry(entry: &ignore::DirEntry) -> bool {
    if entry.depth() == 0{
        return true;
    }

    let path = entry.path();

    if let Some(ext) = path.extension() {
        if ext == "log" {
            return false;
        }
    }

    !matches!(
        entry.file_name().to_str(),
            Some("Windows")
            | Some("$Recycle.Bin")
            | Some("$SysReset")
            //| Some("hp")
            | Some("System.sav")
            | Some("AppData")
            | Some("Default")
            | Some("Recovery")
    )
}

fn normalize_root(root: &mut PathBuf){
    #[cfg(windows)]
    {
        let path_str = root.to_string_lossy().to_string();
        if path_str.len() == 2 && path_str.ends_with(":") {
            *root = PathBuf::from(format!("{}\\", path_str));
        }
        if path_str.len() == 1 && path_str.chars().next().unwrap().is_ascii_alphabetic() {
            *root = PathBuf::from(format!("{}:\\",path_str));
        }
    }
}

fn lower_thread_priority(){
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL as i32);
    }
}