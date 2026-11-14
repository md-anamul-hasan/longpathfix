//! Bounded-parallel Win32 recursive scanner (`FindFirstFileExW` / `FindNextFileW`).
//!
//! Uses extended-length paths (`\\?\` / `\\?\UNC\`) exclusively so paths past 260 characters,
//! trailing dots/spaces, and reserved names are enumerated faithfully. Symbolic links, junctions,
//! and mount points (`FILE_ATTRIBUTE_REPARSE_POINT`) are recorded as nodes so they appear in
//! results, **never** descended into.

use crate::path::{utf16_len, Root};
use crate::security::validate_scan_root;
use crate::windows::{from_wide_nul, to_wide};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT,
    FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
};

/// A single file or folder discovered below the scan root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Relative path from the scanned root, separated by `\`, with no leading separator.
    pub rel: String,
    pub is_directory: bool,
    pub is_reparse_point: bool,
    pub is_readonly: bool,
    pub invalid_unicode: bool,
}

impl Node {
    pub fn new(rel: impl Into<String>, is_directory: bool) -> Self {
        Self {
            rel: rel.into(),
            is_directory,
            is_reparse_point: false,
            is_readonly: false,
            invalid_unicode: false,
        }
    }

    pub fn name(&self) -> &str {
        match self.rel.rfind('\\') {
            Some(pos) => &self.rel[pos + 1..],
            None => &self.rel,
        }
    }

    pub fn parent(&self) -> &str {
        match self.rel.rfind('\\') {
            Some(pos) => &self.rel[..pos],
            None => "",
        }
    }

    pub fn depth(&self) -> usize {
        self.rel.bytes().filter(|&b| b == b'\\').count()
    }
}

/// Live progress snapshot emitted during scanning.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub skipped_reparse_points: usize,
    pub inaccessible_dirs: usize,
    pub max_depth: usize,
    pub max_path_len: usize,
    pub current_dir: String,
}

/// Non-fatal problem encountered while scanning a directory (for example, access denied).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanWarning {
    pub path: String,
    pub message: String,
}

/// Complete result of a scan operation.
#[derive(Debug, Clone)]
pub struct ScanOutput {
    pub root: Root,
    pub nodes: Vec<Node>,
    pub warnings: Vec<ScanWarning>,
    pub skipped_reparse_points: usize,
    pub elapsed_ms: u64,
}

struct FindHandle(HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: `self.0` was obtained from FindFirstFileExW and is valid until FindClose.
            unsafe { FindClose(self.0) };
        }
    }
}

/// Scans `root_input` synchronously without cancellation or progress callbacks.
pub fn scan(root_input: &str) -> Result<Vec<Node>> {
    let cancel = AtomicBool::new(false);
    let out = scan_with_progress(root_input, &cancel, |_| {})?;
    Ok(out.nodes)
}

struct SharedQueue {
    dirs: VecDeque<String>,
    active_workers: usize,
    done: bool,
}

