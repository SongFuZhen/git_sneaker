use std::path::Path;

use crate::engine::bundle::{self, BundleInfo};
use crate::merge::merge::{self, MergeResult};

#[tauri::command]
pub async fn verify_bundle(bundle_path: String, repo_path: Option<String>) -> Result<BundleInfo, String> {
    let repo = repo_path.as_deref().map(Path::new);
    bundle::verify(Path::new(&bundle_path), repo).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn exec_import(repo_path: String, bundle_path: String) -> Result<MergeResult, String> {
    let repo_path = Path::new(&repo_path);
    let bundle_path = Path::new(&bundle_path);

    // Get branch name from bundle
    let branch = bundle::get_branch_name(bundle_path).map_err(|e| e.to_string())?;

    merge::pull_bundle(repo_path, bundle_path, &branch).map_err(|e| e.to_string())
}
