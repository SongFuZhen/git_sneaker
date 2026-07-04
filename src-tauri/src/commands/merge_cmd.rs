use std::path::Path;

use crate::merge::auto_resolve::{self, AutoResolveReport};
use crate::merge::conflict::{self, ConflictFile, ResolvedHunk};
use crate::merge::merge;

#[tauri::command]
pub async fn get_conflicts(repo_path: String) -> Result<Vec<ConflictFile>, String> {
    conflict::scan_conflicts(Path::new(&repo_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn auto_resolve_conflicts(
    _repo_path: String,
    conflicts: Vec<ConflictFile>,
) -> Result<AutoResolveReport, String> {
    Ok(auto_resolve::analyze(&conflicts))
}

#[tauri::command]
pub async fn apply_resolution(
    repo_path: String,
    file_path: String,
    hunks: Vec<ResolvedHunk>,
) -> Result<(), String> {
    conflict::apply_resolution(Path::new(&repo_path), &file_path, &hunks)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn commit_merge(repo_path: String, message: String) -> Result<(), String> {
    conflict::commit_merge(Path::new(&repo_path), &message).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn abort_merge(repo_path: String) -> Result<(), String> {
    merge::abort_merge(Path::new(&repo_path)).map_err(|e| e.to_string())
}