/// Scans `root_input` with bounded parallelism, cancellation, and throttled progress reporting.
pub fn scan_with_progress(
    root_input: &str,
    cancel: &AtomicBool,
    on_progress: impl Fn(ScanProgress) + Send + Sync,
) -> Result<ScanOutput> {
    let started = Instant::now();
    let root = validate_scan_root(root_input)?;

    let queue = Mutex::new(SharedQueue {
        dirs: VecDeque::from([String::new()]),
        active_workers: 0,
        done: false,
    });
    let cv = Condvar::new();

    let files_count = AtomicUsize::new(0);
    let dirs_count = AtomicUsize::new(0);
    let reparse_count = AtomicUsize::new(0);
    let denied_count = AtomicUsize::new(0);
    let max_depth = AtomicUsize::new(0);
    let max_path_len = AtomicUsize::new(root.len16);
    let last_emit_ms = AtomicUsize::new(0);

    let results: Mutex<Vec<Vec<Node>>> = Mutex::new(Vec::new());
    let warnings: Mutex<Vec<ScanWarning>> = Mutex::new(Vec::new());

    let workers = std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4);

    let base_ext = root.extended.trim_end_matches('\\').to_string();

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut local_nodes: Vec<Node> = Vec::with_capacity(256);
                let mut local_subdirs: Vec<String> = Vec::with_capacity(32);

                loop {
                    if cancel.load(Ordering::Relaxed) {
                        let mut q = queue.lock().unwrap();
                        q.done = true;
                        cv.notify_all();
                        break;
                    }

                    let dir_rel = {
                        let mut q = queue.lock().unwrap();
                        loop {
                            if cancel.load(Ordering::Relaxed) || q.done {
                                q.done = true;
                                cv.notify_all();
                                break None;
                            }
                            if let Some(d) = q.dirs.pop_front() {
                                q.active_workers += 1;
                                break Some(d);
                            }
                            if q.active_workers == 0 {
                                q.done = true;
                                cv.notify_all();
                                break None;
                            }
                            q = cv.wait(q).unwrap();
                        }
                    };

                    let Some(dir_rel) = dir_rel else {
                        break;
                    };

                    local_subdirs.clear();
                    scan_single_dir(
                        &base_ext,
                        root.len16,
                        &dir_rel,
                        cancel,
                        &mut local_nodes,
                        &mut local_subdirs,
                        &warnings,
                        &files_count,
                        &dirs_count,
                        &reparse_count,
                        &denied_count,
                        &max_depth,
                        &max_path_len,
                    );

                    // Push discovered subdirectories back into the shared queue and release active worker count.
                    {
                        let mut q = queue.lock().unwrap();
                        for sub in local_subdirs.drain(..) {
                            q.dirs.push_back(sub);
                        }
                        q.active_workers -= 1;
                        if q.dirs.is_empty() && q.active_workers == 0 {
                            q.done = true;
                        }
                        cv.notify_all();
                    }

                    if local_nodes.len() >= 2048 {
                        results
                            .lock()
                            .unwrap()
                            .push(std::mem::take(&mut local_nodes));
                    }

                    let now_ms = started.elapsed().as_millis() as usize;
                    let prev = last_emit_ms.load(Ordering::Relaxed);
                    if now_ms.saturating_sub(prev) >= 60
                        && last_emit_ms
                            .compare_exchange(prev, now_ms, Ordering::Relaxed, Ordering::Relaxed)
                            .is_ok()
                    {
                        on_progress(ScanProgress {
                            scanned_files: files_count.load(Ordering::Relaxed),
                            scanned_dirs: dirs_count.load(Ordering::Relaxed),
                            skipped_reparse_points: reparse_count.load(Ordering::Relaxed),
                            inaccessible_dirs: denied_count.load(Ordering::Relaxed),
                            max_depth: max_depth.load(Ordering::Relaxed),
                            max_path_len: max_path_len.load(Ordering::Relaxed),
                            current_dir: if dir_rel.is_empty() {
                                root.plain.clone()
                            } else {
                                root.join_plain(&dir_rel)
                            },
                        });
                    }
                }

                if !local_nodes.is_empty() {
                    results.lock().unwrap().push(local_nodes);
                }
            });
        }
    });

    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }

    let chunks = results.into_inner().unwrap();
    let total_nodes: usize = chunks.iter().map(Vec::len).sum();
    let mut nodes = Vec::with_capacity(total_nodes);
    for chunk in chunks {
        nodes.extend(chunk);
    }

    // Deterministic order: depth first, then ordinal relative path
    nodes.sort_by(|a, b| a.depth().cmp(&b.depth()).then_with(|| a.rel.cmp(&b.rel)));

    // Emit final 100% progress state
    on_progress(ScanProgress {
        scanned_files: files_count.load(Ordering::Relaxed),
        scanned_dirs: dirs_count.load(Ordering::Relaxed),
        skipped_reparse_points: reparse_count.load(Ordering::Relaxed),
        inaccessible_dirs: denied_count.load(Ordering::Relaxed),
        max_depth: max_depth.load(Ordering::Relaxed),
        max_path_len: max_path_len.load(Ordering::Relaxed),
        current_dir: root.plain.clone(),
    });

    Ok(ScanOutput {
        root,
        nodes,
        warnings: warnings.into_inner().unwrap(),
        skipped_reparse_points: reparse_count.load(Ordering::Relaxed),
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[allow(clippy::too_many_arguments)]
fn scan_single_dir(
    base_ext: &str,
    root_len16: usize,
    dir_rel: &str,
    cancel: &AtomicBool,
    local_nodes: &mut Vec<Node>,
    local_subdirs: &mut Vec<String>,
    warnings: &Mutex<Vec<ScanWarning>>,
    files_count: &AtomicUsize,
    dirs_count: &AtomicUsize,
    reparse_count: &AtomicUsize,
    denied_count: &AtomicUsize,
    max_depth: &AtomicUsize,
    max_path_len: &AtomicUsize,
) {
    let pattern = if dir_rel.is_empty() {
        format!("{base_ext}\\*")
    } else {
        format!("{base_ext}\\{dir_rel}\\*")
    };
    let pattern_w = to_wide(&pattern);
    let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };

    // SAFETY: `pattern_w` is NUL-terminated UTF-16; `find_data` receives directory entry metadata.
    let raw_handle = unsafe {
        FindFirstFileExW(
            pattern_w.as_ptr(),
            FindExInfoBasic,
            (&mut find_data as *mut WIN32_FIND_DATAW).cast::<c_void>(),
            FindExSearchNameMatch,
            std::ptr::null(),
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };

    if raw_handle == INVALID_HANDLE_VALUE {
        let err = unsafe { GetLastError() };
        if err != ERROR_FILE_NOT_FOUND && err != ERROR_NO_MORE_FILES {
            if err == ERROR_ACCESS_DENIED {
                denied_count.fetch_add(1, Ordering::Relaxed);
            }
            let display_dir = if dir_rel.is_empty() {
                crate::path::to_plain(base_ext)
            } else {
                format!("{}\\{dir_rel}", crate::path::to_plain(base_ext))
            };
            warnings.lock().unwrap().push(ScanWarning {
                path: display_dir,
                message: crate::filesystem::describe_io_error(&std::io::Error::from_raw_os_error(
                    err as i32,
                )),
            });
        }
        return;
    }

    let handle = FindHandle(raw_handle);
    let mut iter_count = 0usize;
    let dir_depth = if dir_rel.is_empty() {
        0
    } else {
        dir_rel.bytes().filter(|&b| b == b'\\').count() + 1
    };

    loop {
        if iter_count & 255 == 0 && cancel.load(Ordering::Relaxed) {
            break;
        }
        iter_count += 1;

        let name_len = find_data
            .cFileName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(find_data.cFileName.len());
        let wide_name = &find_data.cFileName[..name_len];

        // Skip "." and ".." quickly in UTF-16
        let is_dot = wide_name == [b'.' as u16] || wide_name == [b'.' as u16, b'.' as u16];
        if !is_dot && !wide_name.is_empty() {
            let (name, invalid_unicode) = match String::from_utf16(wide_name) {
                Ok(s) => (s, false),
                Err(_) => (from_wide_nul(wide_name), true),
            };
            let attrs = find_data.dwFileAttributes;
            let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0;
            let is_reparse = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
            let is_readonly = (attrs & FILE_ATTRIBUTE_READONLY) != 0;

            let rel = if dir_rel.is_empty() {
                name
            } else {
                format!("{dir_rel}\\{name}")
            };

            let full_len16 = root_len16 + 1 + utf16_len(&rel);
            max_depth.fetch_max(dir_depth, Ordering::Relaxed);
            max_path_len.fetch_max(full_len16, Ordering::Relaxed);

            if is_dir {
                dirs_count.fetch_add(1, Ordering::Relaxed);
                if is_reparse {
                    reparse_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    local_subdirs.push(rel.clone());
                }
            } else {
                files_count.fetch_add(1, Ordering::Relaxed);
                if is_reparse {
                    reparse_count.fetch_add(1, Ordering::Relaxed);
                }
            }

            local_nodes.push(Node {
                rel,
                is_directory: is_dir,
                is_reparse_point: is_reparse,
                is_readonly,
                invalid_unicode,
            });
        }

        // SAFETY: `handle.0` is an open find handle; `find_data` is a valid buffer.
        if unsafe { FindNextFileW(handle.0, &mut find_data) } == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::to_extended;

    #[test]
    fn scans_long_paths_trailing_dots_and_skips_junctions() {
        let base = std::env::temp_dir().join(format!(
            "lpf-scan-{}",
            crate::windows::LocalTimestamp::now().batch_id()
        ));
        let root_str = base.to_string_lossy().into_owned();
        let outside = std::env::temp_dir().join(format!(
            "lpf-scan-out-{}",
            crate::windows::LocalTimestamp::now().batch_id()
        ));

        let root_ext = to_extended(&root_str).unwrap();
        std::fs::create_dir_all(&root_ext).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        std::fs::write(format!("{root_ext}\\trail."), b"a").unwrap();
        std::fs::write(format!("{root_ext}\\CON.txt"), b"b").unwrap();
        std::fs::write(outside.join("outside_bad."), b"x").unwrap();

        // Deep chain > 320 chars
        let deep = format!(
            "{root_ext}\\{}\\{}\\deep.bin",
            "d".repeat(120),
            "e".repeat(120)
        );
        std::fs::create_dir_all(deep.rsplit_once('\\').unwrap().0).unwrap();
        std::fs::write(&deep, b"deep").unwrap();

        // Create a junction pointing outside
        let junction = base.join("jump");
        let status = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &junction.to_string_lossy(),
                &outside.to_string_lossy(),
            ])
            .output();

        let nodes = scan(&root_str).unwrap();
        assert!(nodes.iter().any(|n| n.rel == "trail."));
        assert!(nodes.iter().any(|n| n.rel == "CON.txt"));
        assert!(nodes.iter().any(|n| n.rel.ends_with("deep.bin")));

        if let Ok(out) = status {
            if out.status.success() {
                let jump = nodes
                    .iter()
                    .find(|n| n.rel == "jump")
                    .expect("junction listed");
                assert!(jump.is_reparse_point);
                assert!(!nodes.iter().any(|n| n.rel.starts_with("jump\\")));
            }
        }

        let _ = std::fs::remove_dir_all(&root_ext);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn cancellation_stops_scan_promptly() {
        let cancel = AtomicBool::new(true);
        let sys_drive = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let res = scan_with_progress(&sys_drive, &cancel, |_| {});
        assert!(matches!(res, Err(Error::Cancelled)));
    }
}
