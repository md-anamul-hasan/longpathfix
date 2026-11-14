//! Pure rename planner.
//!
//! Computes every issue and every planned rename from an in-memory tree of scanned nodes
//! before touching a single byte on disk. Preserves extensions, cuts names using deterministic
//! `~xxxx` SHA-1 prefixes from the original relative path, resolves clashes with `" (2)"`, `" (3)"`,
//! measures all lengths in UTF-16 code units, and honestly marks unfixable paths.

use crate::path::{self, utf16_len};
use crate::rules::{self, FixKinds, Issue, MAX_NAME_LENGTH};
use crate::scanner::Node;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const MIN_BASE: usize = 8;
pub const HASH_PART: usize = 5; // "~" + 4 lowercase hex digits
pub const CLASSIC_LIMIT: usize = 259;

/// One planned rename operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOp {
    pub rel: String,
    pub old_name: String,
    pub new_name: String,
    pub issues: Issue,
    pub is_directory: bool,
}

impl RenameOp {
    pub fn parent(&self) -> &str {
        match self.rel.rfind('\\') {
            Some(pos) => &self.rel[..pos],
            None => "",
        }
    }
}

/// A scanned item that either has a problem, gets a planned new name, or cannot be fixed by renaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub node: Node,
    pub issues: Issue,
    pub new_name: Option<String>,
    pub unresolved: Option<String>,
    pub current_len: usize,
    pub planned_len: usize,
    pub issue_text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCounts {
    pub too_long: usize,
    pub name_too_long: usize,
    pub trailing_dot_or_space: usize,
    pub leading_space: usize,
    pub reserved_name: usize,
    pub illegal_chars: usize,
    pub case_clash: usize,
    pub invalid_unicode: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LengthBucket {
    pub label: String,
    pub min: usize,
    pub max: usize,
    pub before: usize,
    pub after: usize,
}

/// Complete rename plan for a scanned tree under a chosen character limit and fix mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub findings: Vec<Finding>,
    pub ops: Vec<RenameOp>,
    pub root_length: usize,
    pub limit: usize,
    pub scanned: usize,
    pub scanned_files: usize,
    pub scanned_dirs: usize,
    pub skipped_reparse_points: usize,
    pub readonly_findings: usize,
    pub max_len_before: usize,
    pub max_len_after: usize,
    pub issue_counts: IssueCounts,
    pub histogram: Vec<LengthBucket>,
    pub unresolved_count: usize,
}

pub struct Planner;

