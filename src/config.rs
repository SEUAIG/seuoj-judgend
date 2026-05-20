//! Configuration management for the AIJ server.

use crate::error::{AijError, Result};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, RwLockWriteGuard};
use tracing::{error, info, warn};

static CONFIG: OnceLock<AijConfig> = OnceLock::new();

/// Configuration for the AIJ server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AijConfig {
    /// Address the server will listen on
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    /// Port the server will listen on
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    /// Maximum number of concurrent requests
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
    /// Path of the problems directory
    #[serde(default = "default_problems_dir")]
    pub problems_dir: PathBuf,
    /// Path of the log directory
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
    /// Path of the submissions directory
    #[serde(default = "default_submissions_dir")]
    pub submissions_dir: PathBuf,
    /// Path of testlib directory
    #[serde(default = "default_testlib_dir")]
    pub testlib_dir: PathBuf,
    /// Host of the backend server
    #[serde(default = "default_backend_host")]
    pub backend_host: String,
    /// Port of the backend server
    #[serde(default = "default_backend_port")]
    pub backend_port: u16,
    /// Prefix for backend API endpoints
    #[serde(default = "default_backend_prefix")]
    pub backend_prefix: String,
    /// Save submission files
    #[serde(default = "default_save_submissions")]
    pub save_submissions: bool,
    /// Truncate length of long outputs
    #[serde(default = "default_output_truncate_length")]
    pub output_truncate_length: usize,
    /// List of available toolchains
    #[serde(default = "default_toolchains")]
    pub toolchains: Vec<String>,
    /// Binary path map for toolchains
    #[serde(skip, default)]
    binary_path_map: Arc<RwLock<HashMap<String, PathBuf>>>,
}

fn default_listen_addr() -> String {
    "0.0.0.0".into()
}

fn default_listen_port() -> u16 {
    9090
}

fn default_max_concurrent_requests() -> usize {
    6
}

fn default_problems_dir() -> PathBuf {
    "./assets/problems/".into()
}

fn default_log_dir() -> PathBuf {
    "./assets/logs/".into()
}

fn default_submissions_dir() -> PathBuf {
    "./assets/submissions/".into()
}

fn default_testlib_dir() -> PathBuf {
    "./assets/testlib/".into()
}

fn default_backend_host() -> String {
    "127.0.0.1".into()
}

fn default_backend_port() -> u16 {
    8080
}

fn default_backend_prefix() -> String {
    "".into()
}

fn default_save_submissions() -> bool {
    true
}

fn default_output_truncate_length() -> usize {
    200
}

fn default_toolchains() -> Vec<String> {
    vec![
        "gcc".into(),
        "g++".into(),
        "go".into(),
        "java".into(),
        "javac".into(),
        "python3".into(),
        "node".into(),
    ]
}

impl AijConfig {
    /// get global config
    #[allow(clippy::expect_used)]
    pub fn get() -> &'static AijConfig {
        CONFIG.get_or_init(|| {
            let settings = config::Config::builder()
                .add_source(
                    config::Environment::with_prefix("AIJ")
                        .try_parsing(true)
                        .list_separator(",")
                        .with_list_parse_key("toolchains"),
                )
                .build()
                .expect("Failed to build configuration from environment variables");
            let config: AijConfig = settings
                .try_deserialize()
                .expect("Failed to deserialize configuration from environment variables");
            config
        })
    }

    pub(crate) fn get_backend_base_addr() -> Result<String> {
        let config = Self::get();
        Ok(format!(
            "http://{}:{}{}",
            config.backend_host, config.backend_port, config.backend_prefix
        ))
    }

    pub(crate) async fn get_binary_path(bin_name: impl AsRef<str>) -> Result<PathBuf> {
        let config = Self::get();
        {
            let map = config.binary_path_map.read().await;
            if let Some(path) = map.get(bin_name.as_ref()) {
                return Ok(path.clone());
            }
        }

        warn!(
            "Binary '{}' not found in pre-initialized map, searching in PATH",
            bin_name.as_ref()
        );
        let bin_path = which::which(bin_name.as_ref());
        if let Ok(path) = bin_path {
            Self::update_binary_path_item(
                bin_name.as_ref(),
                path.clone(),
                &mut config.binary_path_map.write().await,
            );

            Ok(path.clone())
        } else {
            Err(AijError::Config(
                StatusCode::INTERNAL_SERVER_ERROR,
                "BINARY_NOT_FOUND".into(),
                format!("Binary '{}' not found in PATH", bin_name.as_ref()),
            ))
        }
    }

    pub(crate) async fn update_binary_path(&self) {
        let config = AijConfig::get();
        for bin in &config.toolchains {
            if let Ok(path) = which::which(bin) {
                Self::update_binary_path_item(bin, path, &mut self.binary_path_map.write().await);
            } else {
                error!("Binary '{}' specified in toolchains not found in PATH", bin);
            }
        }
    }

    fn update_binary_path_item(
        name: impl AsRef<str>,
        path: PathBuf,
        bin_map: &mut RwLockWriteGuard<'_, HashMap<String, PathBuf>>,
    ) {
        match bin_map.insert(name.as_ref().to_string(), path.clone()) {
            Some(existing_path) => {
                if existing_path != path {
                    warn!(
                        "Binary '{}' path updated from '{}' to '{}'",
                        name.as_ref(),
                        existing_path.to_string_lossy(),
                        path.to_string_lossy()
                    );
                }
            }
            None => {
                info!(
                    "Binary '{}' set at path: {}",
                    name.as_ref(),
                    path.to_string_lossy()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    /// Verify that a comma-separated AIJ_TOOLCHAINS value is correctly parsed
    /// into a Vec<String>, preserving backward compatibility with the documented
    /// env var format (e.g. `AIJ_TOOLCHAINS=gcc,g++,python3`).
    ///
    /// The `#[serial]` attribute ensures this test does not race with other
    /// tests that mutate environment variables.
    #[test]
    #[serial]
    fn test_toolchains_from_comma_separated_env() {
        // SAFETY: mutation is safe here because `#[serial]` ensures no other
        // test in this process is running concurrently.
        unsafe {
            std::env::set_var("AIJ_TOOLCHAINS", "gcc,g++,python3");
        }

        let settings = config::Config::builder()
            .add_source(
                config::Environment::with_prefix("AIJ")
                    .try_parsing(true)
                    .list_separator(",")
                    .with_list_parse_key("toolchains"),
            )
            .build()
            .unwrap();

        let toolchains: Vec<String> = settings.get("toolchains").unwrap();
        assert_eq!(toolchains, vec!["gcc", "g++", "python3"]);

        unsafe {
            std::env::remove_var("AIJ_TOOLCHAINS");
        }
    }
}
