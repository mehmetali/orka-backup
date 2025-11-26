use std::fs;
use std::path::Path;
use std::time::Duration;
use anyhow::Result;

pub async fn cleanup_task(temp_path: String) {
    loop {
        tracing::info!("Running cleanup task...");
        if let Err(e) = cleanup_old_files(&temp_path) {
            tracing::error!("Cleanup task failed: {}", e);
        }
        // Run cleanup every 6 hours
        tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
    }
}

fn cleanup_old_files(temp_path: &str) -> Result<()> {
    let path = Path::new(temp_path);
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let metadata = fs::metadata(&path)?;
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed()? > Duration::from_secs(24 * 60 * 60) {
                    tracing::info!("Deleting old backup file: {:?}", path);
                    fs::remove_file(&path)?;
                }
            }
        }
    }
    Ok(())
}