impl Planner {
    pub fn build(root: &str, scanned: &[Node], limit: usize, kinds: FixKinds) -> Plan {
        let root_len = utf16_len(path::to_plain(root).trim_end_matches('\\'));

        // Deduplicate by `rel` while preserving (depth, rel) ordering
        let mut sorted: Vec<Node> = scanned
            .iter()
            .filter(|n| !n.rel.is_empty())
            .cloned()
            .collect();
        sorted.sort_by(|a, b| a.depth().cmp(&b.depth()).then_with(|| a.rel.cmp(&b.rel)));
        let mut seen = HashSet::with_capacity(sorted.len());
        sorted.retain(|n| seen.insert(n.rel.clone()));

        let n_len = sorted.len();
        let mut scanned_files = 0usize;
        let mut scanned_dirs = 0usize;
        let mut skipped_reparse_points = 0usize;
        for n in &sorted {
            if n.is_directory {
                scanned_dirs += 1;
            } else {
                scanned_files += 1;
            }
            if n.is_reparse_point {
                skipped_reparse_points += 1;
            }
        }

        let mut rel_to_idx: HashMap<&str, usize> = HashMap::with_capacity(n_len);
        for (i, n) in sorted.iter().enumerate() {
            rel_to_idx.insert(n.rel.as_str(), i);
        }

        let parent_idx: Vec<Option<usize>> = sorted
            .iter()
            .map(|n| {
                let p = n.parent();
                if p.is_empty() {
                    None
                } else {
                    rel_to_idx.get(p).copied()
                }
            })
            .collect();

        let mut new_name: Vec<String> = sorted.iter().map(|n| n.name().to_string()).collect();
        let mut new_len16: Vec<usize> = new_name.iter().map(|s| utf16_len(s)).collect();
        let mut issues: Vec<Issue> = sorted
            .iter()
            .map(|n| {
                let mut i = rules::check(n.name());
                if n.invalid_unicode {
                    i |= Issue::INVALID_UNICODE;
                }
                i
            })
            .collect();
        let mut unresolved: Vec<Option<String>> = vec![None; n_len];
        let mark_case = kinds.contains(FixKinds::CASE_CLASHES);

        // Step 1: Fix single-segment naming issues (illegal chars, invalid Unicode, leading/trailing whitespace/dots, reserved names)
        let fix = wanted_issues(kinds);
        for i in 0..n_len {
            let present = issues[i] & fix;
            if !present.is_empty() {
                new_name[i] = rules::clean(sorted[i].name(), present);
                new_len16[i] = utf16_len(&new_name[i]);
            }
        }
        resolve_clashes(
            &sorted,
            &mut new_name,
            &mut new_len16,
            &mut issues,
            mark_case,
        );

        // Helper to compute total UTF-16 path length for node `idx` under current `new_name`
        let compute_full_len = |idx: usize, lens: &[usize]| -> usize {
            let mut total = root_len;
            let mut cur_idx = Some(idx);
            let mut cur_rel = sorted[idx].rel.as_str();
            while !cur_rel.is_empty() {
                if let Some(ci) = cur_idx {
                    total += 1 + lens[ci];
                    cur_rel = sorted[ci].parent();
                    cur_idx = parent_idx[ci];
                } else {
                    // Sparse tree fallback when an intermediate parent Node was omitted
                    let (p, seg) = match cur_rel.rfind('\\') {
                        Some(pos) => (&cur_rel[..pos], &cur_rel[pos + 1..]),
                        None => ("", cur_rel),
                    };
                    total += 1 + utf16_len(seg);
                    cur_rel = p;
                    cur_idx = if p.is_empty() {
                        None
                    } else {
                        rel_to_idx.get(p).copied()
                    };
                }
            }
            total
        };

        // Step 2: Over-255 names and over-limit full paths (longest paths first; leaf name before its ancestor folders)
        if kinds.contains(FixKinds::LONG_PATHS) {
            for i in 0..n_len {
                if new_len16[i] > MAX_NAME_LENGTH {
                    let base16 = base_len_utf16(&new_name[i], sorted[i].is_directory);
                    let ext16 = new_len16[i] - base16;
                    let keep = MAX_NAME_LENGTH
                        .saturating_sub(ext16 + HASH_PART)
                        .max(MIN_BASE);
                    new_name[i] = cut(&new_name[i], &sorted[i].rel, keep, sorted[i].is_directory);
                    new_len16[i] = utf16_len(&new_name[i]);
                }
            }

            let mut order: Vec<(usize, usize)> = (0..n_len)
                .map(|i| (i, compute_full_len(i, &new_len16)))
                .collect();
            order.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

            for (idx, initial_len) in order {
                if initial_len <= limit {
                    break;
                }
                let mut cur_total = compute_full_len(idx, &new_len16);
                if cur_total <= limit {
                    continue;
                }

                // Walk node first, then its parent folders upward
                let mut chain_cursor = Some(idx);
                while let Some(c_idx) = chain_cursor {
                    if cur_total <= limit {
                        break;
                    }
                    let excess = cur_total - limit;
                    let cur_base16 = base_len_utf16(&new_name[c_idx], sorted[c_idx].is_directory);
                    let keep = cur_base16.saturating_sub(excess + HASH_PART).max(MIN_BASE);
                    if keep + HASH_PART < cur_base16 {
                        let old_len16 = new_len16[c_idx];
                        new_name[c_idx] = cut(
                            &new_name[c_idx],
                            &sorted[c_idx].rel,
                            keep,
                            sorted[c_idx].is_directory,
                        );
                        new_len16[c_idx] = utf16_len(&new_name[c_idx]);
                        let saved = old_len16.saturating_sub(new_len16[c_idx]);
                        cur_total = cur_total.saturating_sub(saved);
                    }
                    chain_cursor = parent_idx[c_idx];
                }

                let final_len = compute_full_len(idx, &new_len16);
                if final_len > limit {
                    unresolved[idx] = Some(format!(
                        "Even with every name cut to {MIN_BASE} characters the path stays at {final_len} characters. Move the folder closer to the drive root."
                    ));
                }
            }

            resolve_clashes(
                &sorted,
                &mut new_name,
                &mut new_len16,
                &mut issues,
                mark_case,
            );
        }

        // Step 3: Aggregate findings, rename ops, counts, and length histogram
        let mut buckets = [
            (0usize, 159usize, "0–159", 0usize, 0usize),
            (160, 199, "160–199", 0, 0),
            (200, 239, "200–239", 0, 0),
            (240, 259, "240–259", 0, 0),
            (260, 319, "260–319", 0, 0),
            (320, usize::MAX, "320+", 0, 0),
        ];

        let mut findings = Vec::new();
        let mut ops = Vec::new();
        let mut counts = IssueCounts::default();
        let mut max_len_before = root_len;
        let mut max_len_after = root_len;
        let mut readonly_findings = 0usize;
        let mut unresolved_count = 0usize;

        for i in 0..n_len {
            let n = &sorted[i];
            let before_len = root_len + 1 + utf16_len(&n.rel);
            let after_len = compute_full_len(i, &new_len16);
            max_len_before = max_len_before.max(before_len);
            max_len_after = max_len_after.max(after_len);

            for b in &mut buckets {
                if before_len >= b.0 && before_len <= b.1 {
                    b.3 += 1;
                }
                if after_len >= b.0 && after_len <= b.1 {
                    b.4 += 1;
                }
            }

            let mut found = issues[i];
            if before_len > limit {
                found |= Issue::TOO_LONG;
            }

            let target = &new_name[i];
            let changes = target != n.name();
            let mut why = unresolved[i].clone();
            if why.is_none() && kinds.contains(FixKinds::LONG_PATHS) && after_len > limit {
                why = Some("Still too long after the planned renames.".to_string());
            }

            if found.is_empty() && !changes && why.is_none() {
                continue;
            }

            if found.contains(Issue::TOO_LONG) {
                counts.too_long += 1;
            }
            if found.contains(Issue::NAME_TOO_LONG) {
                counts.name_too_long += 1;
            }
            if found.contains(Issue::TRAILING_DOT_OR_SPACE) {
                counts.trailing_dot_or_space += 1;
            }
            if found.contains(Issue::LEADING_SPACE) {
                counts.leading_space += 1;
            }
            if found.contains(Issue::RESERVED_NAME) {
                counts.reserved_name += 1;
            }
            if found.contains(Issue::ILLEGAL_CHARS) {
                counts.illegal_chars += 1;
            }
            if found.contains(Issue::CASE_CLASH) {
                counts.case_clash += 1;
            }
            if found.contains(Issue::INVALID_UNICODE) {
                counts.invalid_unicode += 1;
            }
            if n.is_readonly {
                readonly_findings += 1;
            }
            if why.is_some() {
                unresolved_count += 1;
            }

            if changes {
                ops.push(RenameOp {
                    rel: n.rel.clone(),
                    old_name: n.name().to_string(),
                    new_name: target.clone(),
                    issues: found,
                    is_directory: n.is_directory,
                });
            }

            findings.push(Finding {
                node: n.clone(),
                issues: found,
                new_name: if changes { Some(target.clone()) } else { None },
                unresolved: why,
                current_len: before_len,
                planned_len: after_len,
                issue_text: rules::text(found),
            });
        }

        let histogram = buckets
            .into_iter()
            .map(|(min, max, label, before, after)| LengthBucket {
                label: label.to_string(),
                min,
                max: if max == usize::MAX { 9999 } else { max },
                before,
                after,
            })
            .collect();

        Plan {
            findings,
            ops,
            root_length: root_len,
            limit,
            scanned: n_len,
            scanned_files,
            scanned_dirs,
            skipped_reparse_points,
            readonly_findings,
            max_len_before,
            max_len_after,
            issue_counts: counts,
            histogram,
            unresolved_count,
        }
    }
}

