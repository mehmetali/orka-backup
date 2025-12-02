use crate::config::Config;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tiberius::{AuthMethod, Client, Config as TiberiusConfig, EncryptionLevel, SqlBrowser};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncReadCompatExt;
use time::OffsetDateTime;
use time::macros::format_description;

pub async fn perform_backup(config: &Config) -> Result<PathBuf> {
    let format = format_description!("[year][month][day]_[hour][minute][second]");
    let backup_filename = format!(
        "{}_{}.bak",
        config.mssql.database,
        OffsetDateTime::now_utc().format(&format)?
    );
    let backup_filepath = Path::new(&config.backup.temp_path).join(&backup_filename);

    std::fs::create_dir_all(&config.backup.temp_path)?;

    let mut client = create_mssql_client(config).await?;

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
    let mut client = create_mssql_client(config).await?;

    let verify_command = format!(
        "RESTORE VERIFYONLY FROM DISK = N'{}'",
        backup_path.to_str().context("Invalid backup path")?
    );

    tracing::info!("Verifying backup...");
    client.execute(verify_command, &[]).await?;
    tracing::info!("Backup verified successfully.");

    Ok(())
}

async fn create_mssql_client(
    config: &Config,
) -> Result<Client<tokio_util::compat::Compat<tokio::net::TcpStream>>> {
    let mut t_config = TiberiusConfig::new();

    if let (Some(user), Some(pass)) = (&config.mssql.user, &config.mssql.pass) {
        t_config.authentication(AuthMethod::sql_server(user, pass));
    } else {
        t_config.authentication(AuthMethod::Integrated);
    }
    t_config.encryption(EncryptionLevel::Required);
    t_config.trust_cert(); // Use for development; configure properly for production

    // Scenario 1: Direct connection with host and port
    if let (Some(host), Some(port)) = (&config.mssql.host, config.mssql.port) {
        tracing::info!("Attempting direct connection to {}:{}", host, port);
        t_config.host(host);
        t_config.port(port);
        let tcp = TcpStream::connect(t_config.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let client = Client::connect(t_config, tcp.compat()).await?;
        tracing::info!("Direct connection successful.");
        return Ok(client);
    }

    // Scenario 2: Connection with instance name lookup
    let host = match &config.mssql.host {
        Some(h) => h.clone(),
        None => hostname::get()?.to_string_lossy().into_owned(),
    };
    t_config.host(&host);

    let instances_to_try: Vec<String> = match &config.mssql.instance_name {
        Some(name) => vec![name.clone()],
        None => vec!["MSSQLSERVER".to_string(), "SQLEXPRESS".to_string()],
    };

    for instance in instances_to_try {
        let mut conn_config = t_config.clone();
        tracing::info!(
            "Attempting to connect to instance '{}' on host '{}'",
            instance,
            host
        );
        conn_config.instance_name(&instance);

        match TcpStream::connect_named(&conn_config).await {
            Ok(tcp) => {
                tcp.set_nodelay(true)?;
                let client = Client::connect(conn_config, tcp.compat()).await?;
                tracing::info!(
                    "Successfully connected to instance '{}' on host '{}'",
                    instance,
                    host
                );
                return Ok(client);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to instance '{}' on host '{}': {}",
                    instance,
                    host,
                    e
                );
                continue;
            }
        }
    }

    bail!("Could not connect to any MSSQL instance on host '{}'", host)
}
