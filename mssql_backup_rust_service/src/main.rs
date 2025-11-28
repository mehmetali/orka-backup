#![windows_subsystem = "windows"]
mod config;
mod backup;
mod upload;
mod cleanup;
mod ui;
mod logging;

use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use chrono::Utc;

fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_path = logging::get_log_filepath();
    let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
    let log_filename = log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("service.log"));
    let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    Ok(guard)
}

enum Message {
    ViewLogs,
    EditConfig,
    Quit,
}

#[cfg(windows)]
fn main() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::{synchapi, errhandlingapi, winuser};
    use winapi::shared::minwindef::FALSE;

    let mutex_name: Vec<u16> = OsStr::new("mssql-backup-rust-service-mutex")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mutex = unsafe {
        synchapi::CreateMutexW(std::ptr::null_mut(), FALSE, mutex_name.as_ptr())
    };

    if unsafe { errhandlingapi::GetLastError() } == 183 { // ERROR_ALREADY_EXISTS
        let msg: Vec<u16> = OsStr::new("Application is already running.")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let title: Vec<u16> = OsStr::new("MSSQL Backup Service")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            winuser::MessageBoxW(std::ptr::null_mut(), msg.as_ptr(), title.as_ptr(), winuser::MB_OK | winuser::MB_ICONINFORMATION);
        }
        return;
    }

    let _app = fltk::app::App::default();
    if let Err(e) = run() {
        fltk::dialog::alert_default(&format!("Application Error: {}", e));
    }
}

fn run() -> Result<()> {
    // FLTK UI must run on the main thread.
    // Check for config file first. If it doesn't exist, run setup.
    if !Path::new("config.toml").exists() {
        // If the user saves the config, continue to start the tray icon.
        // If they close the window without saving, the function returns false and the app exits.
        if !ui::show_setup_window()? {
            return Ok(());
        }
    }

    let _guard = init_logging().expect("Failed to initialize logging.");

    let mut tray = tray_item::TrayItem::new(
        "MSSQL Backup Service",
        "app-icon",
    )?;

    let (tx, rx) = std::sync::mpsc::channel();

    let view_logs_tx = tx.clone();
    tray.add_menu_item("View Logs", move || {
        view_logs_tx.send(Message::ViewLogs).unwrap();
    })?;

    let edit_config_tx = tx.clone();
    tray.add_menu_item("Edit Config", move || {
        edit_config_tx.send(Message::EditConfig).unwrap();
    })?;

    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        quit_tx.send(Message::Quit).unwrap();
    })?;

    let _backup_thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(run_app()) {
            tracing::error!("Backup thread failed: {}", e);
        }
    });

    loop {
        match rx.recv() {
            Ok(Message::ViewLogs) => {
                if let Err(e) = ui::show_log_window() {
                    tracing::error!("Failed to show log window: {}", e);
                }
            }
            Ok(Message::EditConfig) => {
                if let Err(e) = ui::show_setup_window() {
                    tracing::error!("Failed to show setup window: {}", e);
                }
            }
            Ok(Message::Quit) => {
                // TODO: properly handle thread shutdown
                break;
            }
            Err(_) => break, // Sender dropped, exit
        }
    }

    Ok(())
}


#[cfg(not(windows))]
fn main() -> Result<()> {
    // Non-windows version doesn't have a tray icon, so we just run the app directly.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_app())
}

async fn run_app() -> Result<()> {
    let config = config::load_config("config.toml")?;

    let cleanup_config_path = config.backup.temp_path.clone();
    tokio::spawn(async move {
        cleanup::cleanup_task(cleanup_config_path).await;
    });

    loop {
        tracing::info!("Starting backup cycle...");
        match run_backup_cycle(&config).await {
            Ok(_) => tracing::info!("Backup cycle completed successfully."),
            Err(e) => tracing::error!("Backup cycle failed: {}", e),
        }

        // Wait for 24 hours before the next backup cycle
        tracing::info!("Waiting for 24 hours until the next cycle.");
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

async fn run_backup_cycle(config: &config::Config) -> Result<()> {
    let start_time = Utc::now();

    // 1. Perform backup
    let backup_filepath = match backup::perform_backup(config).await {
        Ok(path) => {
            tracing::info!("Backup created at: {:?}", path);
            path
        },
        Err(e) => {
            anyhow::bail!("Failed to perform backup: {}", e);
        }
    };

    // 2. Verify backup
    if let Err(e) = backup::verify_backup(config, &backup_filepath).await {
        std::fs::remove_file(&backup_filepath)?;
        anyhow::bail!("Failed to verify backup: {}", e);
    }

    let end_time = Utc::now();
    let duration_seconds = end_time.signed_duration_since(start_time).num_seconds();

    let meta = upload::BackupMeta {
        start_time,
        end_time,
        duration_seconds,
        filepath: backup_filepath.clone(),
    };

    // 3. Upload backup
    if let Err(e) = upload::upload_backup(config, meta).await {
         // Don't delete the file on upload failure, so it can be retried or manually recovered.
         // The cleanup task will eventually remove it.
        anyhow::bail!("Failed to upload backup: {}", e);
    }

    // 4. Delete local backup file after successful upload
    if let Err(e) = std::fs::remove_file(&backup_filepath) {
        tracing::error!("Failed to delete local backup file {:?}: {}", backup_filepath, e);
    } else {
        tracing::info!("Local backup file {:?} deleted.", backup_filepath);
    }

    Ok(())
}
