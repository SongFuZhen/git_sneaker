use std::collections::HashSet;

use serde::Serialize;

use crate::merge::conflict::{ConflictFile, ConflictHunk, HunkDecision, ResolvedHunk};

#[derive(Debug, Clone, Serialize)]
pub struct AutoResolveReport {
    pub resolved: Vec<ResolvedHunk>,
    pub manual_hunks: Vec<usize>,
    pub summary: String,
}

const TRAILER_KEYS: &[&str] = &[
    "signed-off-by:",
    "reviewed-by:",
    "acked-by:",
    "tested-by:",
    "reported-by:",
    "co-authored-by:",
    "cc:",
];

/// Analyze conflicts and attempt auto-resolution.
///
/// # Hunk ID contract
///
/// This assigns **per-file** hunk IDs starting from 0 for each file,
/// matching the IDs produced by [`crate::merge::conflict::scan_conflicts`].
/// Resolved hunks can be passed directly to [`crate::merge::conflict::apply_resolution`].
pub fn analyze(conflicts: &[ConflictFile]) -> AutoResolveReport {
    let mut resolved = Vec::new();
    let mut manual = Vec::new();

    for file in conflicts {
        for hunk in &file.hunks {
            // Use the hunk's own ID (per-file, starting from 0)
            let result = try_resolve(hunk, hunk.id);
            match result {
                Some(r) => resolved.push(r),
                None => manual.push(hunk.id),
            }
        }
    }

    let auto_count = resolved.len();
    let manual_count = manual.len();
    let total = auto_count + manual_count;
    AutoResolveReport {
        resolved,
        manual_hunks: manual,
        summary: format!(
            "{}/{} hunks auto-resolved, {} need manual review",
            auto_count,
            total,
            manual_count
        ),
    }
}

fn try_resolve(hunk: &ConflictHunk, id: usize) -> Option<ResolvedHunk> {
    // Pattern 1: Both-Add-Same
    let l = hunk.local.trim();
    let r = hunk.remote.trim();
    if l == r && !l.is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: hunk.local.clone(),
            auto: true,
            confidence: 1.0,
        });
    }

    // Pattern 3: One-Sided-Delete (must check before Pattern 2 — Non-Overlapping)
    let b = hunk.base.trim();
    if !b.is_empty() && hunk.local.trim().is_empty() && r == b {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: String::new(),
            auto: true,
            confidence: 0.98,
        });
    }
    if !b.is_empty() && hunk.remote.trim().is_empty() && l == b {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeRemote,
            merged_content: String::new(),
            auto: true,
            confidence: 0.98,
        });
    }

    // Pattern 2: Non-Overlapping — one side empty
    if hunk.local.trim().is_empty() && !hunk.remote.trim().is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeRemote,
            merged_content: hunk.remote.clone(),
            auto: true,
            confidence: 1.0,
        });
    }
    if hunk.remote.trim().is_empty() && !hunk.local.trim().is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: hunk.local.clone(),
            auto: true,
            confidence: 1.0,
        });
    }

    // Pattern 4: Whitespace-Only
    let ln = normalize(&hunk.local);
    let rn = normalize(&hunk.remote);
    if ln == rn {
        let better = if hunk.local.lines().count() <= hunk.remote.lines().count() {
            &hunk.local
        } else {
            &hunk.remote
        };
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::Custom(better.clone()),
            merged_content: better.clone(),
            auto: true,
            confidence: 0.95,
        });
    }

    // Pattern 5: Trailer-Lines
    if is_trailer_only(&hunk.local, &hunk.remote, &hunk.base) {
        let merged = merge_trailers(&hunk.local, &hunk.remote);
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::Custom(merged.clone()),
            merged_content: merged,
            auto: true,
            confidence: 1.0,
        });
    }

    None
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_trailer_key(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    TRAILER_KEYS.iter().any(|k| lower.starts_with(k))
}

fn is_trailer_only(local: &str, remote: &str, base: &str) -> bool {
    let local_body = strip_trailers(local);
    let remote_body = strip_trailers(remote);
    let base_body = strip_trailers(base);

    if local_body == remote_body && local_body == base_body {
        let lt = extract_trailers(local);
        let rt = extract_trailers(remote);
        return !lt.is_empty() || !rt.is_empty();
    }
    false
}

fn extract_trailers(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| is_trailer_key(l))
        .map(|l| l.to_string())
        .collect()
}

fn strip_trailers(s: &str) -> String {
    s.lines()
        .filter(|l| !is_trailer_key(l))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn merge_trailers(local: &str, remote: &str) -> String {
    let body = strip_trailers(local);
    let mut result = body;
    let mut seen = HashSet::new();
    let mut has_trailers = false;

    for line in local.lines().chain(remote.lines()) {
        let trimmed = line.trim();
        if is_trailer_key(trimmed) {
            let key = trimmed.to_lowercase();
            if seen.insert(key) {
                if !has_trailers {
                    // Ensure newline between body and first trailer
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    has_trailers = true;
                }
                result.push_str(trimmed);
                result.push('\n');
            }
        }
    }
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hunk(id: usize, local: &str, base: &str, remote: &str) -> ConflictHunk {
        ConflictHunk {
            id,
            local: local.to_string(),
            base: base.to_string(),
            remote: remote.to_string(),
            line_range: (1, 3),
        }
    }

    #[test]
    fn test_both_add_same() {
        let h = make_hunk(0, "foo();\n", "", "foo();\n");
        let r = try_resolve(&h, 0).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_non_overlapping_local_empty() {
        let h = make_hunk(0, "", "", "new_fn();\n");
        let r = try_resolve(&h, 0).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
        assert_eq!(r.merged_content, "new_fn();\n");
    }

    #[test]
    fn test_non_overlapping_remote_empty() {
        let h = make_hunk(0, "new_fn();\n", "", "");
        let r = try_resolve(&h, 0).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
        assert_eq!(r.merged_content, "new_fn();\n");
    }

    #[test]
    fn test_one_sided_delete() {
        let h = make_hunk(1, "", "old\n", "old\n");
        let r = try_resolve(&h, 1).unwrap();
        assert!((r.confidence - 0.98).abs() < 0.01);
        assert!(r.merged_content.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let h = make_hunk(
            2,
            "  foo();\n  bar();\n",
            "foo();\nbar();\n",
            "foo();\nbar();\n",
        );
        let r = try_resolve(&h, 2).unwrap();
        assert!((r.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_non_resolvable() {
        let h = make_hunk(3, "fn a() {}\n", "fn old() {}\n", "fn b() {}\n");
        assert!(try_resolve(&h, 3).is_none());
    }

    #[test]
    fn test_trailer_conflict() {
        let h = make_hunk(
            4,
            "fn foo() {}\n\nSigned-off-by: A <a@x.com>\n",
            "fn foo() {}\n",
            "fn foo() {}\n\nSigned-off-by: B <b@x.com>\n",
        );
        let r = try_resolve(&h, 4).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
        assert!(r.merged_content.contains("Signed-off-by: A"));
        assert!(r.merged_content.contains("Signed-off-by: B"));
    }
}
