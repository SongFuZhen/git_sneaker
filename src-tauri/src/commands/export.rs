use std::path::Path;

use crate::engine::bundle;
use crate::engine::commit::{self, Commit};
use crate::engine::repo::{self, RepoInfo, SyncPoint};
use crate::error::SneakerError;

#[derive(serde::Serialize)]
pub struct ExportPreview {
    pub repo: RepoInfo,
    pub commits: Vec<Commit>,
    pub range: String,
}

#[tauri::command]
pub async fn open_repo(path: String) -> Result<RepoInfo, String> {
    repo::open(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_commits(repo_path: String, limit: Option<usize>) -> Result<Vec<Commit>, String> {
    let repo = git2::Repository::open(&repo_path)
        .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;
    commit::list_all(&repo, limit.unwrap_or(100)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_unpushed_commits(repo_path: String) -> Result<Vec<Commit>, String> {
    let repo = git2::Repository::open(&repo_path)
        .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "main".to_string());
    let last_sync = repo::get_last_sync_tag(&repo, &branch).map_err(|e| e.to_string())?;
    let from = last_sync
        .map(|s| s.tag_name)
        .unwrap_or_else(|| "HEAD".to_string());
    commit::list_range(&repo, &from, "HEAD").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_last_sync(repo_path: String) -> Result<Option<SyncPoint>, String> {
    let repo = git2::Repository::open(&repo_path)
        .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "main".to_string());
    repo::get_last_sync_tag(&repo, &branch).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_export(repo_path: String) -> Result<ExportPreview, String> {
    let repo_path = Path::new(&repo_path);
    let repo_info = repo::open(repo_path).map_err(|e| e.to_string())?;
    let git_repo = git2::Repository::open(repo_path)
        .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;

    let base = repo_info
        .last_sync
        .as_ref()
        .map(|s| s.tag_name.clone())
        .unwrap_or_else(|| "HEAD".to_string());

    let range = format!("{}..HEAD", base);
    let commits = commit::list_range(&git_repo, &base, "HEAD").map_err(|e| e.to_string())?;

    Ok(ExportPreview {
        repo: repo_info,
        commits,
        range,
    })
}

#[tauri::command]
pub async fn exec_export(repo_path: String, output_dir: String, from: Option<String>) -> Result<bundle::ExportResult, String> {
    let repo_path = Path::new(&repo_path);
    let output_dir = Path::new(&output_dir);

    let repo_info = repo::open(repo_path).map_err(|e| e.to_string())?;
    let branch = repo_info.head_branch.clone();

    let range = if let Some(ref from_hash) = from {
        format!("{}..HEAD", from_hash)
    } else {
        // Full export - bundle entire branch
        "HEAD".to_string()
    };

    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let filename = format!("sneaker-{}.bundle", ts);
    let bundle_path = output_dir.join(&filename);

    let mut result = bundle::create(repo_path, &range, &bundle_path).map_err(|e| e.to_string())?;

    // Only create sync tag for incremental exports
    if from.is_some() {
        let git_repo = git2::Repository::open(repo_path)
            .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;
        let sync = repo::create_sync_tag(&git_repo, &branch).map_err(|e| e.to_string())?;
        result.sync_tag = sync.tag_name;
    }

    Ok(result)
}
