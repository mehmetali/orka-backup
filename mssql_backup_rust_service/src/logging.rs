use chrono::Local;
use std::path::{Path, PathBuf};

pub fn get_log_filepath() -> PathBuf {
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let log_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    let today = Local::now().format("%Y-%m-%d").to_string();
    log_dir.join(format!("service.log.{}", today))
}
