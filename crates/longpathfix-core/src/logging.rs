//! Lightweight structured JSON-lines logger with size-based rotation and legacy `error.log` fallback.

use crate::windows::LocalTimestamp;
use crate::AppPaths;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Serialize)]
struct LogEntry<'a> {
    time: &'a str,
    level: LogLevel,
    area: &'a str,
    message: &'a str,
}

pub struct Logger {
    structured_path: PathBuf,
    error_log_path: PathBuf,
    lock: Mutex<()>,
}

impl Logger {
    pub fn from_app_paths(paths: &AppPaths) -> Self {
        Self {
            structured_path: paths.logs().join("longpathfix.log"),
            error_log_path: paths.data.join("error.log"),
            lock: Mutex::new(()),
        }
    }

    pub fn info(&self, area: &str, message: &str) {
        self.write(LogLevel::Info, area, message);
    }

    pub fn warn(&self, area: &str, message: &str) {
        self.write(LogLevel::Warn, area, message);
    }

    pub fn error(&self, area: &str, message: &str) {
        self.write(LogLevel::Error, area, message);
    }

    fn write(&self, level: LogLevel, area: &str, message: &str) {
        let Ok(_guard) = self.lock.lock() else {
            return;
        };
        let ts = LocalTimestamp::now().iso_local();

        if let Some(parent) = self.structured_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        rotate_if_needed(&self.structured_path);

        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.structured_path)
        {
            if let Ok(mut line) = serde_json::to_vec(&LogEntry {
                time: &ts,
                level,
                area,
                message,
            }) {
                line.push(b'\n');
                let _ = f.write_all(&line);
            }
        }

        if matches!(level, LogLevel::Error) {
            if let Some(parent) = self.error_log_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.error_log_path)
            {
                let _ = writeln!(f, "[{ts}] [{area}] {message}");
            }
        }
    }
}

fn rotate_if_needed(path: &std::path::Path) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if meta.len() < MAX_LOG_BYTES {
        return;
    }
    let bak1 = path.with_extension("log.1");
    let bak2 = path.with_extension("log.2");
    let _ = std::fs::remove_file(&bak2);
    let _ = std::fs::rename(&bak1, &bak2);
    let _ = std::fs::rename(path, &bak1);
}
