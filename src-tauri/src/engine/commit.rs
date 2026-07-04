use git2::{Oid, Repository, Sort};
use serde::Serialize;

use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize)]
pub struct Commit {
    pub hash: String,
    pub full_hash: String,
    pub message: String,
    pub author: String,
    pub date: i64,
    pub date_str: String,
}

fn format_date(timestamp: i64) -> String {
    let dt = chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_default();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn make_commit(commit: &git2::Commit) -> Result<Commit, SneakerError> {
    let oid = commit.id();
    let short = commit
        .as_object()
        .short_id()
        .map_err(|e| SneakerError::from(e))?;
    let short_str = short
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let message = commit
        .message()
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    let author = commit.author();
    let author_name = author.name().unwrap_or("unknown").to_string();
    let timestamp = commit.time().seconds();

    Ok(Commit {
        hash: short_str,
        full_hash: oid.to_string(),
        message,
        author: author_name,
        date: timestamp,
        date_str: format_date(timestamp),
    })
}

pub fn list_all(repo: &Repository, limit: usize) -> Result<Vec<Commit>, SneakerError> {
    let mut revwalk = repo.revwalk().map_err(|e| SneakerError::from(e))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| SneakerError::from(e))?;
    revwalk.push_head().map_err(|e| SneakerError::from(e))?;

    let mut commits = Vec::new();

    for (i, oid_result) in revwalk.enumerate() {
        if i >= limit {
            break;
        }
        let oid = oid_result.map_err(|e| SneakerError::from(e))?;
        let commit = repo.find_commit(oid).map_err(|e| SneakerError::from(e))?;
        commits.push(make_commit(&commit)?);
    }

    Ok(commits)
}

pub fn list_range(
    repo: &Repository,
    from: &str,
    to: &str,
) -> Result<Vec<Commit>, SneakerError> {
    let to_oid = resolve_ref(repo, to)?;
    let from_oid = resolve_ref(repo, from)?;

    let mut revwalk = repo.revwalk().map_err(|e| SneakerError::from(e))?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| SneakerError::from(e))?;
    revwalk.push(to_oid).map_err(|e| SneakerError::from(e))?;
    revwalk.hide(from_oid).map_err(|e| SneakerError::from(e))?;

    let mut commits = Vec::new();

    for oid_result in revwalk {
        let oid = oid_result.map_err(|e| SneakerError::from(e))?;
        let commit = repo.find_commit(oid).map_err(|e| SneakerError::from(e))?;
        commits.push(make_commit(&commit)?);
    }

    Ok(commits)
}

fn resolve_ref(repo: &Repository, name: &str) -> Result<Oid, SneakerError> {
    if let Ok(oid) = Oid::from_str(name) {
        return Ok(oid);
    }

    let resolved = repo
        .revparse_single(name)
        .map_err(|_| SneakerError::GitError(format!("cannot resolve ref: {}", name)))?;
    Ok(resolved.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn init_test_repo(dir: &std::path::Path) {
        Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
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
        for i in 1..=3 {
            std::fs::write(dir.join("file.txt"), format!("line {}", i)).unwrap();
            Command::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
            let msg = format!("commit {}", i);
            Command::new("git")
                .args(["commit", "-m", &msg])
                .current_dir(dir)
                .output()
                .unwrap();
        }
    }

    #[test]
    fn test_list_range_has_commits() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());
        let repo = Repository::open(dir.path()).unwrap();

        let commits = list_range(&repo, "HEAD~2", "HEAD").unwrap();
        assert!(!commits.is_empty());
        assert!(commits.len() <= 2);
    }
}
