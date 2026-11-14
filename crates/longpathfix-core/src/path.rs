//! Windows path forms.
//!
//! Three different things are easy to confuse, and this module keeps them apart:
//!
//! - **Representation.** `C:\data\x` is a classic Win32 path: Windows normalises it (removes a trailing dot
//!   or space from each name, resolves `.` and `..`) and, unless the process is long-path aware *and* the
//!   `LongPathsEnabled` policy is on, refuses it above 259 characters. `\\?\C:\data\x` (and
//!   `\\?\UNC\server\share\x` for network paths) is the *extended-length* form: it is passed to the file
//!   system as is, up to about 32,767 characters, so names Windows normally refuses (`notes.`, `CON`) can be
//!   reached. LongPathFix always talks to the disk in the extended form.
//! - **File-system support.** NTFS, ReFS, exFAT and FAT32 store names of up to 255 UTF-16 units; whether a
//!   *path* may be longer than 259 is a question for the program that opens it, not the disk.
//! - **Process compatibility.** Explorer, ZIP tools, backup and sync software differ in whether they use the
//!   extended form. That is why a path can exist on disk and still break the tools around it.
//!
//! Lengths here are counted in UTF-16 code units, the unit `MAX_PATH` and the 255-character name limit use.

use crate::{Error, Result};
use serde::Serialize;

pub const EXTENDED_PREFIX: &str = r"\\?\";
pub const EXTENDED_UNC_PREFIX: &str = r"\\?\UNC\";
/// `MAX_PATH` is 260 including the terminating NUL, so 259 visible characters.
pub const CLASSIC_LIMIT: usize = 259;
/// The longest path the extended form allows (approximately; the exact figure depends on the file system).
pub const EXTENDED_LIMIT: usize = 32_767;

/// The form a path string is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathForm {
    /// `C:\...`
    Drive,
    /// `\\server\share\...`
    Unc,
    /// `\\?\C:\...`
    ExtendedDrive,
    /// `\\?\UNC\server\share\...`
    ExtendedUnc,
    /// `\\.\...`, `\\?\GLOBALROOT\...`, `\\?\Volume{...}\...` and other device paths.
    Device,
    /// `C:foo` (relative to the current folder of drive C).
    DriveRelative,
    /// `\foo` (relative to the root of the current drive).
    RootRelative,
    /// `foo\bar`
    Relative,
    Empty,
}

pub fn classify(p: &str) -> PathForm {
    let b = p.as_bytes();
    if p.is_empty() {
        return PathForm::Empty;
    }
    if p.starts_with(EXTENDED_UNC_PREFIX) || starts_with_ci(p, r"\\?\UNC\") {
        return PathForm::ExtendedUnc;
    }
    if let Some(rest) = p.strip_prefix(EXTENDED_PREFIX) {
        let rb = rest.as_bytes();
        return if rb.len() >= 3 && rb[0].is_ascii_alphabetic() && rb[1] == b':' && rb[2] == b'\\' {
            PathForm::ExtendedDrive
        } else {
            PathForm::Device
        };
    }
    if p.starts_with(r"\\.\") || p.starts_with(r"\??\") {
        return PathForm::Device;
    }
    if p.starts_with(r"\\") {
        return PathForm::Unc;
    }
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return if b.len() >= 3 && b[2] == b'\\' {
            PathForm::Drive
        } else {
            PathForm::DriveRelative
        };
    }
    if b[0] == b'\\' {
        return PathForm::RootRelative;
    }
    PathForm::Relative
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// Length in UTF-16 code units.
pub fn utf16_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().map(char::len_utf16).sum()
    }
}

/// The extended (`\\?\`) form of an absolute path. An already extended path is returned unchanged. The path
/// is only prefixed, never normalised: normalising would cut a trailing dot or space off the last name.
/// Relative paths are rejected; callers resolve them first with [`normalize_absolute`].
pub fn to_extended(p: &str) -> Result<String> {
    match classify(p) {
        PathForm::ExtendedDrive | PathForm::ExtendedUnc => Ok(p.to_string()),
        PathForm::Drive => Ok(format!("{EXTENDED_PREFIX}{p}")),
        PathForm::Unc => Ok(format!("{EXTENDED_UNC_PREFIX}{}", &p[2..])),
        _ => Err(Error::InvalidInput(format!(
            "Not an absolute drive or network path: {p}"
        ))),
    }
}

/// The classic form of a path: `\\?\C:\x` becomes `C:\x` and `\\?\UNC\srv\share` becomes `\\srv\share`.
pub fn to_plain(p: &str) -> String {
    if starts_with_ci(p, EXTENDED_UNC_PREFIX) {
        format!(r"\\{}", &p[EXTENDED_UNC_PREFIX.len()..])
    } else if let Some(rest) = p.strip_prefix(EXTENDED_PREFIX) {
        rest.to_string()
    } else {
        p.to_string()
    }
}

