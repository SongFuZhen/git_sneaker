use std::path::Path;

use chrono::Utc;
use git2::Repository;
use serde::Serialize;

use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize)]
pub struct RepoInfo {
    pub path: String,
    pub head_branch: String,
    pub head_commit: String,
    pub last_sync: Option<SyncPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncPoint {
    pub tag_name: String,
    pub commit: String,
    pub timestamp: i64,
}

pub fn open(path: &Path) -> Result<RepoInfo, SneakerError> {
    if !path.exists() {
        return Err(SneakerError::RepoNotFound(path.display().to_string()));
    }

    let repo = Repository::open(path)
        .map_err(|_| SneakerError::NotAGitRepo(path.display().to_string()))?;

    let head = repo.head().map_err(|e| SneakerError::from(e))?;
    let branch = head.shorthand().unwrap_or("HEAD").to_string();
    let head_commit = head.peel_to_commit().map_err(|e| SneakerError::from(e))?;
    let short_hash = head_commit
        .as_object()
        .short_id()
        .map_err(|e| SneakerError::from(e))?;
    let short_hash_str = short_hash
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let last_sync = get_last_sync_tag(&repo, &branch)?;

    Ok(RepoInfo {
        path: path.display().to_string(),
        head_branch: branch.clone(),
        head_commit: short_hash_str,
        last_sync,
    })
}

pub fn get_last_sync_tag(
    repo: &Repository,
    branch: &str,
) -> Result<Option<SyncPoint>, SneakerError> {
    let mut latest: Option<SyncPoint> = None;

    let tags = repo.tag_names(None).map_err(|e| SneakerError::from(e))?;
    for tag_name in tags.iter().flatten() {
        let prefix = format!("sneaker-sync/{}/", branch);
        if !tag_name.starts_with(&prefix) {
            continue;
        }
        let tag_ref = repo.find_reference(&format!("refs/tags/{}", tag_name));
        if let Ok(tag_ref) = tag_ref {
            if let Ok(commit) = tag_ref.peel_to_commit() {
                let ts = commit.time().seconds();
                if latest.is_none() || ts > latest.as_ref().unwrap().timestamp {
                    let short = commit.as_object().short_id().map_err(|e| SneakerError::from(e))?;
                    let short_str =
                        short.as_str().unwrap_or("").to_string();
                    latest = Some(SyncPoint {
                        tag_name: tag_name.to_string(),
                        commit: short_str,
                        timestamp: ts,
                    });
                }
            }
        }
    }

    Ok(latest)
}

pub fn create_sync_tag(repo: &Repository, branch: &str) -> Result<SyncPoint, SneakerError> {
    let now = Utc::now();
    let ts_str = now.format("%Y-%m-%dT%H%M%S%z").to_string();
    let tag_name = format!("sneaker-sync/{}/{}", branch, ts_str);

    let head = repo.head().map_err(|e| SneakerError::from(e))?;
    let head_commit = head.peel_to_commit().map_err(|e| SneakerError::from(e))?;
    let obj = head_commit.as_object();

    repo.tag_lightweight(&tag_name, obj, false)
        .map_err(|e| SneakerError::GitError(e.message().to_string()))?;

    let short = head_commit.as_object().short_id().map_err(|e| SneakerError::from(e))?;
    let short_str = short.as_str().unwrap_or("").to_string();

    Ok(SyncPoint {
        tag_name,
        commit: short_str,
        timestamp: head_commit.time().seconds(),
    })
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
        std::fs::write(dir.join("README.md"), "# test").unwrap();
        Command::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn test_open_repo() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());
        let info = open(dir.path()).unwrap();
        assert!(info.head_branch.contains("main") || info.head_branch.contains("master"));
        assert!(info.last_sync.is_none());
    }

    #[test]
    fn test_create_and_find_sync_tag() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        let branch = repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();

        assert!(get_last_sync_tag(&repo, &branch).unwrap().is_none());

        let sync = create_sync_tag(&repo, &branch).unwrap();
        assert!(sync.tag_name.starts_with(&format!("sneaker-sync/{}/", branch)));

        let found = get_last_sync_tag(&repo, &branch).unwrap().unwrap();
        assert_eq!(found.commit, sync.commit);
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let result = open(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }
}
