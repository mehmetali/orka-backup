use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::macros::format_description;

pub fn get_log_filepath() -> PathBuf {
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let log_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));

    let format = format_description!("[year]-[month]-[day]");
    let now = OffsetDateTime::now_utc();
    let (h, m, s) = time::UtcOffset::current_local_offset().unwrap_or_else(|_| time::UtcOffset::UTC).as_hms();
    let today = now.to_offset(time::UtcOffset::from_hms(h,m,s).unwrap()).format(&format).unwrap();

    log_dir.join(format!("service.log.{}", today))
}
