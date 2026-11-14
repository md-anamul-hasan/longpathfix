//! Single-segment filename rules and classification.
//!
//! Covers every rule from LongPathFix 1.x (paths over limit, names over 255 UTF-16 units,
//! trailing dot/space, leading space, Windows reserved device names, illegal characters and
//! control chars, and case-only clashes) plus Unicode normalization folding and invalid-Unicode
//! replacement detection.

use crate::path::utf16_len;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

pub const MAX_NAME_LENGTH: usize = 255;

const ILLEGAL_CHARS: &str = "<>:\"/\\|?*";

const RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3",
    "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Bitflags representing issues detected on a single file or directory node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Issue(pub u32);

impl Issue {
    pub const NONE: Self = Self(0);
    pub const TOO_LONG: Self = Self(1);
    pub const NAME_TOO_LONG: Self = Self(2);
    pub const TRAILING_DOT_OR_SPACE: Self = Self(4);
    pub const LEADING_SPACE: Self = Self(8);
    pub const RESERVED_NAME: Self = Self(16);
    pub const ILLEGAL_CHARS: Self = Self(32);
    pub const CASE_CLASH: Self = Self(64);
    pub const INVALID_UNICODE: Self = Self(128);

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0 && other.0 != 0
    }

    #[inline]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl std::ops::BitOr for Issue {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Issue {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for Issue {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Bitflags controlling which categories of issues the planner should actively fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FixKinds(pub u32);

impl FixKinds {
    pub const NONE: Self = Self(0);
    pub const LONG_PATHS: Self = Self(1);
    pub const ILLEGAL_CHARS: Self = Self(2);
    pub const TRAILING_DOTS_AND_SPACES: Self = Self(4);
    pub const RESERVED_NAMES: Self = Self(8);
    pub const CASE_CLASHES: Self = Self(16);
    pub const ALL: Self = Self(1 | 2 | 4 | 8 | 16);

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0 && other.0 != 0
    }
}

impl Default for FixKinds {
    fn default() -> Self {
        Self::ALL
    }
}

