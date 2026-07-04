use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    pub path: String,
    pub hunks: Vec<ConflictHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub id: usize,
    pub local: String,
    pub base: String,
    pub remote: String,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedHunk {
    pub hunk_id: usize,
    pub decision: HunkDecision,
    pub merged_content: String,
    pub auto: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum HunkDecision {
    TakeLocal,
    TakeRemote,
    Custom(String),
}

fn get_unmerged_files(repo: &Path) -> Result<Vec<String>, SneakerError> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo.display().to_string(),
            "diff",
            "--name-only",
            "--diff-filter=U",
        ])
        .output()
        .map_err(|e| SneakerError::GitError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
}

fn parse_conflict_markers(content: &str) -> Vec<ConflictHunk> {
    let mut hunks = Vec::new();
    let mut hunk_id = 0;
    let mut in_conflict = false;
    let mut in_base = false;
    let mut in_remote = false;
    let mut local_lines: Vec<String> = Vec::new();
    let mut base_lines: Vec<String> = Vec::new();
    let mut remote_lines: Vec<String> = Vec::new();
    let mut conflict_start = 0;

    for (i, line) in content.lines().enumerate() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            conflict_start = i + 1;
            local_lines.clear();
            base_lines.clear();
            remote_lines.clear();
            in_base = false;
            in_remote = false;
        } else if line.starts_with("|||||||") && in_conflict {
            in_base = true;
        } else if line.starts_with("=======") && in_conflict {
            if in_base {
                in_base = false;
            }
            in_remote = true;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            in_remote = false;
            hunks.push(ConflictHunk {
                id: hunk_id,
                local: local_lines.join("\n"),
                base: base_lines.join("\n"),
                remote: remote_lines.join("\n"),
                line_range: (conflict_start, i + 1),
            });
            hunk_id += 1;
        } else if in_conflict {
            if in_base {
                base_lines.push(line.to_string());
            } else if in_remote {
                remote_lines.push(line.to_string());
            } else {
                local_lines.push(line.to_string());
            }
        }
    }

    hunks
}

pub fn scan_conflicts(repo: &Path) -> Result<Vec<ConflictFile>, SneakerError> {
    let unmerged = get_unmerged_files(repo)?;
    let mut conflicts = Vec::new();

    for file_path in &unmerged {
        let full_path = repo.join(file_path);
        let content =
            fs::read_to_string(&full_path).map_err(|e| SneakerError::FileNotFound(e.to_string()))?;
        let hunks = parse_conflict_markers(&content);
        conflicts.push(ConflictFile {
            path: file_path.clone(),
            hunks,
        });
    }

    Ok(conflicts)
}

pub fn apply_resolution(
    repo: &Path,
    file_path: &str,
    resolved: &[ResolvedHunk],
) -> Result<(), SneakerError> {
    let full_path = repo.join(file_path);
    let content =
        fs::read_to_string(&full_path).map_err(|e| SneakerError::FileNotFound(e.to_string()))?;

    let decision_map: HashMap<usize, &ResolvedHunk> =
        resolved.iter().map(|r| (r.hunk_id, r)).collect();

    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;
    let mut current_hunk = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("<<<<<<<") {
            let mut end = i + 1;
            while end < lines.len() && !lines[end].starts_with(">>>>>>>") {
                end += 1;
            }
            if end >= lines.len() {
                // Unclosed conflict marker — preserve everything
                for j in i..lines.len() {
                    result.push(lines[j].to_string());
                }
                break;
            }
            if let Some(resolved_hunk) = decision_map.get(&current_hunk) {
                if !resolved_hunk.merged_content.is_empty() {
                    for content_line in resolved_hunk.merged_content.lines() {
                        result.push(content_line.to_string());
                    }
                }
            } else {
                // Keep original conflict markers to prevent data loss
                for j in i..=end {
                    result.push(lines[j].to_string());
                }
            }
            i = end + 1;
            current_hunk += 1;
        } else {
            result.push(line.to_string());
            i += 1;
        }
    }

    let mut new_content = result.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }

    fs::write(&full_path, &new_content)
        .map_err(|e| SneakerError::PermissionDenied(e.to_string()))?;

    let add = Command::new("git")
        .args(["-C", &repo.display().to_string(), "add", file_path])
        .output()
        .map_err(|e| SneakerError::GitError(e.to_string()))?;

    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        return Err(SneakerError::GitError(stderr.to_string()));
    }

    Ok(())
}