fn wanted_issues(k: FixKinds) -> Issue {
    let mut i = Issue::NONE;
    if k.contains(FixKinds::ILLEGAL_CHARS) {
        i |= Issue::ILLEGAL_CHARS | Issue::INVALID_UNICODE;
    }
    if k.contains(FixKinds::TRAILING_DOTS_AND_SPACES) {
        i |= Issue::TRAILING_DOT_OR_SPACE | Issue::LEADING_SPACE;
    }
    if k.contains(FixKinds::RESERVED_NAMES) {
        i |= Issue::RESERVED_NAME;
    }
    i
}

/// Byte offset splitting `name` into `(stem, extension)`, matching 1.x `BaseLength` (hidden dotfiles like `.gitignore` have no extension).
fn base_split_byte(name: &str, is_directory: bool) -> usize {
    if is_directory {
        return name.len();
    }
    match name.rfind('.') {
        Some(dot) if dot > 0 => dot,
        _ => name.len(),
    }
}

fn base_len_utf16(name: &str, is_directory: bool) -> usize {
    utf16_len(&name[..base_split_byte(name, is_directory)])
}

/// Keeps up to `keep_utf16` UTF-16 code units of the base name (never splitting a character or surrogate pair),
/// appends `~` + 4 hex digits of `SHA1(orig_rel)`, and reattaches the original extension.
fn cut(name: &str, orig_rel: &str, keep_utf16: usize, is_directory: bool) -> String {
    let split = base_split_byte(name, is_directory);
    let stem = &name[..split];
    let ext = &name[split..];

    let stem_len16 = utf16_len(stem);
    if keep_utf16 >= stem_len16 {
        return name.to_string();
    }

    let mut cut_bytes = 0usize;
    let mut acc16 = 0usize;
    for (idx, ch) in stem.char_indices() {
        let clen = ch.len_utf16();
        if acc16 + clen > keep_utf16 {
            break;
        }
        acc16 += clen;
        cut_bytes = idx + ch.len_utf8();
    }

    let mut prefix = stem[..cut_bytes].trim_end_matches([' ', '.']);
    if prefix.is_empty() {
        prefix = "_";
    }
    let hash = sha1_hex4(orig_rel.as_bytes());
    format!("{prefix}~{hash}{ext}")
}

