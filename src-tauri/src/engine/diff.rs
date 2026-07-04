use std::cell::RefCell;

use git2::{DiffOptions, Repository};
use serde::Serialize;

use crate::error::SneakerError;

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
    fn test_diff_commits_single_file() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        // Create initial commit
        std::fs::write(dir.path().join("a.txt"), "line1\nline2\n").unwrap();
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

        // Modify file
        std::fs::write(dir.path().join("a.txt"), "line1\nmodified\nline3\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "modify a.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        let diffs = diff_commits(&repo, "HEAD~1", "HEAD").unwrap();

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "a.txt");
        assert!(!diffs[0].hunks.is_empty());
    }

    #[test]
    fn test_diff_commits_new_file() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        // Initial commit with one file
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

        // Add new file
        std::fs::write(dir.path().join("b.txt"), "new content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add b.txt"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        let diffs = diff_commits(&repo, "HEAD~1", "HEAD").unwrap();

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "b.txt");
        assert!(diffs[0].status.contains("Added"));
    }

    #[test]
    fn test_diff_commits_no_changes() {
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

        let repo = Repository::open(dir.path()).unwrap();
        let diffs = diff_commits(&repo, "HEAD", "HEAD").unwrap();

        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_line_types() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        std::fs::write(dir.path().join("a.txt"), "keep\nold\n").unwrap();
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

        std::fs::write(dir.path().join("a.txt"), "keep\nnew\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "modify"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        let diffs = diff_commits(&repo, "HEAD~1", "HEAD").unwrap();

        assert_eq!(diffs.len(), 1);
        let hunk = &diffs[0].hunks[0];
        assert!(!hunk.lines.is_empty());

        // Should have context, deletion, and addition
        let has_context = hunk.lines.iter().any(|l| matches!(l, DiffLine::Context(_)));
        let has_deletion = hunk.lines.iter().any(|l| matches!(l, DiffLine::Deletion(_)));
        let has_addition = hunk.lines.iter().any(|l| matches!(l, DiffLine::Addition(_)));

        assert!(has_context, "should have context lines");
        assert!(has_deletion, "should have deletion lines");
        assert!(has_addition, "should have addition lines");
    }

    #[test]
    fn test_diff_invalid_ref() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        let repo = Repository::open(dir.path()).unwrap();
        let result = diff_commits(&repo, "nonexistent", "HEAD");

        assert!(result.is_err());
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub hunks: Vec<Hunk>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

#[allow(dead_code)]
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