pub fn commit_merge(repo: &Path, message: &str) -> Result<(), SneakerError> {
    let output = Command::new("git")
        .args(["-C", &repo.display().to_string(), "commit", "-m", message])
        .output()
        .map_err(|e| SneakerError::GitError(e.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(SneakerError::MergeFailed(stderr.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn init_test_repo(dir: &std::path::Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn test_parse_single_conflict() {
        let content = "line1\n<<<<<<< HEAD\nlocal change\n=======\nremote change\n>>>>>>> branch\nline3\n";
        let hunks = parse_conflict_markers(content);

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].id, 0);
        assert_eq!(hunks[0].local, "local change");
        assert_eq!(hunks[0].base, "");
        assert_eq!(hunks[0].remote, "remote change");
    }

    #[test]
    fn test_parse_multiple_conflicts() {
        let content = "<<<<<<< HEAD\na\n=======\nb\n>>>>>>>\nmiddle\n<<<<<<< HEAD\nc\n=======\nd\n>>>>>>>\n";
        let hunks = parse_conflict_markers(content);

        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].id, 0);
        assert_eq!(hunks[1].id, 1);
    }

    #[test]
    fn test_parse_conflict_with_base() {
        let content = "<<<<<<< HEAD\nnew local\n||||||| base version\nold content\n=======\nnew remote\n>>>>>>>\n";
        let hunks = parse_conflict_markers(content);

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].local, "new local");
        assert_eq!(hunks[0].base, "old content");
        assert_eq!(hunks[0].remote, "new remote");
    }

    #[test]
    fn test_parse_no_conflict() {
        let content = "just normal text\nno conflicts here\n";
        let hunks = parse_conflict_markers(content);

        assert!(hunks.is_empty());
    }

    #[test]
    fn test_apply_resolution_take_local() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        // Create initial commit
        std::fs::write(dir.path().join("conflict.txt"), "base\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Create conflict
        let conflict_content = "<<<<<<< HEAD\nlocal\n=======\nremote\n>>>>>>>\n";
        std::fs::write(dir.path().join("conflict.txt"), conflict_content).unwrap();
        Command::new("git")
            .args(["add", "conflict.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let resolved = vec![ResolvedHunk {
            hunk_id: 0,
            decision: HunkDecision::TakeLocal,
            merged_content: "local".to_string(),
            auto: false,
            confidence: 1.0,
        }];

        apply_resolution(dir.path(), "conflict.txt", &resolved).unwrap();

        let result = std::fs::read_to_string(dir.path().join("conflict.txt")).unwrap();
        assert_eq!(result, "local\n");
    }

    #[test]
    fn test_apply_resolution_take_remote() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("conflict.txt"), "base\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let conflict_content = "<<<<<<< HEAD\nlocal\n=======\nremote\n>>>>>>>\n";
        std::fs::write(dir.path().join("conflict.txt"), conflict_content).unwrap();
        Command::new("git")
            .args(["add", "conflict.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let resolved = vec![ResolvedHunk {
            hunk_id: 0,
            decision: HunkDecision::TakeRemote,
            merged_content: "remote".to_string(),
            auto: false,
            confidence: 1.0,
        }];

        apply_resolution(dir.path(), "conflict.txt", &resolved).unwrap();

        let result = std::fs::read_to_string(dir.path().join("conflict.txt")).unwrap();
        assert_eq!(result, "remote\n");
    }

    #[test]
    fn test_apply_resolution_custom() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("conflict.txt"), "base\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let conflict_content = "<<<<<<< HEAD\nlocal\n=======\nremote\n>>>>>>>\n";
        std::fs::write(dir.path().join("conflict.txt"), conflict_content).unwrap();
        Command::new("git")
            .args(["add", "conflict.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let resolved = vec![ResolvedHunk {
            hunk_id: 0,
            decision: HunkDecision::Custom("custom merge".to_string()),
            merged_content: "custom merge".to_string(),
            auto: false,
            confidence: 1.0,
        }];

        apply_resolution(dir.path(), "conflict.txt", &resolved).unwrap();

        let result = std::fs::read_to_string(dir.path().join("conflict.txt")).unwrap();
        assert_eq!(result, "custom merge\n");
    }

    #[test]
    fn test_apply_resolution_empty_content() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("conflict.txt"), "base\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let conflict_content = "<<<<<<< HEAD\nlocal\n=======\nremote\n>>>>>>>\n";
        std::fs::write(dir.path().join("conflict.txt"), conflict_content).unwrap();
        Command::new("git")
            .args(["add", "conflict.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let resolved = vec![ResolvedHunk {
            hunk_id: 0,
            decision: HunkDecision::TakeLocal,
            merged_content: String::new(),
            auto: false,
            confidence: 1.0,
        }];

        apply_resolution(dir.path(), "conflict.txt", &resolved).unwrap();

        let result = std::fs::read_to_string(dir.path().join("conflict.txt")).unwrap();
        assert!(result.is_empty() || result == "\n");
    }

    #[test]
    fn test_commit_merge() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Make a change and stage it
        std::fs::write(dir.path().join("a.txt"), "modified").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let result = commit_merge(dir.path(), "test merge commit");
        assert!(result.is_ok());

        // Verify commit was created
        let log = Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log_str = String::from_utf8_lossy(&log.stdout);
        assert!(log_str.contains("test merge commit"));
    }

    #[test]
    fn test_commit_merge_no_staged_changes() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let result = commit_merge(dir.path(), "empty commit");
        assert!(result.is_err());
    }
}
