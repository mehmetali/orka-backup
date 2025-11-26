use crate::config::Config;
use anyhow::{Result, Context};
use tiberius::{Client, Config as TiberiusConfig, AuthMethod};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use std::path::{Path, PathBuf};
use chrono::Utc;

pub async fn perform_backup(config: &Config) -> Result<PathBuf> {
    let backup_filename = format!(
        "{}_{}.bak",
        config.mssql.database,
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    let backup_filepath = Path::new(&config.backup.temp_path).join(&backup_filename);

    std::fs::create_dir_all(&config.backup.temp_path)?;

    let tiberius_config = create_tiberius_config(config);
    let tcp = TcpStream::connect(tiberius_config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    let mut client = Client::connect(tiberius_config, tcp.compat_write()).await?;

    let backup_command = format!(
        "BACKUP DATABASE [{}] TO DISK = N'{}' WITH NOFORMAT, NOINIT, NAME = N'{}-Full Database Backup', SKIP, NOREWIND, NOUNLOAD, STATS = 10",
        config.mssql.database,
        backup_filepath.to_str().context("Invalid backup path")?,
        config.mssql.database
    );

    tracing::info!("Starting backup...");
    client.execute(backup_command, &[]).await?;
    tracing::info!("Backup command executed.");

    Ok(backup_filepath)
}

pub async fn verify_backup(config: &Config, backup_path: &Path) -> Result<()> {
    let tiberius_config = create_tiberius_config(config);
    let tcp = TcpStream::connect(tiberius_config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    let mut client = Client::connect(tiberius_config, tcp.compat_write()).await?;

    let verify_command = format!(
        "RESTORE VERIFYONLY FROM DISK = N'{}'",
        backup_path.to_str().context("Invalid backup path")?
    );

    tracing::info!("Verifying backup...");
    client.execute(verify_command, &[]).await?;
    tracing::info!("Backup verified successfully.");

    Ok(())
}

fn create_tiberius_config(config: &Config) -> TiberiusConfig {
    let mut t_config = TiberiusConfig::new();
    t_config.host(&config.mssql.host);
    t_config.port(config.mssql.port);
    t_config.authentication(AuthMethod::sql_server(&config.mssql.user, &config.mssql.pass));
    t_config.trust_cert(); // Use for development; configure properly for production
    t_config
}
