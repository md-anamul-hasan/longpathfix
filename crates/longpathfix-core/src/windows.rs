//! Native Windows OS integration via Win32 APIs (`windows-sys`).
//!
//! Handles:
//! - Registry inspection (`LongPathsEnabled` policy and Windows build number).
//! - Explicit UAC-elevated policy enabling (`ShellExecuteExW` with `runas` launching `--enable-long-paths`).
//! - Process manifest (`longPathAware`) and elevation inspection.
//! - Mounted volume compatibility inspection (`GetVolumeInformationW`, `GetVolumePathNameW`, `GetDriveTypeW`).
//! - Local timestamps (`GetLocalTime`) without pulling external datetime crates.
//! - Explorer integration (`open_folder`, `reveal_in_explorer`).

use crate::filesystem;
use crate::path;
use crate::{Error, Result};
use serde::Serialize;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_CANCELLED, HANDLE, MAX_PATH, SYSTEMTIME, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, GetVolumePathNameW,
};
use windows_sys::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegGetValueW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, ShellExecuteW, SHELLEXECUTEINFOW};
use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

// Constants not always exposed directly under every sub-module in windows-sys
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
const FILE_CASE_SENSITIVE_SEARCH_FLAG: u32 = 0x0000_0001;
const FILE_UNICODE_ON_DISK_FLAG: u32 = 0x0000_0004;
const FILE_SUPPORTS_REPARSE_POINTS_FLAG: u32 = 0x0000_0080;
const FILE_READ_ONLY_VOLUME_FLAG: u32 = 0x0008_0000;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const DRIVE_RAMDISK: u32 = 6;
const RT_MANIFEST_ID: usize = 24;

/// Converts a Rust string slice into a NUL-terminated UTF-16 buffer for Win32 `W` APIs.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Converts a NUL-terminated UTF-16 buffer slice to a `String`.
pub(crate) fn from_wide_nul(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Snapshot of Windows long-path policy, process manifest state, and OS version.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongPathPolicy {
    pub long_paths_enabled: bool,
    pub registry_value: Option<u32>,
    pub process_long_path_aware: bool,
    pub is_elevated: bool,
    pub os_build: u32,
    pub classic_max_path: usize,
    pub extended_max_path: usize,
}

impl LongPathPolicy {
    /// Inspects live HKLM registry state, process token elevation, and embedded application manifest.
    pub fn detect() -> Self {
        let registry_value = read_hklm_dword(
            r"SYSTEM\CurrentControlSet\Control\FileSystem",
            "LongPathsEnabled",
        );
        let os_build = read_hklm_string(
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "CurrentBuildNumber",
        )
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);

        Self {
            long_paths_enabled: registry_value == Some(1),
            registry_value,
            process_long_path_aware: detect_process_long_path_aware(),
            is_elevated: is_process_elevated(),
            os_build,
            classic_max_path: path::CLASSIC_LIMIT,
            extended_max_path: path::EXTENDED_LIMIT,
        }
    }
}

fn read_hklm_dword(subkey: &str, value: &str) -> Option<u32> {
    let subkey_w = to_wide(subkey);
    let value_w = to_wide(value);
    let mut data: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;
    // SAFETY: subkey_w and value_w are NUL-terminated UTF-16 buffers; `data` is sized for a DWORD.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey_w.as_ptr(),
            value_w.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut data as *mut u32).cast::<c_void>(),
            &mut size,
        )
    };
    if status == 0 {
        Some(data)
    } else {
        None
    }
}

fn read_hklm_string(subkey: &str, value: &str) -> Option<String> {
    let subkey_w = to_wide(subkey);
    let value_w = to_wide(value);
    let mut buf = vec![0u16; 128];
    let mut size: u32 = (buf.len() * std::mem::size_of::<u16>()) as u32;
    // SAFETY: subkey_w and value_w are NUL-terminated; `buf` is mutable and byte length matches `size`.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey_w.as_ptr(),
            value_w.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            buf.as_mut_ptr().cast::<c_void>(),
            &mut size,
        )
    };
    if status == 0 {
        Some(from_wide_nul(&buf))
    } else {
        None
    }
}

