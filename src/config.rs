use crate::error::{AijError, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Initialize the AIJ configuration by reading from the specified config file
pub(crate) async fn init_config() -> Result<AijConfig> {
    let cli_config = CliConfig::parse();
    let config_content =
        tokio::fs::read_to_string(&cli_config.config_path).await
            .map_err(|e| {
                AijError::ConfigError(format!(
                    "Failed to read config file {}: {}",
                    cli_config.config_path.display(),
                    e
                ))
            })?;
    let aij_config: AijConfig =
        toml::from_str(&config_content).map_err(|e| {
            AijError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                cli_config.config_path.display(),
                e
            ))
        })?;
    Ok(aij_config)
}


#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub(crate) struct CliConfig {
    /// Path to the configuration file
    #[clap(short, long, default_value = "config.toml")]
    pub(crate) config_path: PathBuf,
}


/// Configuration for the AIJ server
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}