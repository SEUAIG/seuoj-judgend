//! Configuration management for the AIJ server.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

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
}

impl Default for AijConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".into(),
            listen_port: 9090,
            max_concurrent_requests: 6,
            problems_dir: "./assets/problems/".into(),
            log_dir: "./assets/logs/".into(),
            backend_host: "127.0.0.1".into(),
            backend_port: 8080,
            backend_prefix: "".into(),
            save_submissions: false,
            output_truncate_length: 200,
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
        self
    }

    pub(crate) async fn get_backend_base_addr() -> Result<String> {
        let config = Self::get();
        Ok(format!(
            "http://{}:{}{}",
            config.backend_host, config.backend_port, config.backend_prefix
        ))
    }
}
