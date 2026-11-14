//! User configuration stored in `%APPDATA%\LongPathFix\settings.json`.

use crate::filesystem::atomic_write;
use crate::rules::FixKinds;
use crate::{AppPaths, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_RECENT_ROOTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default = "default_limit")]
    pub default_limit: usize,
    #[serde(default = "default_custom_limit")]
    pub custom_limit: usize,
    #[serde(default = "default_fix_kinds")]
    pub default_fix_kinds: FixKinds,
    #[serde(default = "default_true")]
    pub clear_readonly_when_authorized: bool,
    #[serde(default)]
    pub recent_roots: Vec<String>,
}

const fn default_limit() -> usize {
    259
}

const fn default_custom_limit() -> usize {
    220
}

const fn default_fix_kinds() -> FixKinds {
    FixKinds::ALL
}

const fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            default_limit: default_limit(),
            custom_limit: default_custom_limit(),
            default_fix_kinds: FixKinds::ALL,
            clear_readonly_when_authorized: true,
            recent_roots: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn sanitized(mut self) -> Self {
        self.default_limit = self.default_limit.clamp(40, 1000);
        self.custom_limit = self.custom_limit.clamp(40, 1000);
        let mut deduped: Vec<String> = Vec::with_capacity(MAX_RECENT_ROOTS);
        for r in self.recent_roots {
            let trimmed = r.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !deduped
                .iter()
                .any(|existing| crate::rules::eq_ignore_case(existing, trimmed))
            {
                deduped.push(trimmed.to_string());
                if deduped.len() == MAX_RECENT_ROOTS {
                    break;
                }
            }
        }
        self.recent_roots = deduped;
        self
    }

    pub fn push_recent_root(&mut self, root_plain: &str) {
        self.recent_roots
            .retain(|existing| !crate::rules::eq_ignore_case(existing, root_plain));
        self.recent_roots.insert(0, root_plain.to_string());
        self.recent_roots.truncate(MAX_RECENT_ROOTS);
    }

    pub fn load_from(path: &std::path::Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice::<Self>(&bytes)
            .map(Self::sanitized)
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
        let clean = self.clone().sanitized();
        let bytes = serde_json::to_vec_pretty(&clean)?;
        atomic_write(path, &bytes)
    }

    pub fn load(paths: &AppPaths) -> Self {
        Self::load_from(&paths.settings())
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        self.save_to(&paths.settings())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    pub path: PathBuf,
}

impl ConfigStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            path: paths.settings(),
        }
    }

    pub fn load(&self) -> AppConfig {
        AppConfig::load_from(&self.path)
    }

    pub fn save(&self, cfg: &AppConfig) -> Result<()> {
        cfg.save_to(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_sanitizes_limits_and_recent_roots() {
        let dir = std::env::temp_dir().join(format!(
            "lpf-cfg-{}",
            crate::windows::LocalTimestamp::now().batch_id()
        ));
        let path = dir.join("settings.json");

        let mut cfg = AppConfig {
            theme: ThemePreference::Dark,
            default_limit: 5,   // clamped to 40
            custom_limit: 9999, // clamped to 1000
            default_fix_kinds: FixKinds::ALL,
            clear_readonly_when_authorized: false,
            recent_roots: vec![r"C:\a".into(), r"c:\A".into(), r"D:\work".into()],
        };
        cfg.push_recent_root(r"E:\photos");
        cfg.save_to(&path).unwrap();

        let loaded = AppConfig::load_from(&path);
        assert_eq!(loaded.theme, ThemePreference::Dark);
        assert_eq!(loaded.default_limit, 40);
        assert_eq!(loaded.custom_limit, 1000);
        assert_eq!(loaded.recent_roots, vec![r"E:\photos", r"C:\a", r"D:\work"]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
