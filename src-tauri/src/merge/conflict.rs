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
