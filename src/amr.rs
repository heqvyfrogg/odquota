//! Import credentials from the Open Design / Vela CLI config.
//!
//! The file at `~/.amr/config.json` is read, never written. Only the fields we
//! need are extracted; the `user` block (email, avatar) is deliberately not
//! copied into our store.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;
use crate::store::AccountEntry;

pub const DEFAULT_PROFILE: &str = "prod";
pub const DEFAULT_API_URL: &str = "https://amr-api.open-design.ai";
pub const DEFAULT_LINK_URL: &str = "https://amr-link.open-design.ai";

#[derive(Debug, Deserialize)]
struct AmrConfig {
    #[serde(default)]
    profiles: std::collections::BTreeMap<String, AmrProfile>,
}

#[derive(Debug, Deserialize)]
struct AmrProfile {
    #[serde(rename = "controlKey")]
    control_key: Option<String>,
    #[serde(rename = "runtimeKey")]
    runtime_key: Option<String>,
    #[serde(rename = "apiUrl")]
    api_url: Option<String>,
    #[serde(rename = "linkUrl")]
    link_url: Option<String>,
}

impl AmrProfile {
    /// Trim away whitespace and treat an empty string as absent so a half-written
    /// config cannot produce an empty bearer token that fails later with 401.
    fn clean(value: &Option<String>) -> Option<String> {
        value
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }
}

/// Locate `~/.amr/config.json` without consulting platform-specific registry data.
pub fn default_config_path() -> Result<PathBuf, AppError> {
    let home = std::env::var_os("USERPROFILE")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("HOME").filter(|v| !v.is_empty()))
        .ok_or_else(|| {
            AppError::store("ホームディレクトリを解決できません（.amr を探せません）")
        })?;
    Ok(PathBuf::from(home).join(".amr").join("config.json"))
}

/// Read the profile and convert it into a store entry.
pub fn read_profile(
    path: &Path,
    profile: &str,
    label: &str,
    workspace_id: Option<String>,
) -> Result<AccountEntry, AppError> {
    let raw = std::fs::read(path)
        .map_err(|e| AppError::store(format!("設定を読めません {}: {e}", path.display())))?;
    let text = String::from_utf8(raw).map_err(|e| {
        AppError::store(format!(
            "設定が UTF-8 ではありません {}: {e}",
            path.display()
        ))
    })?;
    let cfg: AmrConfig = serde_json::from_str(&text).map_err(|e| {
        AppError::store(format!(
            "設定の JSON を解釈できません {}: {e}",
            path.display()
        ))
    })?;

    let prof = cfg.profiles.get(profile).ok_or_else(|| {
        let known: Vec<&str> = cfg.profiles.keys().map(String::as_str).collect();
        AppError::usage(format!(
            "プロファイル \"{profile}\" が設定にありません（存在する: {}）",
            if known.is_empty() {
                "なし".to_string()
            } else {
                known.join(", ")
            }
        ))
    })?;

    let control_key = AmrProfile::clean(&prof.control_key).ok_or_else(|| {
        AppError::usage(format!(
            "プロファイル \"{profile}\" に controlKey がありません（再ログインが必要です）"
        ))
    })?;

    let api_url = AmrProfile::clean(&prof.api_url).unwrap_or_else(|| DEFAULT_API_URL.to_string());
    let link_url =
        AmrProfile::clean(&prof.link_url).unwrap_or_else(|| DEFAULT_LINK_URL.to_string());

    Ok(AccountEntry {
        label: label.to_string(),
        control_key,
        runtime_key: AmrProfile::clean(&prof.runtime_key),
        api_url,
        link_url,
        workspace_id: workspace_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        api_url_source: "amr-config".to_string(),
        added_at: chrono::Utc::now().to_rfc3339(),
        source: format!("amr-config:{profile}"),
    })
}