impl std::ops::BitOr for FixKinds {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for FixKinds {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::Not for FixKinds {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

/// Returns `true` if `name` (with or without an extension) matches a Windows reserved DOS device name,
/// including `COM1..9`, `LPT1..9`, `COM0`/`LPT0`, `CON`, `PRN`, `AUX`, `NUL`, `CLOCK$`, `CONIN$`, `CONOUT$`,
/// and Unicode superscript variants `COM¹`/`COM²`/`COM³` / `LPT¹`/`LPT²`/`LPT³`.
pub fn is_reserved(name: &str) -> bool {
    let before_dot = name.split('.').next().unwrap_or(name);
    let stem = before_dot.trim_end_matches([' ', '.']);
    if RESERVED_STEMS.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
        return true;
    }
    // Check Windows-special COM¹/²/³ and LPT¹/²/³ superscript device aliases
    let mut chars = stem.chars();
    let prefix: String = (&mut chars).take(3).collect();
    let rest: String = chars.collect();
    if (prefix.eq_ignore_ascii_case("COM") || prefix.eq_ignore_ascii_case("LPT"))
        && matches!(rest.as_str(), "\u{00B9}" | "\u{00B2}" | "\u{00B3}")
    {
        return true;
    }
    false
}

/// Returns `true` if `name` contains ASCII control chars (< 32) or Windows-disallowed characters `<>:"/\|?*`.
pub fn has_illegal_chars(name: &str) -> bool {
    name.chars()
        .any(|c| (c as u32) < 32 || ILLEGAL_CHARS.contains(c))
}

/// Returns `true` if `name` contains U+FFFD (produced by lossy UTF-16 decoding of unpaired surrogates).
pub fn has_invalid_unicode(name: &str) -> bool {
    name.contains('\u{FFFD}')
}

/// Inspects a single filename or folder name for local defects (path length and case clashes are evaluated across the tree).
pub fn check(name: &str) -> Issue {
    let mut issue = Issue::NONE;
    if utf16_len(name) > MAX_NAME_LENGTH {
        issue |= Issue::NAME_TOO_LONG;
    }
    if name.ends_with('.') || name.ends_with(' ') {
        issue |= Issue::TRAILING_DOT_OR_SPACE;
    }
    if name.starts_with(' ') {
        issue |= Issue::LEADING_SPACE;
    }
    if is_reserved(name) {
        issue |= Issue::RESERVED_NAME;
    }
    if has_illegal_chars(name) {
        issue |= Issue::ILLEGAL_CHARS;
    }
    if has_invalid_unicode(name) {
        issue |= Issue::INVALID_UNICODE;
    }
    issue
}

/// Produces a cleaned copy of `name` with the problems selected by `fix` repaired.
pub fn clean(name: &str, fix: Issue) -> String {
    let mut n = name.to_string();

    if fix.contains(Issue::ILLEGAL_CHARS) || fix.contains(Issue::INVALID_UNICODE) {
        let replace_illegal = fix.contains(Issue::ILLEGAL_CHARS);
        let replace_unicode = fix.contains(Issue::INVALID_UNICODE);
        let mut out = String::with_capacity(n.len());
        for c in n.chars() {
            if replace_illegal && (c as u32) < 32 {
                continue;
            }
            if (replace_illegal && ILLEGAL_CHARS.contains(c))
                || (replace_unicode && c == '\u{FFFD}')
            {
                out.push('_');
            } else {
                out.push(c);
            }
        }
        n = out;
    }

    if fix.contains(Issue::LEADING_SPACE) {
        n = n.trim_start_matches(' ').to_string();
    }
    if fix.contains(Issue::TRAILING_DOT_OR_SPACE) {
        n = n.trim_end_matches(['.', ' ']).to_string();
    }
    if n.is_empty() {
        n = "_".to_string();
    }
    if fix.contains(Issue::RESERVED_NAME) && is_reserved(&n) {
        n = if let Some(dot) = n.find('.') {
            format!("{}_{}", n[..dot].trim_end_matches(' '), &n[dot..])
        } else {
            format!("{}_", n.trim_end_matches(' '))
        };
    }
    n
}

/// Returns human-readable labels for every issue flag set on `i`.
pub fn labels(i: Issue) -> Vec<&'static str> {
    let mut p = Vec::new();
    if i.contains(Issue::TOO_LONG) {
        p.push("path too long");
    }
    if i.contains(Issue::NAME_TOO_LONG) {
        p.push("name over 255 characters");
    }
    if i.contains(Issue::TRAILING_DOT_OR_SPACE) {
        p.push("ends with dot or space");
    }
    if i.contains(Issue::LEADING_SPACE) {
        p.push("starts with a space");
    }
    if i.contains(Issue::RESERVED_NAME) {
        p.push("reserved name");
    }
    if i.contains(Issue::ILLEGAL_CHARS) {
        p.push("illegal characters");
    }
    if i.contains(Issue::CASE_CLASH) {
        p.push("differs only by case from another name");
    }
    if i.contains(Issue::INVALID_UNICODE) {
        p.push("invalid Unicode characters");
    }
    p
}

/// Comma-joined human description matching 1.x `NameRules.Text`.
pub fn text(i: Issue) -> String {
    labels(i).join(", ")
}

/// Unicode NFC + uppercase fold used for Windows/macOS case-clash detection.
pub fn fold(s: &str) -> String {
    if s.is_ascii() {
        s.to_ascii_uppercase()
    } else {
        s.nfc().collect::<String>().to_uppercase()
    }
}

/// Case-insensitive equality using NFC folding for non-ASCII strings and a zero-alloc ASCII fast path.
pub fn eq_ignore_case(a: &str, b: &str) -> bool {
    if a.is_ascii() && b.is_ascii() {
        a.eq_ignore_ascii_case(b)
    } else {
        fold(a) == fold(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_tree_names_are_valid() {
        for name in [
            "src",
            "main.c",
            "readme.md",
            "a b (1).txt",
            "CON1.txt",
            "console.log",
            "COM10.txt",
        ] {
            assert_eq!(check(name), Issue::NONE, "{name}");
        }
    }

    #[test]
    fn detects_and_cleans_illegal_characters() {
        assert_eq!(
            clean("report<final>.txt", Issue::ILLEGAL_CHARS),
            "report_final_.txt"
        );
        assert_eq!(
            clean("a:b|c?d*e\"f.txt", Issue::ILLEGAL_CHARS),
            "a_b_c_d_e_f.txt"
        );
        assert_eq!(clean("tab\there.txt", Issue::ILLEGAL_CHARS), "tabhere.txt");
    }

    #[test]
    fn detects_and_cleans_trailing_and_leading_chars() {
        assert_eq!(clean("notes.", Issue::TRAILING_DOT_OR_SPACE), "notes");
        assert_eq!(clean("notes...", Issue::TRAILING_DOT_OR_SPACE), "notes");
        assert_eq!(clean("notes ", Issue::TRAILING_DOT_OR_SPACE), "notes");
        assert_eq!(clean("folder. ", Issue::TRAILING_DOT_OR_SPACE), "folder");
        assert_eq!(clean(" lead.txt", Issue::LEADING_SPACE), "lead.txt");
        assert_eq!(clean("...", Issue::TRAILING_DOT_OR_SPACE), "_");
    }

    #[test]
    fn detects_and_cleans_reserved_names() {
        for (input, expected) in [
            ("CON", "CON_"),
            ("con.txt", "con_.txt"),
            ("Nul.tar.gz", "Nul_.tar.gz"),
            ("COM1", "COM1_"),
            ("lpt9.log", "lpt9_.log"),
            ("aux .txt", "aux_.txt"),
            ("COM0.txt", "COM0_.txt"),
            ("COM\u{00B9}.txt", "COM\u{00B9}_.txt"),
            ("LPT\u{00B3}.log", "LPT\u{00B3}_.log"),
            ("CLOCK$", "CLOCK$_"),
        ] {
            assert!(check(input).contains(Issue::RESERVED_NAME), "{input}");
            assert_eq!(clean(input, Issue::RESERVED_NAME), expected, "{input}");
        }
    }

    #[test]
    fn detects_invalid_unicode_surrogate_replacement() {
        let bad = "file_\u{FFFD}.txt";
        assert!(check(bad).contains(Issue::INVALID_UNICODE));
        assert_eq!(clean(bad, Issue::INVALID_UNICODE), "file__.txt");
    }

    #[test]
    fn utf16_name_limit_handles_emoji() {
        // 128 emojis = 256 UTF-16 code units -> over 255
        let over: String = std::iter::repeat_n('😀', 128).collect();
        assert!(check(&over).contains(Issue::NAME_TOO_LONG));
        // 127 emojis = 254 UTF-16 code units -> fine
        let ok: String = std::iter::repeat_n('😀', 127).collect();
        assert!(!check(&ok).contains(Issue::NAME_TOO_LONG));
    }

    #[test]
    fn nfc_folding_matches_composed_and_decomposed_equivalents() {
        assert!(eq_ignore_case("e\u{0301}.txt", "É.txt"));
        assert!(eq_ignore_case("ä", "Ä"));
    }
}
