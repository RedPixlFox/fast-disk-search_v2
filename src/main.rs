// #![allow(dead_code, unused_variables, unused_imports)]
// #![allow(unused_mut)]

use std::{num::NonZeroUsize, path::PathBuf, time::Instant};

use log::{self, error, info};

use crate::fs_search::{search, SearchData};

mod fs_search;

// ------------------------------------------------

const LOG_LEVEL: &str = "debug";

// ------------------------------------------------

fn init_logger() {
    let env = env_logger::Env::default()
        .filter_or("MY_LOG_LEVEL", LOG_LEVEL)
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);
}

// ------------------------------------------------

fn main() {
    init_logger();
    log::trace!("Hey there!");

    let start = Instant::now();

    let search_data = SearchData {
        path: PathBuf::from("C:\\"),
        pattern: String::from(".rs"),
        file_type: None,
    };

    let result = search(search_data, NonZeroUsize::new(8).unwrap());
    let elapsed = start.elapsed();

    match result {
        Ok(result) => {
            let count = result.len();

            for i in 0..result.len() {
                info!("[{}/{count}] {}", i + 1, result[i].display())
            }

            info!("found {} after {:?}", result.len(), elapsed);
        }
        Err(err) => {
            error!("{err}")
        }
    };
}
