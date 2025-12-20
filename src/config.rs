//! Configuration management for the AIJ server.
use crate::error::{AijError, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

static CONFIG: OnceLock<AijConfig> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub(crate) struct CliConfig {
    /// Path to the configuration file
    #[clap(short, long, default_value = "config.example.toml")]
    pub(crate) config_path: PathBuf,
}

/// Configuration for the AIJ server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AijConfig {
    /// Address the server will listen on
    pub listen_addr: String,
    /// Port the server will listen on
    pub listen_port: u16,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Path of the problems directory
    pub problems_dir: PathBuf,
    /// Path of the log directory
    pub log_dir: PathBuf,
    /// Host of the backend server
    pub backend_host: String,
    /// Port of the backend server
    pub backend_port: u16,
    /// Prefix for backend API endpoints
    pub backend_prefix: String,
    /// Save submission files
    pub save_submissions: bool,
    /// Truncate length of long outputs
    #[serde(default = "default_output_truncate_length")]
    pub output_truncate_length: usize,
}

impl AijConfig {
    /// get global config, initialized by [init_config]
    pub(crate) async fn get() -> Result<&'static AijConfig> {
        CONFIG
            .get()
            .ok_or_else(|| AijError::Config("Configuration is not initialized".to_string()))
    }

    pub(crate) async fn get_backend_base_addr() -> Result<String> {
        let config = AijConfig::get().await?;
        Ok(format!(
            "http://{}:{}{}",
            config.backend_host, config.backend_port, config.backend_prefix
        ))
    }

    /// Initialize the AIJ configuration by reading from the specified config file
    pub async fn init() -> Result<&'static AijConfig> {
        let cli_config = CliConfig::parse();
        let config_content = tokio::fs::read_to_string(&cli_config.config_path)
            .await
            .map_err(|e| {
                AijError::Config(format!(
                    "Failed to read config file {}: {}",
                    cli_config.config_path.display(),
                    e
                ))
            })?;
        let aij_config: AijConfig = toml::from_str(&config_content).map_err(|e| {
            AijError::Config(format!(
                "Failed to parse config file {}: {}",
                cli_config.config_path.display(),
                e
            ))
        })?;
        CONFIG
            .set(aij_config)
            .map_err(|_| AijError::Config("Configuration already initialized".to_string()))?;
        AijConfig::get().await
    }
}

fn default_output_truncate_length() -> usize {
    200
}
