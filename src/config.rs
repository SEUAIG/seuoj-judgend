use crate::error::{AijError, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

static CONFIG: OnceLock<AijConfig> = OnceLock::new();
/// Initialize the AIJ configuration by reading from the specified config file
pub(crate) async fn init_config() -> Result<&'static AijConfig> {
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
    CONFIG.set(aij_config).map_err(
        |_| AijError::Config("Configuration already initialized".to_string()),
    )?;
    AijConfig::get().await
}

#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub(crate) struct CliConfig {
    /// Path to the configuration file
    #[clap(short, long, default_value = "config.toml")]
    pub(crate) config_path: PathBuf,
}

/// Configuration for the AIJ server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct AijConfig {
    /// Address the server will listen on
    pub(crate) listen_addr: String,
    /// Port the server will listen on
    pub(crate) listen_port: u16,
    /// Maximum number of concurrent requests
    pub(crate) max_concurrent_requests: usize,
    /// Path of the problems directory
    pub(crate) problems_dir: PathBuf,
    /// Path of the log directory
    pub(crate) log_dir: PathBuf,
    /// Host of the backend server
    pub(crate) backend_host: String,
    /// Port of the backend server
    pub(crate) backend_port: u16,
    /// Prefix for backend API endpoints
    pub(crate) backend_prefix: String,
    /// Save submission files
    pub(crate) save_submissions: bool,
}

impl AijConfig {
    /// get global config, initialized by [init_config]
    pub(crate) async fn get() -> Result<&'static AijConfig> {
        CONFIG.get().ok_or_else(|| {
            AijError::Config("Configuration is not initialized".to_string())
        })
    }

    pub(crate) async fn get_backend_base_addr() -> Result<String> {
        let config = AijConfig::get().await?;
        Ok(format!(
            "http://{}:{}{}",
            config.backend_host, config.backend_port, config.backend_prefix
        ))
    }
}
