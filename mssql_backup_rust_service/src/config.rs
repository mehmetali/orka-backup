use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct Config {
    pub mssql: MssqlConfig,
    pub api: ApiConfig,
    pub backup: BackupConfig,
}

#[derive(Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct MssqlConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub database: String,
    pub instance_name: Option<String>,
}

#[derive(Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct ApiConfig {
    pub url: String,
    pub server_token: String,
    pub auth_token: String,
}

#[derive(Deserialize, serde::Serialize, Debug, Default, Clone)]
pub struct BackupConfig {
    pub temp_path: String,
}

pub fn load_config(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save_config(path: &str, config: &Config) -> Result<()> {
    let content = toml::to_string(config)?;
    fs::write(path, content)?;
    Ok(())
}
