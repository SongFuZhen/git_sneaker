use std::path::Path;

use crate::engine::bundle::{self, BundleInfo};
use crate::merge::merge::{self, MergeResult};

#[tauri::command]
pub async fn verify_bundle(bundle_path: String) -> Result<BundleInfo, String> {
    bundle::verify(Path::new(&bundle_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn exec_import(repo_path: String, bundle_path: String) -> Result<MergeResult, String> {
    let repo_path = Path::new(&repo_path);
    let bundle_path = Path::new(&bundle_path);

    let repo = git2::Repository::open(repo_path)
        .map_err(|e| crate::error::SneakerError::GitError(e.message().to_string()).to_string())?;
    let head = repo
        .head()
        .map_err(|e| crate::error::SneakerError::GitError(e.message().to_string()).to_string())?;
    let branch = head.shorthand().unwrap_or("main").to_string();

    merge::pull_bundle(repo_path, bundle_path, &branch).map_err(|e| e.to_string())
}
