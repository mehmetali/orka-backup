use crate::config::Config;
use anyhow::{Result, Context, anyhow};
use reqwest::multipart;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use sha2::{Sha256, Digest};
use std::path::Path;
use std::time::Duration;
use chrono::{DateTime, Utc};

pub struct BackupMeta {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: i64,
    pub filepath: std::path::PathBuf,
}

pub async fn upload_backup(config: &Config, meta: BackupMeta) -> Result<()> {
    let file = File::open(&meta.filepath).await?;
    let checksum = calculate_checksum(&meta.filepath).await?;

    let stream = FramedRead::new(file, BytesCodec::new());
    let file_part = multipart::Part::stream(reqwest::Body::wrap_stream(stream))
        .file_name(meta.filepath.file_name().unwrap().to_str().unwrap().to_string())
        .mime_str("application/octet-stream")?;

    let form = multipart::Form::new()
        .text("server_name", config.backup.server_name.clone())
        .text("database_name", config.mssql.database.clone())
        .text("backup_started_at", meta.start_time.to_rfc3339())
        .text("backup_completed_at", meta.end_time.to_rfc3339())
        .text("duration_seconds", meta.duration_seconds.to_string())
        .text("checksum_sha256", checksum)
        .part("backup_file", file_part);

    let client = reqwest::Client::new();
    let mut attempts = 0;
    let max_attempts = 10;
    let mut delay = Duration::from_secs(1);

    loop {
        attempts += 1;
        tracing::info!("Uploading backup... Attempt {}/{}", attempts, max_attempts);

        let response_result = client.post(&config.api.url)
            .bearer_auth(&config.api.token)
            .multipart(form.try_clone().unwrap()) // It's unfortunate we have to clone the form for retries.
            .send()
            .await;

        match response_result {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json().await?;
                    if json.get("status").and_then(|s| s.as_str()) == Some("ok") {
                        tracing::info!("Upload successful.");
                        return Ok(());
                    } else {
                        tracing::error!("API error: {:?}", json);
                    }
                } else {
                    tracing::error!("Upload failed with status: {}", response.status());
                }
            },
            Err(e) => {
                tracing::error!("Upload request failed: {}", e);
            }
        }

        if attempts >= max_attempts {
            return Err(anyhow!("Upload failed after {} attempts.", max_attempts));
        }

        tokio::time::sleep(delay).await;
        delay *= 2; // Exponential backoff
    }
}

async fn calculate_checksum(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024 * 64]; // 64KB buffer

    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
