use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub mssql: MssqlConfig,
    pub api: ApiConfig,
    pub backup: BackupConfig,
}

#[derive(Deserialize, Debug)]
pub struct MssqlConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub database: String,
    pub instance_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ApiConfig {
    pub url: String,
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct BackupConfig {
    pub temp_path: String,
}

pub fn load_config(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