fn resolve_clashes(
    nodes: &[Node],
    new_name: &mut [String],
    new_len16: &mut [usize],
    issues: &mut [Issue],
    mark_case: bool,
) {
    let mut by_parent: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        by_parent.entry(n.parent()).or_default().push(i);
    }

    for mut siblings in by_parent.into_values() {
        if siblings.len() < 2 {
            continue;
        }

        // Unchanged names keep priority over newly renamed ones; then ordinal order decides
        siblings.sort_by(|&a, &b| {
            let ca = new_name[a] != nodes[a].name();
            let cb = new_name[b] != nodes[b].name();
            ca.cmp(&cb)
                .then_with(|| nodes[a].name().cmp(nodes[b].name()))
        });

        let mut used: HashSet<String> = HashSet::with_capacity(siblings.len());
        for &idx in &siblings {
            let key = if mark_case {
                rules::fold(&new_name[idx])
            } else {
                new_name[idx].clone()
            };
            if used.insert(key) {
                continue;
            }

            let cur = new_name[idx].clone();
            let split = base_split_byte(&cur, nodes[idx].is_directory);
            let stem = &cur[..split];
            let ext = &cur[split..];
            for n in 2usize.. {
                let candidate = format!("{stem} ({n}){ext}");
                let c_key = if mark_case {
                    rules::fold(&candidate)
                } else {
                    candidate.clone()
                };
                if used.insert(c_key) {
                    new_len16[idx] = utf16_len(&candidate);
                    new_name[idx] = candidate;
                    break;
                }
            }
        }

        if mark_case {
            let mut by_folded: HashMap<String, Vec<usize>> = HashMap::new();
            for &idx in &siblings {
                by_folded
                    .entry(rules::fold(nodes[idx].name()))
                    .or_default()
                    .push(idx);
            }
            for group in by_folded.into_values() {
                if group.len() > 1 {
                    for idx in group {
                        issues[idx] |= Issue::CASE_CLASH;
                    }
                }
            }
        }
    }
}

