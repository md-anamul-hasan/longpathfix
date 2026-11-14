//! Low-level Win32 extended-path filesystem operations.
//!
//! All disk queries and renames pass `\\?\` or `\\?\UNC\` paths directly to Win32 `W` APIs so
//! over-long paths (> 260 UTF-16 units), trailing dots/spaces, and reserved names work reliably.

use crate::windows::to_wide;
use crate::{Error, Result};
use serde::Serialize;
use std::ffi::c_void;
use std::io::Write;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileAttributesExW, GetFileAttributesW, GetFileExInfoStandard, MoveFileExW,
    SetFileAttributesW, DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SYSTEM,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, INVALID_FILE_ATTRIBUTES, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    OPEN_EXISTING, WIN32_FILE_ATTRIBUTE_DATA,
};

/// Metadata read via `GetFileAttributesExW` on an extended-length path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMeta {
    pub exists: bool,
    pub is_dir: bool,
    pub is_readonly: bool,
    pub is_reparse_point: bool,
    pub is_system: bool,
    pub is_hidden: bool,
    pub attributes: u32,
    pub size: u64,
}

/// Queries metadata using `GetFileAttributesExW`, avoiding Windows path normalization.
/// Returns `None` if the path does not exist or cannot be queried.
pub fn metadata_extended(ext_path: &str) -> Option<EntryMeta> {
    let w = to_wide(ext_path);
    let mut data: WIN32_FILE_ATTRIBUTE_DATA = unsafe { std::mem::zeroed() };
    // SAFETY: `w` is a NUL-terminated UTF-16 buffer; `data` receives `WIN32_FILE_ATTRIBUTE_DATA`.
    let ok = unsafe {
        GetFileAttributesExW(
            w.as_ptr(),
            GetFileExInfoStandard,
            (&mut data as *mut WIN32_FILE_ATTRIBUTE_DATA).cast::<c_void>(),
        )
    };
    if ok == 0 {
        return None;
    }
    let attrs = data.dwFileAttributes;
    let size = ((data.nFileSizeHigh as u64) << 32) | (data.nFileSizeLow as u64);
    Some(EntryMeta {
        exists: true,
        is_dir: (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0,
        is_readonly: (attrs & FILE_ATTRIBUTE_READONLY) != 0,
        is_reparse_point: (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0,
        is_system: (attrs & FILE_ATTRIBUTE_SYSTEM) != 0,
        is_hidden: (attrs & FILE_ATTRIBUTE_HIDDEN) != 0,
        attributes: attrs,
        size,
    })
}

/// Checks if a file is currently locked against renaming by another process (`ERROR_SHARING_VIOLATION` / `ERROR_LOCK_VIOLATION`).
/// Directories return `false` unless locked; non-existent files return `false`.
pub fn is_locked_for_rename(ext_path: &str, is_dir: bool) -> bool {
    if is_dir {
        return false;
    }
    let w = to_wide(ext_path);
    // SAFETY: `w` is NUL-terminated UTF-16; handle is closed immediately on success.
    let handle = unsafe {
        CreateFileW(
            w.as_ptr(),
            DELETE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let err = unsafe { GetLastError() };
        return err == ERROR_SHARING_VIOLATION || err == ERROR_LOCK_VIOLATION;
    }
    // SAFETY: `handle` is valid and non-INVALID_HANDLE_VALUE.
    unsafe { CloseHandle(handle) };
    false
}

/// Sets or clears `FILE_ATTRIBUTE_READONLY` on an extended-length path.
pub fn set_readonly(ext_path: &str, readonly: bool) -> std::io::Result<()> {
    let w = to_wide(ext_path);
    // SAFETY: `w` is NUL-terminated UTF-16.
    let attrs = unsafe { GetFileAttributesW(w.as_ptr()) };
    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(std::io::Error::last_os_error());
    }
    let next = if readonly {
        attrs | FILE_ATTRIBUTE_READONLY
    } else {
        attrs & !FILE_ATTRIBUTE_READONLY
    };
    if next == attrs {
        return Ok(());
    }
    // SAFETY: `w` is NUL-terminated UTF-16.
    if unsafe { SetFileAttributesW(w.as_ptr(), next) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Renames `from_ext` to `to_ext` in the same folder using `MoveFileExW(..., 0)` (kernel-enforced no-overwrite).
/// Verifies that `to_ext` exists after the move before returning success.
pub fn rename_no_overwrite(from_ext: &str, to_ext: &str) -> std::io::Result<()> {
    let from_w = to_wide(from_ext);
    let to_w = to_wide(to_ext);
    // SAFETY: both buffers are NUL-terminated UTF-16 strings; flags = 0 forbids overwriting an existing target.
    if unsafe { MoveFileExW(from_w.as_ptr(), to_w.as_ptr(), 0) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    if metadata_extended(to_ext).is_none() {
        return Err(std::io::Error::other(
            "Windows reported rename success, but the new name could not be verified on disk",
        ));
    }
    Ok(())
}

/// Atomically writes `content` to `path` via `<path>.tmp`, `sync_all`, and `MoveFileExW` replacement.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::io(format!("Could not create folder {}", parent.display()), e))?;
    }
    let tmp = path.with_extension(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}.tmp"))
            .unwrap_or_else(|| "tmp".to_string()),
    );
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| {
            Error::io(
                format!("Could not write temporary file {}", tmp.display()),
                e,
            )
        })?;
        f.write_all(content)
            .map_err(|e| Error::io(format!("Could not write {}", tmp.display()), e))?;
        f.sync_all()
            .map_err(|e| Error::io(format!("Could not flush {}", tmp.display()), e))?;
    }

    let tmp_str = tmp
        .to_str()
        .ok_or_else(|| Error::Data("Temporary file path is not valid Unicode.".into()))?;
    let dst_str = path
        .to_str()
        .ok_or_else(|| Error::Data("Target file path is not valid Unicode.".into()))?;
    let tmp_ext = crate::path::to_extended(tmp_str).unwrap_or_else(|_| tmp_str.to_string());
    let dst_ext = crate::path::to_extended(dst_str).unwrap_or_else(|_| dst_str.to_string());

    let from_w = to_wide(&tmp_ext);
    let to_w = to_wide(&dst_ext);
    // SAFETY: both buffers are NUL-terminated UTF-16 strings; replaces target atomically and flushes metadata.
    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        let err = std::io::Error::last_os_error();
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::io(format!("Could not save {}", path.display()), err));
    }
    Ok(())
}

