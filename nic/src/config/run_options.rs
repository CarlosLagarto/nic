use std::{
    env,
    path::{Path, PathBuf},
};

use getopts::Options;
use tracing::warn;

use crate::{config::CONFIG_FILE, utils::remove_folder_from_path};

#[derive(Clone, Debug, Default)]
pub struct Args {
    pub cfg_file: PathBuf,
    // test helper
    pub cfg_str: Option<String>,
}

pub fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [config_file]", program);
    print!("{}", opts.usage(&brief));
}

pub fn get_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let opts = Options::new();

    let default_args = Args {
        cfg_file: default_cfg_file(),
        cfg_str: None,
    };
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            warn!("Error parsing arguments: {}", f);
            warn!("Proceeding with defaults.");
            print_usage(&program, opts);
            return default_args;
        }
    };

    let config_file_path = matches.free.first().map(|s| s.as_str());
    let Some(config_file_path) = config_file_path else {
        return default_args;
    };
    let path = remove_folder_from_path(Path::new(config_file_path), "");

    // Attempt to load the config file, but proceed with default if it fails
    if !path.exists() {
        eprintln!(
            "Warning: Config file '{}' does not exist. Proceeding with defaults.",
            config_file_path
        );
        return default_args;
    }

    Args { cfg_file: path , cfg_str: None}
}

pub fn default_cfg_file() -> PathBuf {
    let config_path = std::env::current_dir().unwrap();
    let mut new_configpath = remove_folder_from_path(&config_path, "");

    new_configpath.push(CONFIG_FILE);
    new_configpath
}
