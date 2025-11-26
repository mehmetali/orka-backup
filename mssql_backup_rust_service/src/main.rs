mod config;
mod backup;
mod upload;
mod cleanup;

use anyhow::Result;
use std::time::Duration;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
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
