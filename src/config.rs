//! Configuration management for the AIJ server.

use crate::error::{AijError, Result};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, RwLockWriteGuard};
use tracing::{error, info, warn};

static CONFIG: OnceLock<AijConfig> = OnceLock::new();

/// Configuration for the AIJ server
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Path of testlib directory
    pub testlib_dir: PathBuf,
    /// Host of the backend server
    pub backend_host: String,
    /// Port of the backend server
    pub backend_port: u16,
    /// Prefix for backend API endpoints
    pub backend_prefix: String,
    /// Save submission files
    pub save_submissions: bool,
    /// Truncate length of long outputs
    pub output_truncate_length: usize,
    /// List of available toolchains
    pub toolchains: Vec<String>,
    /// Binary path map for toolchains
    #[serde(skip)]
    binary_path_map: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl Default for AijConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".into(),
            listen_port: 9090,
            max_concurrent_requests: 6,
            problems_dir: "./assets/problems/".into(),
            log_dir: "./assets/logs/".into(),
            testlib_dir: "./assets/testlib/".into(),
            backend_host: "127.0.0.1".into(),
            backend_port: 8080,
            backend_prefix: "".into(),
            save_submissions: false,
            output_truncate_length: 200,
            toolchains: vec![
                "gcc".into(),
                "g++".into(),
                "go".into(),
                "java".into(),
                "javac".into(),
                "python3".into(),
                "node".into(),
            ],
            binary_path_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AijConfig {
    /// get global config
    pub fn get() -> &'static AijConfig {
        CONFIG.get_or_init(|| Self::default().update_from_env())
    }

    fn update_from_env(mut self) -> Self {
        if let Ok(val) = env::var("AIJ_LISTEN_ADDR") {
            self.listen_addr = val;
        }
        if let Some(val) = env::var("AIJ_LISTEN_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.listen_port = val;
        }
        if let Some(val) = env::var("AIJ_MAX_CONCURRENT_REQUESTS")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.max_concurrent_requests = val;
        }
        if let Ok(val) = env::var("AIJ_PROBLEMS_DIR") {
            self.problems_dir = PathBuf::from(val);
        }
        if let Ok(val) = env::var("AIJ_LOG_DIR") {
            self.log_dir = PathBuf::from(val);
        }
        if let Ok(val) = env::var("AIJ_TESTLIB_DIR") {
            self.testlib_dir = PathBuf::from(val);
        }
        if let Ok(val) = env::var("AIJ_BACKEND_HOST") {
            self.backend_host = val;
        }
        if let Some(val) = env::var("AIJ_BACKEND_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.backend_port = val;
        }
        if let Ok(val) = env::var("AIJ_BACKEND_PREFIX") {
            self.backend_prefix = val;
        }
        if let Some(val) = env::var("AIJ_SAVE_SUBMISSIONS")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.save_submissions = val;
        }
        if let Some(val) = env::var("AIJ_OUTPUT_TRUNCATE_LENGTH")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            self.output_truncate_length = val;
        }
        if let Ok(val) = env::var("AIJ_TOOLCHAINS") {
            self.toolchains = val
                .split(',')
                .filter_map(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    }
                })
                .collect();
        }
        self
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
