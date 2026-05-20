use std::path::Path;

use crate::config::AijConfig;
use crate::error::Result;
use crate::judger::SupportedLanguages;
use axum::Json;
use serde::Serialize;
use serde_json::json;
use strum::IntoEnumIterator;
use tracing::warn;

#[derive(Serialize)]
struct LanguageInfo {
    name: String,
    available: bool,
    version: Option<String>,
}

async fn get_binary_version(bin_path: impl AsRef<Path>, version_arg: &str) -> Option<String> {
    match tokio::process::Command::new(bin_path.as_ref())
        .arg(version_arg)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version_text = if stdout.trim().is_empty() {
                stderr.trim().to_string()
            } else {
                stdout.trim().to_string()
            };
            let first_line = version_text.lines().next().unwrap_or("").to_string();
            if first_line.is_empty() {
                None
            } else {
                Some(first_line)
            }
        }
        Err(e) => {
            warn!(
                "Failed to get version for '{}': {}",
                bin_path.as_ref().display(),
                e
            );
            None
        }
    }
}

pub(crate) async fn get_languages() -> Result<Json<serde_json::Value>> {
    let mut languages = Vec::new();

    for lang in SupportedLanguages::iter() {
        let (bin_name, version_arg) = match lang {
            SupportedLanguages::C => ("gcc", "--version"),
            SupportedLanguages::Cpp => ("g++", "--version"),
            SupportedLanguages::Cpp20 => ("g++", "--version"),
            SupportedLanguages::Python => ("python3", "--version"),
            SupportedLanguages::Nodejs => ("node", "--version"),
            SupportedLanguages::Go => ("go", "version"),
            SupportedLanguages::Java => ("java", "-version"),
        };
        if let Ok(bin_path) = AijConfig::get_binary_path(bin_name).await
            && let Some(version) = get_binary_version(&bin_path, version_arg).await
        {
            languages.push(LanguageInfo {
                name: lang.to_string(),
                available: true,
                version: Some(version),
            });
        } else {
            languages.push(LanguageInfo {
                name: lang.to_string(),
                available: false,
                version: None,
            });
        }
    }

    Ok(Json(json!({
        "code": 0,
        "message": "Success",
        "data": {
            "languages": languages
        },
    })))
}