/// Checks whether the current executable's embedded PE manifest (`RT_MANIFEST`) declares `<longPathAware>true</longPathAware>`.
pub fn detect_process_long_path_aware() -> bool {
    // SAFETY: GetModuleHandleW(null()) gets the current process module; FindResourceW/LoadResource/LockResource
    // only read immutable bytes within the loaded executable image.
    unsafe {
        let hmod = GetModuleHandleW(null());
        if hmod.is_null() {
            return false;
        }
        let rt_manifest = RT_MANIFEST_ID as *const u16;
        for res_id in [1usize, 2usize] {
            let hrsrc = FindResourceW(hmod, res_id as *const u16, rt_manifest);
            if hrsrc.is_null() {
                continue;
            }
            let size = SizeofResource(hmod, hrsrc) as usize;
            let hglobal = LoadResource(hmod, hrsrc);
            if hglobal.is_null() || size == 0 {
                continue;
            }
            let ptr = LockResource(hglobal);
            if ptr.is_null() {
                continue;
            }
            let slice = std::slice::from_raw_parts(ptr.cast::<u8>(), size);
            let xml = String::from_utf8_lossy(slice);
            if xml.contains("<longPathAware>true</longPathAware>")
                || xml.contains("<ws2:longPathAware>true</ws2:longPathAware>")
            {
                return true;
            }
        }
        false
    }
}

/// Returns `true` if the current process token is elevated (Administrator privileges).
pub fn is_process_elevated() -> bool {
    let mut token: HANDLE = null_mut();
    // SAFETY: OpenProcessToken writes to `token`; GetTokenInformation reads into `TOKEN_ELEVATION`.
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast::<c_void>(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// Sets `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1` (`REG_DWORD`).
/// Requires administrator privileges (typically invoked via the `--enable-long-paths` UAC-elevated helper path).
pub fn enable_long_paths_in_registry() -> Result<()> {
    let subkey_w = to_wide(r"SYSTEM\CurrentControlSet\Control\FileSystem");
    let value_w = to_wide("LongPathsEnabled");
    let mut hkey: HKEY = null_mut();
    let val: u32 = 1;

    // SAFETY: subkey_w and value_w are NUL-terminated UTF-16 buffers; `val` is a stack DWORD.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey_w.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut hkey,
            null_mut(),
        )
    };
    if status != 0 {
        return Err(Error::io(
            "Could not open the Windows FileSystem policy registry key (administrator rights required)",
            std::io::Error::from_raw_os_error(status as i32),
        ));
    }

    // SAFETY: `hkey` was just opened by RegCreateKeyExW; `val` is 4 bytes.
    let write_status = unsafe {
        let s = RegSetValueExW(
            hkey,
            value_w.as_ptr(),
            0,
            REG_DWORD,
            (&val as *const u32).cast::<u8>(),
            std::mem::size_of::<u32>() as u32,
        );
        RegCloseKey(hkey);
        s
    };

    if write_status != 0 {
        return Err(Error::io(
            "Could not write LongPathsEnabled to the registry",
            std::io::Error::from_raw_os_error(write_status as i32),
        ));
    }
    Ok(())
}

/// Explicitly requests UAC elevation via `ShellExecuteExW` ("runas") to run the current executable with
/// `--enable-long-paths`. Returns `Ok(true)` if enabled, `Ok(false)` if the user cancelled UAC.
pub fn request_enable_long_paths_elevated() -> Result<bool> {
    if is_process_elevated() {
        enable_long_paths_in_registry()?;
        return Ok(true);
    }

    let exe = std::env::current_exe()
        .map_err(|e| Error::io("Could not determine the LongPathFix executable path", e))?;
    let exe_str = exe
        .to_str()
        .ok_or_else(|| Error::InvalidInput("Executable path is not valid Unicode.".into()))?;

    let verb_w = to_wide("runas");
    let file_w = to_wide(exe_str);
    let params_w = to_wide("--enable-long-paths");

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = verb_w.as_ptr();
    sei.lpFile = file_w.as_ptr();
    sei.lpParameters = params_w.as_ptr();
    sei.nShow = SW_HIDE;

    // SAFETY: `sei` fields point to NUL-terminated UTF-16 buffers that outlive the call;
    // any returned `hProcess` is waited on and closed before returning.
    unsafe {
        if ShellExecuteExW(&mut sei) == 0 {
            let err = GetLastError();
            if err == ERROR_CANCELLED {
                return Ok(false);
            }
            return Err(Error::io(
                "Could not start elevated helper to enable Windows long paths",
                std::io::Error::from_raw_os_error(err as i32),
            ));
        }

        if sei.hProcess.is_null() {
            return Err(Error::InvalidInput(
                "Windows did not return a process handle for the elevation helper.".into(),
            ));
        }

        let wait = WaitForSingleObject(sei.hProcess, 30_000);
        let mut exit_code: u32 = 1;
        let got_code = GetExitCodeProcess(sei.hProcess, &mut exit_code) != 0;
        CloseHandle(sei.hProcess);

        if wait != WAIT_OBJECT_0 || !got_code || exit_code != 0 {
            return Err(Error::InvalidInput(
                "The elevated helper could not update LongPathsEnabled in the registry.".into(),
            ));
        }
    }

    Ok(LongPathPolicy::detect().long_paths_enabled)
}

