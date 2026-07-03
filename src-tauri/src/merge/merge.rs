use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum MergeResult {
    Success,
    Conflicted { files: Vec<String> },
    AlreadyUpToDate,
}

pub fn pull_bundle(
    repo: &Path,
    bundle_path: &Path,
    branch: &str,
) -> Result<MergeResult, SneakerError> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo.display().to_string(),
            "pull",
            "--no-edit",
            &bundle_path.display().to_string(),
            branch,
        ])
        .output()
        .map_err(|e| SneakerError::MergeFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    if combined.contains("Already up to date") || combined.contains("Already up-to-date") {
        return Ok(MergeResult::AlreadyUpToDate);
    }

    if output.status.success() {
        return Ok(MergeResult::Success);
    }

    let status = Command::new("git")
        .args(["-C", &repo.display().to_string(), "diff", "--name-only", "--diff-filter=U"])
        .output()
        .map_err(|e| SneakerError::MergeFailed(e.to_string()))?;

    let status_str = String::from_utf8_lossy(&status.stdout);
    let conflicted: Vec<String> = status_str
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    if !conflicted.is_empty() {
        Ok(MergeResult::Conflicted { files: conflicted })
    } else {
        Err(SneakerError::MergeFailed(stderr.to_string()))
    }
}

pub fn abort_merge(repo: &Path) -> Result<(), SneakerError> {
    let output = Command::new("git")
        .args(["-C", &repo.display().to_string(), "merge", "--abort"])
        .output()
        .map_err(|e| SneakerError::GitError(e.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(SneakerError::MergeFailed(stderr.to_string()))
    }
}
