use std::cell::RefCell;

use git2::{DiffOptions, Repository};
use serde::Serialize;

use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

pub fn diff_commits(
    repo: &Repository,
    from: &str,
    to: &str,
) -> Result<Vec<FileDiff>, SneakerError> {
    let from_obj = repo
        .revparse_single(from)
        .map_err(|e| SneakerError::GitError(e.message().to_string()))?;
    let to_obj = repo
        .revparse_single(to)
        .map_err(|e| SneakerError::GitError(e.message().to_string()))?;

    let from_tree = from_obj.peel_to_tree().map_err(|e| SneakerError::from(e))?;
    let to_tree = to_obj.peel_to_tree().map_err(|e| SneakerError::from(e))?;

    let mut opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut opts))
        .map_err(|e| SneakerError::from(e))?;

    let files = RefCell::new(Vec::new());

    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let status = format!("{:?}", delta.status());
            files.borrow_mut().push(FileDiff {
                path,
                status,
                hunks: Vec::new(),
            });
            true
        },
        None,
        Some(&mut |_delta, hunk| {
            if let Some(file) = files.borrow_mut().last_mut() {
                let header = format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start(),
                    hunk.old_lines(),
                    hunk.new_start(),
                    hunk.new_lines(),
                );
                file.hunks.push(Hunk {
                    old_start: hunk.old_start() as u32,
                    old_lines: hunk.old_lines() as u32,
                    new_start: hunk.new_start() as u32,
                    new_lines: hunk.new_lines() as u32,
                    header,
                    lines: Vec::new(),
                });
            }
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            if let Some(file) = files.borrow_mut().last_mut() {
                if let Some(hunk) = file.hunks.last_mut() {
                    let content = String::from_utf8_lossy(line.content()).to_string();
                    let diff_line = match line.origin() {
                        '+' => DiffLine::Addition(content),
                        '-' => DiffLine::Deletion(content),
                        _ => DiffLine::Context(content),
                    };
                    hunk.lines.push(diff_line);
                }
            }
            true
        }),
    )
    .map_err(|e| SneakerError::GitError(e.message().to_string()))?;

    Ok(files.into_inner())
}
