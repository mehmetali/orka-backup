mod config;
mod backup;
mod upload;
mod cleanup;
mod ui;

use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use chrono::Utc;

#[cfg(windows)]
use {
    std::ffi::OsString,
    windows_service::{define_windows_service, service_dispatcher},
};

#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

#[cfg(windows)]
fn service_main(_args: Vec<OsString>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(run_app()) {
        tracing::error!("Service failed: {}", e);
    }
}

#[cfg(windows)]
fn main() -> Result<()> {
    // Attempt to run as a service first.
    if let Err(e) = service_dispatcher::start(config::SERVICE_NAME, ffi_service_main) {
        if let Some(io_error) = e.get_ref() {
            if io_error.kind() == std::io::ErrorKind::Other && io_error.raw_os_error() == Some(1063) {
                // This is our cue to run interactively.
                run_interactive()?;
            } else {
                return Err(e.into());
            }
        } else {
            return Err(e.into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() -> Result<()> {
    run_interactive()
}

fn run_interactive() -> Result<()> {
    if !Path::new("config.toml").exists() {
        if ui::show_setup_window()? {
            #[cfg(windows)]
            {
                use std::process::Command;
                // Config saved, now install and start the service
                let exe_path = std::env::current_exe()?;
                let bin_path = format!("binPath=\"{}\"", exe_path.display());
                let status = Command::new("sc")
                    .args(&["create", config::SERVICE_NAME, &bin_path])
                    .status()?;
                if !status.success() {
                    fltk::dialog::alert_default("Failed to install service. Please run as Administrator.");
                    return Err(anyhow::anyhow!("Failed to install service"));
                }
                let status = Command::new("sc")
                    .args(&["start", config::SERVICE_NAME])
                    .status()?;
                if !status.success() {
                    fltk::dialog::alert_default("Failed to start service. Please check the logs.");
                    return Err(anyhow::anyhow!("Failed to start service"));
                }
            }
        }
    } else {
        #[cfg(windows)]
        {
            println!("Service is already configured. To manage the service, use:");
            println!("sc start {}", config::SERVICE_NAME);
            println!("sc stop {}", config::SERVICE_NAME);
            println!("sc delete {}", config::SERVICE_NAME);
        }
        #[cfg(not(windows))]
        {
            println!("Application is configured. Starting backup cycle...");
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(run_app())?;
        }
    }
    Ok(())
}

async fn run_app() -> Result<()> {
    tracing_subscriber::fmt::init();

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