/// A folder chosen for scanning, in its two forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Root {
    /// Classic form without a trailing backslash, except for a drive root (`C:\`).
    pub plain: String,
    /// Extended form, used for every disk access.
    pub extended: String,
    /// UTF-16 length that names below the root add to: `C:\data` gives 7 (`C:\data` + `\` + name), `C:\` gives 2.
    pub len16: usize,
    pub form: PathForm,
}

impl Root {
    /// The root from an already normalised absolute path (see [`normalize_absolute`]).
    pub fn new(normalized_plain: &str) -> Result<Self> {
        let form = classify(normalized_plain);
        if !matches!(form, PathForm::Drive | PathForm::Unc) {
            return Err(Error::InvalidInput(format!(
                "Not an absolute drive or network path: {normalized_plain}"
            )));
        }
        let trimmed = normalized_plain.trim_end_matches('\\');
        Ok(Self {
            plain: normalized_plain.to_string(),
            extended: to_extended(normalized_plain)?,
            len16: utf16_len(trimmed),
            form,
        })
    }

    /// `plain` followed by `\rel` (or just `rel` after a drive root).
    pub fn join_plain(&self, rel: &str) -> String {
        if rel.is_empty() {
            self.plain.clone()
        } else if self.plain.ends_with('\\') {
            format!("{}{rel}", self.plain)
        } else {
            format!("{}\\{rel}", self.plain)
        }
    }
}

/// Turns user input into a clean absolute path in classic form, resolving `.` and `..` *lexically* (names
/// are kept exactly as typed, including trailing dots, which `GetFullPathName` would remove).
///
/// Accepts `C:\...`, `\\server\share\...` and their `\\?\` forms; `/` is treated as `\`; surrounding quotes
/// and whitespace are removed. Rejects relative and drive-relative paths, device paths (`\\.\`,
/// `\\?\GLOBALROOT`), wildcards and control characters.
pub fn normalize_absolute(input: &str) -> Result<String> {
    let outer = input.trim_start().trim_end_matches(['\r', '\n', '\t']);
    let outer_no_trail = outer.trim_end_matches(' ');
    let s = if outer_no_trail.len() >= 2
        && outer_no_trail.starts_with('"')
        && outer_no_trail.ends_with('"')
    {
        &outer_no_trail[1..outer_no_trail.len() - 1]
    } else {
        outer
    };
    if s.is_empty() {
        return Err(Error::InvalidInput("Choose a folder first.".into()));
    }
    if s.chars().any(|c| (c as u32) < 32) {
        return Err(Error::InvalidInput(
            "The path contains control characters.".into(),
        ));
    }
    if s.contains(['*', '?']) && !s.starts_with(EXTENDED_PREFIX) {
        return Err(Error::InvalidInput(
            "The path contains * or ?; wildcards are not allowed.".into(),
        ));
    }
    let s = s.replace('/', "\\");
    let (prefix, rest) = match classify(&s) {
        PathForm::Drive => {
            let letter = s.as_bytes()[0].to_ascii_uppercase() as char;
            (format!("{letter}:"), s[2..].to_string())
        }
        PathForm::ExtendedDrive => {
            let p = &s[EXTENDED_PREFIX.len()..];
            let letter = p.as_bytes()[0].to_ascii_uppercase() as char;
            (format!("{letter}:"), p[2..].to_string())
        }
        PathForm::Unc | PathForm::ExtendedUnc => {
            let body = if classify(&s) == PathForm::Unc { &s[2..] } else { &s[EXTENDED_UNC_PREFIX.len()..] };
            let mut parts = body.splitn(3, '\\');
            let server = parts.next().unwrap_or("");
            let share = parts.next().unwrap_or("");
            if server.is_empty() || share.is_empty() || server == "." || server == "?" {
                return Err(Error::InvalidInput("A network path needs a server and a share: \\\\server\\share".into()));
            }
            (format!(r"\\{server}\{share}"), parts.next().map(|r| format!("\\{r}")).unwrap_or_default())
        }
        PathForm::Device => {
            return Err(Error::InvalidInput(
                "Device paths (\\\\.\\ or \\\\?\\GLOBALROOT) are not supported. Use a drive letter or a network path.".into(),
            ))
        }
        PathForm::DriveRelative | PathForm::RootRelative | PathForm::Relative | PathForm::Empty => {
            return Err(Error::InvalidInput(
                "Use a full path that starts with a drive letter (C:\\...) or a network path (\\\\server\\share).".into(),
            ))
        }
    };
    if rest.contains(['*', '?', '<', '>', '|', '"']) || rest.contains(':') {
        return Err(Error::InvalidInput(
            "The path contains characters that are not allowed in a folder path.".into(),
        ));
    }
    let mut parts: Vec<&str> = Vec::new();
    for c in rest.split('\\') {
        match c {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(Error::InvalidInput(
                        "The path goes above the root of the drive or share.".into(),
                    ));
                }
            }
            other => parts.push(other),
        }
    }
    let unc = prefix.starts_with(r"\\");
    Ok(if parts.is_empty() {
        if unc {
            prefix
        } else {
            format!("{prefix}\\")
        }
    } else {
        format!("{prefix}\\{}", parts.join("\\"))
    })
}