/// Converts raw Win32 I/O errors into clear plain-English explanations.
pub fn describe_io_error(err: &std::io::Error) -> String {
    match err.raw_os_error() {
        Some(2) => "The file or folder no longer exists.".to_string(),
        Some(3) => "Part of the folder path could not be found.".to_string(),
        Some(5) => "Access is denied (check permissions or read-only protection).".to_string(),
        Some(32) | Some(33) => "The file is open or locked by another program.".to_string(),
        Some(80) | Some(183) => "An item with that name already exists.".to_string(),
        Some(112) => "There is not enough space on the disk.".to_string(),
        Some(123) => {
            "The name or path syntax is rejected by this drive or file system.".to_string()
        }
        Some(145) => "The folder is not empty.".to_string(),
        Some(206) => "The path exceeds the system limit for this operation.".to_string(),
        _ => err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::to_extended;

    #[test]
    fn extended_metadata_and_no_overwrite_rename_work_on_trailing_dot_and_reserved() {
        let dir = std::env::temp_dir().join(format!(
            "lpf-fs-test-{}",
            crate::windows::LocalTimestamp::now().batch_id()
        ));
        let dir_str = dir.to_string_lossy().into_owned();
        let dir_ext = to_extended(&dir_str).unwrap();
        std::fs::create_dir_all(&dir_ext).unwrap();

        let src = format!("{dir_ext}\\trail.");
        let dst = format!("{dir_ext}\\trail");
        std::fs::write(&src, b"hello").unwrap();

        let meta = metadata_extended(&src).expect("trailing dot exists");
        assert!(meta.exists && !meta.is_dir && meta.size == 5);
        assert!(!is_locked_for_rename(&src, false));

        rename_no_overwrite(&src, &dst).unwrap();
        assert!(metadata_extended(&src).is_none());
        assert!(metadata_extended(&dst).is_some());

        // Second file cannot overwrite dst
        let src2 = format!("{dir_ext}\\other.");
        std::fs::write(&src2, b"world").unwrap();
        assert!(rename_no_overwrite(&src2, &dst).is_err());

        let _ = std::fs::remove_dir_all(&dir_ext);
    }

    #[test]
    fn atomic_write_replaces_target_cleanly() {
        let dir = std::env::temp_dir().join(format!(
            "lpf-aw-test-{}",
            crate::windows::LocalTimestamp::now().batch_id()
        ));
        let file = dir.join("config.json");
        atomic_write(&file, br#"{"a":1}"#).unwrap();
        atomic_write(&file, br#"{"a":2}"#).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), r#"{"a":2}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
