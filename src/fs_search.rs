use std::{
    error::Error,
    fs::{read_dir, FileType},
    num::NonZeroUsize,
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex},
    thread::{self, JoinHandle},
};

use log::{debug, trace};

const DIRS_PER_TICK: usize = 24;

pub struct SearchData {
    pub path: PathBuf,
    pub pattern: String,
    pub file_type: Option<FileType>,
}

fn set_state(states: Arc<Mutex<Vec<bool>>>, thread_id: usize, working: bool) {
    let mut states_lock = states.lock().unwrap();
    if let Some(state) = states_lock.get_mut(thread_id) {
        *state = working
    }
}

fn is_someone_working(states: Arc<Mutex<Vec<bool>>>) -> bool {
    let states_lock = states.lock().unwrap();
    for val in states_lock.iter() {
        if *val == true {
            return true;
        }
    }
    false
}

pub fn search(
    search_data: SearchData,
    thread_count: NonZeroUsize,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let dir_paths: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(vec![search_data.path]));
    let states: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![]));
    let pattern: Arc<String> = Arc::new(search_data.pattern);

    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    let (tx, rx) = channel::<PathBuf>();

    for thread_id in 0..thread_count.get() {
        let dir_paths = Arc::clone(&dir_paths);
        let states: Arc<Mutex<Vec<bool>>> = Arc::clone(&states);
        let pattern = Arc::clone(&pattern);
        let tx = tx.clone();

        handles.push(thread::spawn(move || {
            let mut states_lock = states.lock().unwrap();
            states_lock.push(true);
            drop(states_lock);

            debug!("[{thread_id}]: spawned");

            'thread_tick: loop {
                if !is_someone_working(Arc::clone(&states)) {
                    trace!("Noone working anymore");
                    break 'thread_tick;
                }

                let mut dirs_to_scan: Vec<PathBuf> = vec![];

                let mut dir_paths_lock = dir_paths.lock().unwrap();
                if dir_paths_lock.len() == 0 {
                    set_state(Arc::clone(&states), thread_id, false);
                    continue 'thread_tick;
                }
                set_state(Arc::clone(&states), thread_id, true);
                for _ in 0..DIRS_PER_TICK {
                    if let Some(popped) = dir_paths_lock.pop() {
                        dirs_to_scan.push(popped);
                    }
                }
                drop(dir_paths_lock);

                let mut next_dirs_to_scan: Vec<PathBuf> = Vec::new();

                'dir_loop: for dir in &dirs_to_scan {
                    trace!("[{thread_id}]: scanning {}", dir.display());

                    match read_dir(&dir) {
                        Ok(read_dir) => {
                            for entry in read_dir {
                                if let Ok(entry) = entry {
                                    if let Some(path) = entry.path().file_name() {
                                        if let Some(path) = path.to_str() {
                                            if path.to_lowercase().contains(&pattern.to_lowercase())
                                            {
                                                debug!(
                                                    "[{thread_id}]: found {}",
                                                    entry.path().display()
                                                );
                                                tx.send(entry.path()).unwrap();
                                            }
                                        }
                                    }
                                    if entry.path().is_dir() {
                                        next_dirs_to_scan.push(entry.path());
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            trace!("[{thread_id}]: {err} in {}", dir.display());
                            continue 'dir_loop;
                        }
                    }
                }

                if next_dirs_to_scan.is_empty() {
                    continue 'thread_tick;
                }

                let mut dir_paths_lock = dir_paths.lock().unwrap();
                for dir in next_dirs_to_scan {
                    dir_paths_lock.push(dir);
                }
                drop(dir_paths_lock);
            }

            let mut states_lock = states.lock().unwrap();
            if let Some(state) = states_lock.get_mut(thread_id) {
                *state = false
            }
            drop(states_lock);

            debug!("[{thread_id}]: finished");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut found_paths = vec![];

    drop(tx);
    while let Ok(recved) = rx.recv() {
        found_paths.push(recved);
    }

    Ok(found_paths)
}
