//! Security controls and boundary checks for scanning and renaming.
//!
//! Ensures every path passed from the UI or stored history is normalized lexically,
//! stays strictly inside the chosen scan root, and never traverses symlink/junction swaps.

use crate::filesystem;
use crate::path::{is_within, normalize_absolute, to_extended, utf16_len, Root};
use crate::rules::MAX_NAME_LENGTH;
use crate::{Error, Result};

/// Normalizes `input` and confirms it points to an existing folder on disk.
pub fn validate_scan_root(input: &str) -> Result<Root> {
    let normalized = normalize_absolute(input)?;
    let root = Root::new(&normalized)?;
    let meta = filesystem::metadata_extended(&root.extended)
        .ok_or_else(|| Error::NotFound(format!("The folder does not exist: {}", root.plain)))?;
    if !meta.is_dir {
        return Err(Error::InvalidInput(format!(
            "Choose a folder, not a file: {}",
            root.plain
        )));
    }
    Ok(root)
}

/// Returns a warning if `plain_root` is a drive root or an OS/application system directory.
pub fn protected_path_warning(plain_root: &str) -> Option<String> {
    let trimmed = plain_root.trim_end_matches('\\');
    if trimmed.len() == 2 && trimmed.as_bytes()[1] == b':' {
        return Some(format!(
            "Scanning the entire {trimmed}\\ drive includes Windows system and program folders. Consider picking a specific user or project folder instead."
        ));
    }

    let mut protected = vec![
        r"C:\Windows".to_string(),
        r"C:\Program Files".to_string(),
        r"C:\Program Files (x86)".to_string(),
        r"C:\ProgramData".to_string(),
    ];
    for var in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() && !protected.iter().any(|p| p.eq_ignore_ascii_case(&val)) {
                protected.push(val);
            }
        }
    }

    for p in protected {
        if is_within(plain_root, &p) || is_within(&p, plain_root) {
            return Some(format!(
                "This folder overlaps a Windows system location ({p}). Renaming files inside Windows or installed applications can break programs."
            ));
        }
    }
    None
}

/// Validates that `name` is a single filename/folder segment with no path separators, traversal tokens,
/// control characters, or UTF-16 length over 255.
pub fn validate_single_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidInput("The new name cannot be empty.".into()));
    }
    if name == "." || name == ".." {
        return Err(Error::InvalidInput(
            "'.' and '..' cannot be used as file or folder names.".into(),
        ));
    }
    if name.contains(['\\', '/']) {
        return Err(Error::InvalidInput(
            "A rename can only change the item's own name, not move it into another folder.".into(),
        ));
    }
    if name.chars().any(|c| (c as u32) < 32) {
        return Err(Error::InvalidInput(
            "The name contains control characters.".into(),
        ));
    }
    if utf16_len(name) > MAX_NAME_LENGTH {
        return Err(Error::InvalidInput(format!(
            "The name is {} UTF-16 characters, which exceeds the 255-character filesystem limit.",
            utf16_len(name)
        )));
    }
    Ok(())
}

/// Verifies a planned or undo rename stays strictly inside `root`, touches only a single segment,
/// and does not follow an intermediate directory symlink/junction created after the scan (TOCTOU guard).
/// Returns `(from_ext, to_ext)`.
pub fn verify_rename_pair(
    root: &Root,
    dir_plain: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(String, String)> {
    if !is_within(dir_plain, &root.plain) {
        return Err(Error::InvalidInput(format!(
            "Refused rename outside the scanned folder ({}): {dir_plain}",
            root.plain
        )));
    }
    if old_name.is_empty() || old_name == "." || old_name == ".." || old_name.contains(['\\', '/'])
    {
        return Err(Error::InvalidInput(format!(
            "Invalid source name: {old_name}"
        )));
    }
    validate_single_name(new_name)?;

    let dir_ext = to_extended(dir_plain)?;
    let dir_meta = filesystem::metadata_extended(&dir_ext)
        .ok_or_else(|| Error::NotFound(format!("Parent folder no longer exists: {dir_plain}")))?;
    if !dir_meta.is_dir {
        return Err(Error::InvalidInput(format!(
            "Parent path is not a folder: {dir_plain}"
        )));
    }
    let root_trimmed = root.plain.trim_end_matches('\\');
    let dir_trimmed = dir_plain.trim_end_matches('\\');
    if !crate::rules::eq_ignore_case(dir_trimmed, root_trimmed) && dir_meta.is_reparse_point {
        return Err(Error::InvalidInput(format!(
            "Skipped: parent folder is a symlink or junction ({dir_plain})"
        )));
    }

    let join = |name: &str| -> Result<String> {
        let plain = if dir_plain.ends_with('\\') {
            format!("{dir_plain}{name}")
        } else {
            format!("{dir_plain}\\{name}")
        };
        to_extended(&plain)
    };

    Ok((join(old_name)?, join(new_name)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_paths_warn_appropriately() {
        assert!(protected_path_warning(r"C:\").is_some());
        assert!(protected_path_warning(r"C:\Windows\System32").is_some());
        assert!(protected_path_warning(r"C:\Program Files\App").is_some());
        assert!(protected_path_warning(r"C:\Users\alice\Documents").is_none());
    }

    #[test]
    fn single_name_validation_rejects_traversal_and_separators() {
        for bad in [
            "",
            ".",
            "..",
            r"a\b",
            "a/b",
            "bad\u{0}name",
            &"x".repeat(256),
        ] {
            assert!(validate_single_name(bad).is_err(), "{bad:?}");
        }
        assert!(validate_single_name("good file.txt").is_ok());
    }

    #[test]
    fn verify_rename_pair_blocks_root_escape() {
        let root = Root::new(r"C:\data\project").unwrap();
        assert!(verify_rename_pair(&root, r"C:\data\other", "a.txt", "b.txt").is_err());
        assert!(verify_rename_pair(&root, r"C:\data\project2", "a.txt", "b.txt").is_err());
    }
}