/// Is `path` equal to `base` or below it? Case-insensitive, component-aware (`C:\data2` is not below `C:\data`).
pub fn is_within(path: &str, base: &str) -> bool {
    let p = path.trim_end_matches('\\');
    let b = base.trim_end_matches('\\');
    if p.len() < b.len() || !p.is_char_boundary(b.len()) {
        return false;
    }
    let (head, tail) = p.split_at(b.len());
    crate::rules::eq_ignore_case(head, b) && (tail.is_empty() || tail.starts_with('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_every_form() {
        assert_eq!(classify(r"C:\a"), PathForm::Drive);
        assert_eq!(classify(r"\\srv\share\a"), PathForm::Unc);
        assert_eq!(classify(r"\\?\C:\a"), PathForm::ExtendedDrive);
        assert_eq!(classify(r"\\?\UNC\srv\share"), PathForm::ExtendedUnc);
        assert_eq!(classify(r"\\.\PhysicalDrive0"), PathForm::Device);
        assert_eq!(classify(r"\\?\GLOBALROOT\Device\X"), PathForm::Device);
        assert_eq!(classify(r"\\?\Volume{1234}\x"), PathForm::Device);
        assert_eq!(classify("C:foo"), PathForm::DriveRelative);
        assert_eq!(classify(r"\foo"), PathForm::RootRelative);
        assert_eq!(classify(r"foo\bar"), PathForm::Relative);
        assert_eq!(classify(""), PathForm::Empty);
    }

    #[test]
    fn plain_and_extended_convert_both_ways() {
        assert_eq!(to_extended(r"C:\a\b").unwrap(), r"\\?\C:\a\b");
        assert_eq!(
            to_extended(r"\\srv\share\x").unwrap(),
            r"\\?\UNC\srv\share\x"
        );
        assert_eq!(to_plain(r"\\?\C:\a\b"), r"C:\a\b");
        assert_eq!(to_plain(r"\\?\UNC\srv\share\x"), r"\\srv\share\x");
        assert_eq!(to_extended(r"\\?\C:\a").unwrap(), r"\\?\C:\a");
        assert_eq!(
            to_extended(r"C:\trail. ").unwrap(),
            r"\\?\C:\trail. ",
            "never normalised"
        );
        assert!(to_extended(r"a\b").is_err());
    }

    #[test]
    fn normalises_user_input_lexically() {
        assert_eq!(
            normalize_absolute(r#"  "c:/Data/./x/../y"  "#).unwrap(),
            r"C:\Data\y"
        );
        assert_eq!(normalize_absolute(r"C:\").unwrap(), r"C:\");
        assert_eq!(normalize_absolute(r"C:\\a\\\b\").unwrap(), r"C:\a\b");
        assert_eq!(
            normalize_absolute(r"C:\keep. ").unwrap(),
            r"C:\keep. ",
            "trailing dot and space survive"
        );
        assert_eq!(normalize_absolute(r"\\?\C:\long").unwrap(), r"C:\long");
        assert_eq!(
            normalize_absolute(r"\\srv\share\a\..\b").unwrap(),
            r"\\srv\share\b"
        );
        assert_eq!(
            normalize_absolute(r"\\?\UNC\srv\share").unwrap(),
            r"\\srv\share"
        );
        assert_eq!(normalize_absolute(r"\\srv\share").unwrap(), r"\\srv\share");
    }

    #[test]
    fn rejects_unsafe_or_relative_input() {
        for bad in [
            "",
            "   ",
            r"relative\path",
            "C:foo",
            r"\rooted",
            r"\\.\C:",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1",
            r"C:\a*",
            r"C:\a?b",
            r"C:\..",
            r"\\srv",
            r"\\srv\",
            "C:\\a\u{0}b",
            r"C:\a:b",
        ] {
            assert!(
                normalize_absolute(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn root_lengths_match_what_names_add_to() {
        assert_eq!(Root::new(r"C:\data").unwrap().len16, 7);
        assert_eq!(Root::new(r"C:\").unwrap().len16, 2);
        assert_eq!(Root::new(r"C:\").unwrap().extended, r"\\?\C:\");
        assert_eq!(
            Root::new(r"\\srv\share").unwrap().extended,
            r"\\?\UNC\srv\share"
        );
        assert_eq!(Root::new(r"C:\").unwrap().join_plain("a"), r"C:\a");
        assert_eq!(Root::new(r"C:\d").unwrap().join_plain(r"a\b"), r"C:\d\a\b");
    }

    #[test]
    fn utf16_length_counts_surrogate_pairs_as_two() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("é"), 1);
        assert_eq!(utf16_len("😀"), 2);
        assert_eq!(utf16_len("日本"), 2);
    }

    #[test]
    fn within_is_component_aware() {
        assert!(is_within(r"C:\Windows\System32", r"C:\WINDOWS"));
        assert!(is_within(r"C:\Windows", r"C:\Windows\"));
        assert!(!is_within(r"C:\Windows2", r"C:\Windows"));
        assert!(!is_within(r"C:\", r"C:\Windows"));
    }
}
