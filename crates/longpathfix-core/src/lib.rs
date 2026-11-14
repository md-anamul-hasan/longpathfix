//! LongPathFix engine.
//!
//! Everything that decides or changes something lives here; the Tauri shell (`longpathfix` crate) only
//! moves data between this crate and the window. The modules, from the bottom up:
//!
//! - [`errors`]: one error type for the whole engine.
//! - [`path`]: parsing, classifying and normalising Windows paths, `\\?\` and `\\?\UNC\` forms, UTF-16 lengths.
//! - [`rules`]: what is wrong with a single name and how to clean it.
//! - [`windows`]: the Win32 calls (registry, volume information, elevation, shell, local time).
//! - [`filesystem`]: file-system primitives: attributes, no-overwrite rename, atomic writes.
//! - [`security`]: input validation and the safety checks every rename passes.
//! - [`scanner`]: bounded-parallel recursive scan into a compact tree.
//! - [`planner`]: pure logic that turns a tree into findings and a rename plan.
//! - [`repair`]: dry run, apply and undo, each rename verified.
//! - [`storage`]: the rename history (compatible with 1.x files) and the CSV rename log.
//! - [`config`]: user settings.
//! - [`logging`]: a small structured (JSON lines) log with rotation.
//! - [`export`]: CSV export of findings and results.

#[cfg(not(windows))]
compile_error!("LongPathFix is a Windows application.");

pub mod config;
pub mod errors;
pub mod export;
pub mod filesystem;
pub mod logging;
pub mod path;
pub mod planner;
pub mod repair;
pub mod rules;
pub mod scanner;
pub mod security;
pub mod storage;
pub mod windows;

pub use errors::{Error, Result};

/// Product version, from the workspace `Cargo.toml` (the single source of the version number).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where LongPathFix keeps its own files: `%APPDATA%\LongPathFix` (the same folder 1.x used).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data: std::path::PathBuf,
}

impl AppPaths {
    pub fn from_env() -> Result<Self> {
        let appdata = std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| Error::InvalidInput("The APPDATA folder is not set.".into()))?;
        Ok(Self {
            data: appdata.join("LongPathFix"),
        })
    }

    pub fn with_root(root: impl Into<std::path::PathBuf>) -> Self {
        Self { data: root.into() }
    }

    pub fn history(&self) -> std::path::PathBuf {
        self.data.join("history")
    }

    pub fn logs(&self) -> std::path::PathBuf {
        self.data.join("logs")
    }

    pub fn settings(&self) -> std::path::PathBuf {
        self.data.join("settings.json")
    }

    pub fn rename_log(&self) -> std::path::PathBuf {
        self.data.join("rename-log.csv")
    }
}