/// Filesystem and capability metadata for a Windows volume or drive.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub root_path: String,
    pub volume_label: String,
    pub filesystem_name: String,
    pub max_component_length: u32,
    pub supports_unicode_on_disk: bool,
    pub supports_reparse_points: bool,
    pub supports_case_sensitive_dirs: bool,
    pub is_read_only: bool,
    pub drive_type: String,
}

fn describe_drive_type(dt: u32) -> &'static str {
    match dt {
        DRIVE_FIXED => "Fixed drive",
        DRIVE_REMOVABLE => "Removable drive",
        DRIVE_REMOTE => "Network drive",
        DRIVE_CDROM => "Optical drive",
        DRIVE_RAMDISK => "RAM disk",
        _ => "Drive",
    }
}

/// Queries volume capabilities for the drive or UNC share containing `any_path`.
pub fn inspect_volume_for_path(any_path: &str) -> Option<VolumeInfo> {
    let plain = path::to_plain(any_path);
    let plain_w = to_wide(&plain);
    let mut vol_root = vec![0u16; MAX_PATH as usize + 64];

    // SAFETY: `plain_w` is NUL-terminated; `vol_root` receives the NUL-terminated volume mount point.
    let ok = unsafe {
        GetVolumePathNameW(
            plain_w.as_ptr(),
            vol_root.as_mut_ptr(),
            vol_root.len() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    let root_str = from_wide_nul(&vol_root);
    query_volume_root(&root_str)
}

fn query_volume_root(root_str: &str) -> Option<VolumeInfo> {
    let root_w = to_wide(root_str);
    let mut label_buf = [0u16; 260];
    let mut fs_buf = [0u16; 128];
    let mut max_comp: u32 = 0;
    let mut flags: u32 = 0;

    // SAFETY: `root_w` is NUL-terminated; all output buffers are sized with their lengths in characters.
    let ok = unsafe {
        GetVolumeInformationW(
            root_w.as_ptr(),
            label_buf.as_mut_ptr(),
            label_buf.len() as u32,
            null_mut(),
            &mut max_comp,
            &mut flags,
            fs_buf.as_mut_ptr(),
            fs_buf.len() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    // SAFETY: `root_w` is NUL-terminated.
    let dtype = unsafe { GetDriveTypeW(root_w.as_ptr()) };

    Some(VolumeInfo {
        root_path: root_str.to_string(),
        volume_label: from_wide_nul(&label_buf),
        filesystem_name: from_wide_nul(&fs_buf),
        max_component_length: max_comp,
        supports_unicode_on_disk: (flags & FILE_UNICODE_ON_DISK_FLAG) != 0,
        supports_reparse_points: (flags & FILE_SUPPORTS_REPARSE_POINTS_FLAG) != 0,
        supports_case_sensitive_dirs: (flags & FILE_CASE_SENSITIVE_SEARCH_FLAG) != 0,
        is_read_only: (flags & FILE_READ_ONLY_VOLUME_FLAG) != 0,
        drive_type: describe_drive_type(dtype).to_string(),
    })
}

/// Enumerates all mounted logical drive letters (`A:\` through `Z:\`) that return valid volume info.
pub fn list_logical_volumes() -> Vec<VolumeInfo> {
    // SAFETY: GetLogicalDrives takes no pointers and returns a 26-bit mask.
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for bit in 0..26u32 {
        if (mask & (1 << bit)) != 0 {
            let letter = (b'A' + bit as u8) as char;
            let root = format!("{letter}:\\");
            let dtype = unsafe { GetDriveTypeW(to_wide(&root).as_ptr()) };
            if matches!(
                dtype,
                DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_REMOTE | DRIVE_RAMDISK
            ) {
                if let Some(info) = query_volume_root(&root) {
                    out.push(info);
                }
            }
        }
    }
    out
}

/// Snapshot of local wall-clock time via Win32 `GetLocalTime`.
#[derive(Debug, Clone, Copy)]
pub struct LocalTimestamp {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub millis: u16,
}

impl LocalTimestamp {
    pub fn now() -> Self {
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        // SAFETY: `st` is a valid mutable `SYSTEMTIME` struct.
        unsafe { GetLocalTime(&mut st) };
        Self {
            year: st.wYear,
            month: st.wMonth,
            day: st.wDay,
            hour: st.wHour,
            minute: st.wMinute,
            second: st.wSecond,
            millis: st.wMilliseconds,
        }
    }

    pub fn batch_id(self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millis
        )
    }

    pub fn iso_local(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.millis
        )
    }

    pub fn csv_time(self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Opens an existing local folder in Windows Explorer (never executes arbitrary files or URLs).
pub fn open_folder(folder_path: &str) -> Result<()> {
    let normalized = path::normalize_absolute(folder_path)?;
    let ext = path::to_extended(&normalized)?;
    let meta = filesystem::metadata_extended(&ext)
        .ok_or_else(|| Error::NotFound(format!("Folder does not exist: {normalized}")))?;
    if !meta.is_dir {
        return Err(Error::InvalidInput(
            "Only folders can be opened directly.".into(),
        ));
    }

    let op_w = to_wide("open");
    let file_w = to_wide(&normalized);
    // SAFETY: `op_w` and `file_w` are NUL-terminated UTF-16 buffers; target was validated as an existing directory.
    let rc = unsafe {
        ShellExecuteW(
            null_mut(),
            op_w.as_ptr(),
            file_w.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if rc <= 32 {
        return Err(Error::InvalidInput(format!(
            "Could not open folder in Explorer (code {rc})."
        )));
    }
    Ok(())
}

/// Reveals an existing file or folder in Windows Explorer using `explorer.exe /select,"..."`.
pub fn reveal_in_explorer(target_path: &str) -> Result<()> {
    let normalized = path::normalize_absolute(target_path)?;
    let ext = path::to_extended(&normalized)?;
    if filesystem::metadata_extended(&ext).is_none() {
        return Err(Error::NotFound(format!(
            "Item no longer exists on disk: {normalized}"
        )));
    }

    let op_w = to_wide("open");
    let exe_w = to_wide("explorer.exe");
    let params_w = to_wide(&format!("/select,\"{normalized}\""));
    // SAFETY: All strings are NUL-terminated UTF-16 buffers; `explorer.exe` is invoked safely from system directory.
    let rc = unsafe {
        ShellExecuteW(
            null_mut(),
            op_w.as_ptr(),
            exe_w.as_ptr(),
            params_w.as_ptr(),
            null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if rc <= 32 {
        return Err(Error::InvalidInput(format!(
            "Could not open Windows Explorer (code {rc})."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_detection_returns_valid_build_and_limits() {
        let p = LongPathPolicy::detect();
        assert!(p.os_build > 0);
        assert_eq!(p.classic_max_path, 259);
        assert_eq!(p.extended_max_path, 32_767);
    }

    #[test]
    fn volume_query_finds_system_drive() {
        let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) + "\\";
        let info = inspect_volume_for_path(&sys_drive).expect("system drive volume info");
        assert!(!info.filesystem_name.is_empty());
        assert!(info.max_component_length >= 255);
        assert!(info.supports_unicode_on_disk);
    }

    #[test]
    fn local_timestamp_formats_consistently() {
        let ts = LocalTimestamp {
            year: 2026,
            month: 11,
            day: 14,
            hour: 9,
            minute: 5,
            second: 7,
            millis: 42,
        };
        assert_eq!(ts.batch_id(), "20261114-090507-042");
        assert_eq!(ts.iso_local(), "2026-11-14T09:05:07.042");
        assert_eq!(ts.csv_time(), "2026-11-14 09:05:07");
    }
}