/// FIPS 180-4 SHA-1 returning the first 4 lowercase hex characters (matches 1.x `SHA1.HashData` truncation).
pub fn sha1_hex4(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 80];
    for chunk in padded.chunks_exact(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    format!("{:04x}", h[0] >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = r"C:\data";

    fn f(rel: &str) -> Node {
        Node::new(rel, false)
    }
    fn d(rel: &str) -> Node {
        Node::new(rel, true)
    }
    fn run(nodes: &[Node]) -> Plan {
        Planner::build(ROOT, nodes, CLASSIC_LIMIT, FixKinds::ALL)
    }

    fn after(nodes: &[Node], plan: &Plan) -> Vec<String> {
        let ops: HashMap<&str, &RenameOp> = plan.ops.iter().map(|o| (o.rel.as_str(), o)).collect();
        let new_names: HashMap<&str, &str> = nodes
            .iter()
            .map(|n| {
                let name = ops
                    .get(n.rel.as_str())
                    .map(|o| o.new_name.as_str())
                    .unwrap_or(n.name());
                (n.rel.as_str(), name)
            })
            .collect();
        nodes
            .iter()
            .map(|n| {
                let mut parts = Vec::new();
                let mut cur = n.rel.as_str();
                while !cur.is_empty() {
                    parts.push(new_names[cur]);
                    cur = match cur.rfind('\\') {
                        Some(p) => &cur[..p],
                        None => "",
                    };
                }
                parts.reverse();
                parts.join("\\")
            })
            .collect()
    }

    fn deep_tree(dirs: usize, dir_len: usize, file_len: usize, ext: &str) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut cur = String::new();
        for i in 0..dirs {
            let seg: String = std::iter::repeat_n((b'a' + i as u8) as char, dir_len).collect();
            cur = if cur.is_empty() {
                seg
            } else {
                format!("{cur}\\{seg}")
            };
            nodes.push(d(&cur));
        }
        let fname: String = std::iter::repeat_n('f', file_len).collect();
        nodes.push(f(&format!("{cur}\\{fname}{ext}")));
        nodes
    }

    #[test]
    fn sha1_matches_nist_vectors() {
        assert_eq!(sha1_hex4(b"abc"), "a999");
        assert_eq!(sha1_hex4(b""), "da39");
    }

    #[test]
    fn clean_tree_has_no_findings() {
        let nodes = [
            d("src"),
            f(r"src\main.c"),
            f("readme.md"),
            f("a b (1).txt"),
            f("CON1.txt"),
            f("console.log"),
            f("COM10.txt"),
        ];
        let p = run(&nodes);
        assert!(p.findings.is_empty());
        assert!(p.ops.is_empty());
        assert_eq!(p.scanned, 7);
    }

    #[test]
    fn only_chosen_kinds_are_fixed() {
        let nodes = [f("a?.txt"), f("b."), f("CON.txt")];
        let p = Planner::build(ROOT, &nodes, CLASSIC_LIMIT, FixKinds::RESERVED_NAMES);
        assert_eq!(
            p.ops
                .iter()
                .map(|o| o.old_name.as_str())
                .collect::<Vec<_>>(),
            vec!["CON.txt"]
        );
        assert_eq!(p.findings.len(), 3);
        assert!(Planner::build(ROOT, &nodes, CLASSIC_LIMIT, FixKinds::NONE)
            .ops
            .is_empty());
    }

    #[test]
    fn case_clashes_reported_on_both_and_second_numbered() {
        let nodes = [f("Readme.md"), f("README.md"), f("other.txt")];
        let p = run(&nodes);
        assert_eq!(
            p.findings
                .iter()
                .filter(|x| x.issues.contains(Issue::CASE_CLASH))
                .count(),
            2
        );
        assert_eq!(
            p.ops
                .iter()
                .find(|o| o.rel == "Readme.md")
                .unwrap()
                .new_name,
            "Readme (2).md"
        );
        assert!(!p.ops.iter().any(|o| o.rel == "README.md"));
        let without_case = Planner::build(
            ROOT,
            &nodes,
            CLASSIC_LIMIT,
            FixKinds::ALL & !FixKinds::CASE_CLASHES,
        );
        assert!(without_case.ops.is_empty());
    }

    #[test]
    fn cleaning_collisions_preserve_existing_clean_name() {
        let nodes = [f("a?.txt"), f("a_.txt"), f("a*.txt")];
        let p = Planner::build(ROOT, &nodes, CLASSIC_LIMIT, FixKinds::ILLEGAL_CHARS);
        let names = after(&nodes, &p);
        let uniq: HashSet<_> = names.iter().collect();
        assert_eq!(uniq.len(), 3);
        assert!(names.contains(&"a_.txt".to_string()));
        assert!(names.contains(&"a_ (2).txt".to_string()));
    }

    #[test]
    fn folders_are_cleaned_and_children_follow() {
        let nodes = [
            d("old."),
            f(r"old.\file.txt"),
            d(r"old.\sub?"),
            f(r"old.\sub?\x.txt"),
        ];
        let p = run(&nodes);
        assert_eq!(
            p.ops
                .iter()
                .map(|o| o.new_name.as_str())
                .collect::<Vec<_>>(),
            vec!["old", "sub_"]
        );
        assert_eq!(
            after(&nodes, &p),
            vec!["old", r"old\file.txt", r"old\sub_", r"old\sub_\x.txt"]
        );
    }

    #[test]
    fn long_filename_is_cut_first_keeping_extension() {
        let nodes = deep_tree(4, 30, 150, ".dat");
        let p = run(&nodes);
        assert!(p
            .findings
            .iter()
            .any(|x| x.issues.contains(Issue::TOO_LONG)));
        assert!(!p.ops.iter().any(|o| o.is_directory));
        let out = after(&nodes, &p);
        for rel in &out {
            assert!(ROOT.len() + 1 + utf16_len(rel) <= 259, "{rel}");
        }
        assert!(out.last().unwrap().ends_with(".dat"));
        assert!(out.last().unwrap().contains('~'));
    }

    #[test]
    fn folders_cut_when_filename_alone_cannot_fix() {
        let nodes = deep_tree(8, 40, 10, ".dat");
        let p = run(&nodes);
        let out = after(&nodes, &p);
        for rel in &out {
            assert!(ROOT.len() + 1 + utf16_len(rel) <= 259);
        }
        assert!(p.ops.iter().any(|o| o.is_directory));
        assert_eq!(p.unresolved_count, 0);
        assert_eq!(run(&nodes).ops, p.ops);
    }

    #[test]
    fn siblings_sharing_long_prefix_stay_unique() {
        let prefix = "x".repeat(60);
        let dir = "d".repeat(100);
        let mut nodes = vec![d(&dir)];
        for i in 0..20 {
            nodes.push(f(&format!("{dir}\\{prefix}{i}{prefix}.txt")));
        }
        let p = Planner::build(ROOT, &nodes, 150, FixKinds::ALL);
        let out = after(&nodes, &p);
        let upper: HashSet<_> = out.iter().map(|s| s.to_uppercase()).collect();
        assert_eq!(upper.len(), 21);
        for rel in &out {
            assert!(ROOT.len() + 1 + utf16_len(rel) <= 150);
        }
    }

    #[test]
    fn impossible_limit_is_reported_honestly() {
        let nodes = deep_tree(6, 40, 10, ".dat");
        let p = Planner::build(ROOT, &nodes, 40, FixKinds::ALL);
        assert!(p.unresolved_count > 0);
        let msg = p
            .findings
            .iter()
            .find_map(|x| x.unresolved.as_deref())
            .unwrap();
        assert!(msg.contains("Move the folder"));
    }

    #[test]
    fn names_over_255_are_cut_even_in_short_path() {
        let name = format!("{}.log", "n".repeat(300));
        let nodes = [f(&name)];
        let p = run(&nodes);
        assert!(p.findings[0].issues.contains(Issue::NAME_TOO_LONG));
        let cut_name = &p.ops[0].new_name;
        assert!(utf16_len(cut_name) <= 255);
        assert!(cut_name.ends_with(".log"));
    }

    #[test]
    fn emoji_name_over_255_utf16_units_cuts_safely_on_char_boundary() {
        let name = format!("{}.txt", "😀".repeat(130)); // 264 UTF-16 code units
        let nodes = [f(&name)];
        let p = run(&nodes);
        assert!(p.findings[0].issues.contains(Issue::NAME_TOO_LONG));
        let cut_name = &p.ops[0].new_name;
        assert!(utf16_len(cut_name) <= 255);
        assert!(cut_name.ends_with(".txt"));
    }

    #[test]
    fn randomized_trees_end_clean_and_unique() {
        let bits = [
            "a",
            "B",
            "con",
            "NUL",
            "x.",
            " y",
            "q?",
            "ä",
            "Ä",
            "e\u{0301}",
            "é",
            &format!("long{}", "z".repeat(70)),
            "dot.",
            "OK name",
            "COM3",
        ];
        // Simple deterministic LCG matching 40 rounds x 60 nodes
        let mut seed = 7u64;
        let mut next = || -> usize {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };

        for _round in 0..40 {
            let mut nodes: Vec<Node> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            let mut dirs = vec![String::new()];
            for _ in 0..60 {
                let parent = dirs[next() % dirs.len()].clone();
                let b1 = bits[next() % bits.len()];
                let b2 = if next() % 3 == 0 {
                    ""
                } else {
                    bits[next() % bits.len()]
                };
                let ext = if next() % 2 == 0 { ".txt" } else { "" };
                let name = format!("{b1}{b2}{ext}");
                let rel = if parent.is_empty() {
                    name
                } else {
                    format!("{parent}\\{name}")
                };
                if !seen.insert(rel.clone()) {
                    continue;
                }
                let is_dir = next() % 3 == 0 && rel.len() < 120;
                if is_dir {
                    dirs.push(rel.clone());
                }
                nodes.push(Node::new(rel, is_dir));
            }

            let p = Planner::build(ROOT, &nodes, 140, FixKinds::ALL);
            let out = after(&nodes, &p);
            let folded: HashSet<_> = out.iter().map(|s| rules::fold(s)).collect();
            assert_eq!(folded.len(), nodes.len());

            for (n, rel) in nodes.iter().zip(out.iter()) {
                let name = rel.rsplit_once('\\').map(|(_, r)| r).unwrap_or(rel);
                assert_eq!(rules::check(name), Issue::NONE, "{name}");
                let is_unres = p
                    .findings
                    .iter()
                    .any(|f| f.node.rel == n.rel && f.unresolved.is_some());
                if !is_unres {
                    assert!(ROOT.len() + 1 + utf16_len(rel) <= 140, "{rel}");
                }
            }
        }
    }
}
