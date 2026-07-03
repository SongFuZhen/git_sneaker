# GitSneaker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build offline Git sync tool — bundle export to USB, import + merge on target machine, 3-way conflict resolver with auto-detection

**Architecture:** Tauri 2 shell wrapping a Rust core engine (git2-rs + git CLI hybrid) with Petite-Vue frontend. Three-layer: Engine (pure Rust) → Commands (Tauri handlers) → Frontend (Petite-Vue). Engine modules have zero Tauri dependency.

**Tech Stack:** Tauri 2, Rust (git2 0.19, serde, thiserror 2, chrono 0.4), Petite-Vue 0.4, vanilla HTML/CSS

## Global Constraints

- Engine modules (`engine/`, `merge/`) MUST NOT import `tauri` — pure Rust, `cargo test` directly
- Command modules (`commands/`) only convert params and call engine functions
- Frontend calls backend ONLY through `api.js` invoke() wrappers, never `__TAURI__` directly
- Bundle operations (`create`, `verify`, `pull`) use git CLI via `std::process::Command`
- Revwalk, diff, ref/tag operations use git2-rs
- Sync tag format: `sneaker-sync/<branch>/<ISO8601>`
- Config stored in `.sneaker.toml` at repo root, NOT registry or ~/.config
- Target binary <10MB (lto=true, opt-level="z", strip=true)
- Petite-Vue bundled locally (offline-capable), no CDN
- All errors through `SneakerError` enum with Serialize

---

## File Map

```
git-sneaker/
├── Cargo.toml                              # Workspace root
├── .gitignore
├── src-tauri/
│   ├── Cargo.toml                          # Tauri app deps
│   ├── tauri.conf.json                     # Tauri 2 config
│   ├── build.rs                            # Tauri build script
│   ├── capabilities/
│   │   └── default.json                    # Permissions
│   ├── icons/                              # App icons (placeholder)
│   └── src/
│       ├── main.rs                         # Entry: calls lib::run()
│       ├── lib.rs                          # Tauri builder + command registration
│       ├── error.rs                        # SneakerError enum
│       ├── engine/
│       │   ├── mod.rs
│       │   ├── repo.rs                     # RepoInfo, SyncPoint, open(), get/create_sync_tag()
│       │   ├── commit.rs                   # Commit struct, list_range()
│       │   ├── bundle.rs                   # BundleInfo, create(), verify()
│       │   └── diff.rs                     # FileDiff, Hunk, diff_commits()
│       ├── merge/
│       │   ├── mod.rs
│       │   ├── merge.rs                    # MergeResult, pull_bundle(), abort_merge()
│       │   ├── conflict.rs                 # ConflictFile, ConflictHunk, scan/apply/commit
│       │   └── auto_resolve.rs             # AutoResolveReport, analyze() with 5 patterns
│       └── commands/
│           ├── mod.rs
│           ├── export.rs                   # open_repo, preview_export, exec_export
│           ├── import.rs                   # verify_bundle, exec_import
│           └── merge_cmd.rs               # get_conflicts, auto_resolve_conflicts, apply, commit, abort
├── src/                                    # Frontend (Petite-Vue)
│   ├── index.html                          # Single-page app shell
│   ├── js/
│   │   ├── vendor/
│   │   │   └── petite-vue.js               # Petite-Vue 0.4 (local, no CDN)
│   │   ├── app.js                          # Global helpers (selectRepo, etc.)
│   │   ├── api.js                          # Tauri invoke() wrappers
│   │   └── views/
│   │       ├── export.js                   # Export view component
│   │       ├── import.js                   # Import view component
│   │       └── conflict.js                 # Conflict resolve view component
│   ├── css/
│   │   └── style.css                       # All styles (Catppuccin Mocha theme)
│   └── assets/
│       └── icon.svg
```

---

### Task 1: Scaffold Tauri 2 project skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/error.rs` (stub)
- Create: `src-tauri/src/engine/mod.rs`
- Create: `src-tauri/src/merge/mod.rs`
- Create: `src-tauri/src/commands/mod.rs`

**Produces:** `cargo build` succeeds with empty Tauri app + full module tree

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["src-tauri"]
resolver = "2"
```

- [ ] **Step 2: Create .gitignore**

```
target/
node_modules/
*.bundle
.DS_Store
```

- [ ] **Step 3: Create src-tauri/Cargo.toml**

```toml
[package]
name = "git-sneaker"
version = "0.1.0"
edition = "2021"

[lib]
name = "git_sneaker_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
git2 = "0.19"
chrono = "0.4"

[dev-dependencies]
tempfile = "3"

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "z"
strip = true
```

- [ ] **Step 4: Create src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicedoc/schemas/main/schemas/tauri/config.schema.json",
  "productName": "GitSneaker",
  "identifier": "com.gitsneaker.app",
  "version": "0.1.0",
  "build": {
    "frontendDist": "../src",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "",
    "beforeBuildCommand": ""
  },
  "app": {
    "windows": [
      {
        "title": "GitSneaker",
        "width": 1100,
        "height": 750,
        "minWidth": 900,
        "minHeight": 600
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "app", "msi", "deb", "appimage"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 5: Create src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 6: Create src-tauri/capabilities/default.json**

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "shell:default"
  ]
}
```

- [ ] **Step 7: Create src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    git_sneaker_lib::run()
}
```

- [ ] **Step 8: Create src-tauri/src/lib.rs**

```rust
mod commands;
mod engine;
mod error;
mod merge;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Commands registered in later tasks
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitSneaker");
}
```

- [ ] **Step 9: Create stub module files**

`src-tauri/src/error.rs`:
```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum SneakerError {
    #[error("Generic error: {0}")]
    Generic(String),
}
```

`src-tauri/src/engine/mod.rs`:
```rust
pub mod bundle;
pub mod commit;
pub mod diff;
pub mod repo;
```

`src-tauri/src/merge/mod.rs`:
```rust
pub mod auto_resolve;
pub mod conflict;
pub mod merge;
```

`src-tauri/src/commands/mod.rs`:
```rust
pub mod export;
pub mod import;
pub mod merge_cmd;
```

Create empty placeholder files for each module referenced in mod.rs:
```bash
for f in src-tauri/src/engine/{bundle,commit,diff,repo}.rs \
         src-tauri/src/merge/{merge,conflict,auto_resolve}.rs \
         src-tauri/src/commands/{export,import,merge_cmd}.rs; do
  echo "// placeholder" > "$f"
done
```

- [ ] **Step 10: Verify build compiles**

Run: `cargo build 2>&1`
Expected: Compiles successfully (warnings about dead code acceptable).

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml .gitignore src-tauri/
git commit -m "feat: scaffold Tauri 2 project with complete module tree"
```

---

### Task 2: Define SneakerError enum

**Files:**
- Modify: `src-tauri/src/error.rs`

**Produces:** Full `SneakerError` with Serialize, all spec variants, `From<git2::Error>` and `From<String>` impls

- [ ] **Step 1: Replace error.rs stub**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum SneakerError {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Not a git repository: {0}")]
    NotAGitRepo(String),

    #[error("Working tree is dirty: {0}")]
    DirtyWorktree(String),

    #[error("Failed to create bundle: {0}")]
    BundleCreateFailed(String),

    #[error("Failed to verify bundle: {0}")]
    BundleVerifyFailed(String),

    #[error("Bundle is corrupted: {0}")]
    BundleCorrupted(String),

    #[error("Merge failed: {0}")]
    MergeFailed(String),

    #[error("Merge was aborted")]
    MergeAborted,

    #[error("Unresolved conflicts remain in: {0:?}")]
    UnresolvedConflicts(Vec<String>),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("No space left on {path}: available={available}, needed={needed}")]
    NoSpaceLeft {
        path: String,
        available: u64,
        needed: u64,
    },

    #[error("Git error: {0}")]
    GitError(String),

    #[error("Git is not installed or version < 2.25")]
    GitNotAvailable,

    #[error("{0}")]
    Generic(String),
}

impl From<git2::Error> for SneakerError {
    fn from(e: git2::Error) -> Self {
        SneakerError::GitError(e.message().to_string())
    }
}

impl From<String> for SneakerError {
    fn from(s: String) -> Self {
        SneakerError::Generic(s)
    }
}
```

- [ ] **Step 2: Verify build**

Run: `cargo build 2>&1`
Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/error.rs
git commit -m "feat: add full SneakerError enum with all spec variants"
```

---

### Task 3: engine::repo — repository info and sync tags

**Files:**
- Modify: `src-tauri/src/engine/repo.rs`

**Interfaces:**
- Produces: `RepoInfo`, `SyncPoint` structs + `open()`, `get_last_sync_tag()`, `create_sync_tag()`

- [ ] **Step 1: Write engine/repo.rs**

```rust
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
    let short_hash_str = std::str::from_utf8(short_hash.as_bytes())
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
                    let short = commit.as_object().short_id().unwrap();
                    let short_str =
                        std::str::from_utf8(short.as_bytes()).unwrap_or("").to_string();
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

    let short = head_commit.as_object().short_id().unwrap();
    let short_str = std::str::from_utf8(short.as_bytes()).unwrap_or("").to_string();

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
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib engine::repo::tests`
Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/engine/repo.rs
git commit -m "feat: add engine::repo — RepoInfo, SyncPoint, open, get/create sync tag"
```

---

### Task 4: engine::commit — commit range listing via revwalk

**Files:**
- Modify: `src-tauri/src/engine/commit.rs`

**Interfaces:**
- Produces: `Commit` struct + `list_range(repo, from, to) -> Vec<Commit>`

- [ ] **Step 1: Write engine/commit.rs**

```rust
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

        let short = commit
            .as_object()
            .short_id()
            .map_err(|e| SneakerError::from(e))?;
        let short_str = std::str::from_utf8(short.as_bytes())
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

        commits.push(Commit {
            hash: short_str,
            full_hash: oid.to_string(),
            message,
            author: author_name,
            date: commit.time().seconds(),
        });
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
            Command::new("git")
                .args(["commit", "-m", format!("commit {}", i)])
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
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib engine::commit::tests`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/engine/commit.rs
git commit -m "feat: add engine::commit — list_range via git2-rs revwalk"
```

---

### Task 5: engine::bundle — create and verify via git CLI

**Files:**
- Modify: `src-tauri/src/engine/bundle.rs`

**Interfaces:**
- Produces: `BundleInfo`, `ExportResult` + `create()`, `verify()`

- [ ] **Step 1: Write engine/bundle.rs**

```rust
use std::path::Path;
use std::process::Command as ShellCommand;

use serde::Serialize;

use crate::engine::commit::Commit;
use crate::error::SneakerError;

#[derive(Debug, Clone, Serialize)]
pub struct BundleInfo {
    pub head_commit: String,
    pub head_message: String,
    pub commits: Vec<Commit>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub file_path: String,
    pub file_size: u64,
    pub sync_tag: String,
}

pub fn create(repo: &Path, range: &str, output: &Path) -> Result<ExportResult, SneakerError> {
    let child = ShellCommand::new("git")
        .args([
            "-C",
            &repo.display().to_string(),
            "bundle",
            "create",
            &output.display().to_string(),
            range,
        ])
        .output()
        .map_err(|e| SneakerError::BundleCreateFailed(e.to_string()))?;

    if !child.status.success() {
        let stderr = String::from_utf8_lossy(&child.stderr);
        return Err(SneakerError::BundleCreateFailed(stderr.to_string()));
    }

    let metadata = std::fs::metadata(output)
        .map_err(|e| SneakerError::FileNotFound(e.to_string()))?;

    Ok(ExportResult {
        file_path: output.display().to_string(),
        file_size: metadata.len(),
        sync_tag: String::new(),
    })
}

pub fn verify(bundle_path: &Path) -> Result<BundleInfo, SneakerError> {
    if !bundle_path.exists() {
        return Err(SneakerError::FileNotFound(bundle_path.display().to_string()));
    }

    let verify_output = ShellCommand::new("git")
        .args(["bundle", "verify", &bundle_path.display().to_string()])
        .output()
        .map_err(|e| SneakerError::BundleVerifyFailed(e.to_string()))?;

    if !verify_output.status.success() {
        let stderr = String::from_utf8_lossy(&verify_output.stderr);
        return Err(SneakerError::BundleVerifyFailed(stderr.to_string()));
    }

    let verify_stdout = String::from_utf8_lossy(&verify_output.stdout);
    let head_commit = verify_stdout
        .lines()
        .find(|l| l.len() == 40 && l.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s[..7].to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let metadata = std::fs::metadata(bundle_path)
        .map_err(|e| SneakerError::FileNotFound(e.to_string()))?;

    Ok(BundleInfo {
        head_commit,
        head_message: String::new(),
        commits: Vec::new(),
        file_size: metadata.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn init_repo(dir: &std::path::Path) {
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
        std::fs::write(dir.join("a.txt"), "v1").unwrap();
        Command::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
        Command::new("git")
            .args(["commit", "-m", "c1"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn test_create_and_verify_bundle() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let bundle_path = dir.path().join("test.bundle");

        let result = create(dir.path(), "HEAD", &bundle_path).unwrap();
        assert!(result.file_size > 0);
        assert!(bundle_path.exists());

        let info = verify(&bundle_path).unwrap();
        assert!(info.file_size > 0);
        assert!(!info.head_commit.is_empty());
    }

    #[test]
    fn test_verify_nonexistent_bundle() {
        let result = verify(Path::new("/no/such/bundle"));
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib engine::bundle::tests`
Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/engine/bundle.rs
git commit -m "feat: add engine::bundle — create and verify via git CLI"
```

---

### Task 6: engine::diff — structured diff via git2-rs

**Files:**
- Modify: `src-tauri/src/engine/diff.rs`

**Interfaces:**
- Produces: `FileDiff`, `Hunk`, `DiffLine` + `diff_commits(repo, from, to) -> Vec<FileDiff>`

- [ ] **Step 1: Write engine/diff.rs**

```rust
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

    let mut files = Vec::new();

    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let status = format!("{:?}", delta.status());
            files.push(FileDiff {
                path,
                status,
                hunks: Vec::new(),
            });
            true
        },
        None,
        Some(&mut |_delta, hunk, _| {
            if let Some(file) = files.last_mut() {
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
        Some(&mut |_delta, _hunk, line, _| {
            if let Some(file) = files.last_mut() {
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

    Ok(files)
}
```

- [ ] **Step 2: Build check**

Run: `cargo build 2>&1`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/engine/diff.rs
git commit -m "feat: add engine::diff — structured diff via git2-rs tree diff"
```

---

### Task 7: merge::merge — pull bundle and detect state

**Files:**
- Modify: `src-tauri/src/merge/merge.rs`

**Interfaces:**
- Produces: `MergeResult` enum + `pull_bundle(repo, bundle_path, branch)`, `abort_merge(repo)`

- [ ] **Step 1: Write merge/merge.rs**

```rust
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
```

- [ ] **Step 2: Build check**

Run: `cargo build 2>&1`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/merge/merge.rs
git commit -m "feat: add merge::merge — pull_bundle and abort via git CLI"
```

---

### Task 8: merge::conflict — parse conflict markers and apply resolutions

**Files:**
- Modify: `src-tauri/src/merge/conflict.rs`

**Interfaces:**
- Produces: `ConflictFile`, `ConflictHunk`, `ResolvedHunk`, `HunkDecision` + `scan_conflicts()`, `apply_resolution()`, `commit_merge()`

- [ ] **Step 1: Write merge/conflict.rs**

```rust
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
            if let Some(resolved_hunk) = decision_map.get(&current_hunk) {
                if !resolved_hunk.merged_content.is_empty() {
                    for content_line in resolved_hunk.merged_content.lines() {
                        result.push(content_line.to_string());
                    }
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
```

- [ ] **Step 2: Build check**

Run: `cargo build 2>&1`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/merge/conflict.rs
git commit -m "feat: add merge::conflict — scan markers, apply resolution, commit merge"
```

---

### Task 9: merge::auto_resolve — 5-pattern deterministic conflict resolver

**Files:**
- Modify: `src-tauri/src/merge/auto_resolve.rs`

**Interfaces:**
- Produces: `AutoResolveReport` + `analyze(conflicts) -> AutoResolveReport`

- [ ] **Step 1: Write merge/auto_resolve.rs**

```rust
use std::collections::HashSet;

use serde::Serialize;

use crate::merge::conflict::{ConflictFile, ConflictHunk, HunkDecision, ResolvedHunk};

#[derive(Debug, Clone, Serialize)]
pub struct AutoResolveReport {
    pub resolved: Vec<ResolvedHunk>,
    pub manual_hunks: Vec<usize>,
    pub summary: String,
}

const TRAILER_KEYS: &[&str] = &[
    "signed-off-by:",
    "reviewed-by:",
    "acked-by:",
    "tested-by:",
    "reported-by:",
    "co-authored-by:",
    "cc:",
];

pub fn analyze(conflicts: &[ConflictFile]) -> AutoResolveReport {
    let mut resolved = Vec::new();
    let mut manual = Vec::new();
    let mut hunk_id = 0;

    for file in conflicts {
        for hunk in &file.hunks {
            let result = try_resolve(hunk, hunk_id);
            match result {
                Some(r) => resolved.push(r),
                None => manual.push(hunk_id),
            }
            hunk_id += 1;
        }
    }

    let total = resolved.len() + manual.len();
    AutoResolveReport {
        resolved,
        manual_hunks: manual,
        summary: format!(
            "{}/{} hunks auto-resolved, {} need manual review",
            resolved.len(),
            total,
            manual.len()
        ),
    }
}

fn try_resolve(hunk: &ConflictHunk, id: usize) -> Option<ResolvedHunk> {
    // Pattern 1: Both-Add-Same
    let l = hunk.local.trim();
    let r = hunk.remote.trim();
    if l == r && !l.is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: hunk.local.clone(),
            auto: true,
            confidence: 1.0,
        });
    }

    // Pattern 2: Non-Overlapping — one side empty
    if hunk.local.trim().is_empty() && !hunk.remote.trim().is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeRemote,
            merged_content: hunk.remote.clone(),
            auto: true,
            confidence: 1.0,
        });
    }
    if hunk.remote.trim().is_empty() && !hunk.local.trim().is_empty() {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: hunk.local.clone(),
            auto: true,
            confidence: 1.0,
        });
    }

    // Pattern 3: One-Sided-Delete
    let b = hunk.base.trim();
    if !b.is_empty() && hunk.local.trim().is_empty() && r == b {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeLocal,
            merged_content: String::new(),
            auto: true,
            confidence: 0.98,
        });
    }
    if !b.is_empty() && hunk.remote.trim().is_empty() && l == b {
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::TakeRemote,
            merged_content: String::new(),
            auto: true,
            confidence: 0.98,
        });
    }

    // Pattern 4: Whitespace-Only
    let ln = normalize(&hunk.local);
    let rn = normalize(&hunk.remote);
    if ln == rn {
        let better = if hunk.local.lines().count() <= hunk.remote.lines().count() {
            &hunk.local
        } else {
            &hunk.remote
        };
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::Custom(better.clone()),
            merged_content: better.clone(),
            auto: true,
            confidence: 0.95,
        });
    }

    // Pattern 5: Trailer-Lines
    if is_trailer_only(&hunk.local, &hunk.remote, &hunk.base) {
        let merged = merge_trailers(&hunk.local, &hunk.remote);
        return Some(ResolvedHunk {
            hunk_id: id,
            decision: HunkDecision::Custom(merged.clone()),
            merged_content,
            auto: true,
            confidence: 1.0,
        });
    }

    None
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_trailer_key(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    TRAILER_KEYS.iter().any(|k| lower.starts_with(k))
}

fn is_trailer_only(local: &str, remote: &str, base: &str) -> bool {
    let local_body = strip_trailers(local);
    let remote_body = strip_trailers(remote);
    let base_body = strip_trailers(base);

    if local_body == remote_body && local_body == base_body {
        let lt = extract_trailers(local);
        let rt = extract_trailers(remote);
        return !lt.is_empty() || !rt.is_empty();
    }
    false
}

fn extract_trailers(s: &str) -> Vec<String> {
    s.lines().filter(|l| is_trailer_key(l)).map(|l| l.to_string()).collect()
}

fn strip_trailers(s: &str) -> String {
    s.lines()
        .filter(|l| !is_trailer_key(l))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn merge_trailers(local: &str, remote: &str) -> String {
    let body = strip_trailers(local);
    let mut result = body;
    if !result.is_empty() {
        result.push('\n');
    }
    let mut seen = HashSet::new();
    for line in local.lines().chain(remote.lines()) {
        let trimmed = line.trim();
        if is_trailer_key(trimmed) {
            let key = trimmed.to_lowercase();
            if seen.insert(key) {
                result.push_str(line);
                result.push('\n');
            }
        }
    }
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hunk(id: usize, local: &str, base: &str, remote: &str) -> ConflictHunk {
        ConflictHunk {
            id,
            local: local.to_string(),
            base: base.to_string(),
            remote: remote.to_string(),
            line_range: (1, 3),
        }
    }

    #[test]
    fn test_both_add_same() {
        let h = make_hunk(0, "foo();\n", "", "foo();\n");
        let r = try_resolve(&h, 0).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_one_sided_delete() {
        let h = make_hunk(1, "", "old\n", "old\n");
        let r = try_resolve(&h, 1).unwrap();
        assert!((r.confidence - 0.98).abs() < 0.01);
        assert!(r.merged_content.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let h = make_hunk(2, "  foo();\n  bar();\n", "foo();\nbar();\n", "foo();\nbar();\n");
        let r = try_resolve(&h, 2).unwrap();
        assert!((r.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_non_resolvable() {
        let h = make_hunk(3, "fn a() {}\n", "fn old() {}\n", "fn b() {}\n");
        assert!(try_resolve(&h, 3).is_none());
    }

    #[test]
    fn test_trailer_conflict() {
        let h = make_hunk(
            4,
            "fn foo() {}\n\nSigned-off-by: A <a@x.com>\n",
            "fn foo() {}\n",
            "fn foo() {}\n\nSigned-off-by: B <b@x.com>\n",
        );
        let r = try_resolve(&h, 4).unwrap();
        assert!((r.confidence - 1.0).abs() < 0.01);
        assert!(r.merged_content.contains("Signed-off-by: A"));
        assert!(r.merged_content.contains("Signed-off-by: B"));
    }
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test --lib merge::auto_resolve::tests`
Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/merge/auto_resolve.rs
git commit -m "feat: add merge::auto_resolve — 5-pattern deterministic resolver with tests"
```

---

### Task 10: Tauri commands — export handlers

**Files:**
- Modify: `src-tauri/src/commands/export.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)

**Interfaces:**
- Produces: `open_repo`, `get_unpushed_commits`, `get_last_sync`, `preview_export`, `exec_export` Tauri commands

- [ ] **Step 1: Write commands/export.rs**

```rust
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
pub async fn exec_export(repo_path: String, output_dir: String) -> Result<crate::engine::bundle::ExportResult, String> {
    let repo_path = Path::new(&repo_path);
    let output_dir = Path::new(&output_dir);

    let repo_info = repo::open(repo_path).map_err(|e| e.to_string())?;
    let branch = repo_info.head_branch.clone();

    let base = repo_info
        .last_sync
        .as_ref()
        .map(|s| s.tag_name.clone())
        .unwrap_or_else(|| "HEAD".to_string());

    let range = format!("{}..HEAD", base);

    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let filename = format!("sneaker-{}.bundle", ts);
    let bundle_path = output_dir.join(&filename);

    let mut result = bundle::create(repo_path, &range, &bundle_path).map_err(|e| e.to_string())?;

    let git_repo = git2::Repository::open(repo_path)
        .map_err(|e| SneakerError::GitError(e.message().to_string()).to_string())?;
    let sync = repo::create_sync_tag(&git_repo, &branch).map_err(|e| e.to_string())?;
    result.sync_tag = sync.tag_name;

    Ok(result)
}
```

- [ ] **Step 2: Register commands in lib.rs**

Update the `invoke_handler` in `src-tauri/src/lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    commands::export::open_repo,
    commands::export::get_unpushed_commits,
    commands::export::get_last_sync,
    commands::export::preview_export,
    commands::export::exec_export,
])
```

- [ ] **Step 3: Build check**

Run: `cargo build 2>&1`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/export.rs src-tauri/src/lib.rs
git commit -m "feat: add export Tauri commands — open_repo, preview, exec_export"
```

---

### Task 11: Tauri commands — import handlers

**Files:**
- Modify: `src-tauri/src/commands/import.rs`
- Modify: `src-tauri/src/lib.rs` (add to invoke_handler)

- [ ] **Step 1: Write commands/import.rs**

```rust
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
```

- [ ] **Step 2: Add to invoke_handler in lib.rs**

Append:
```rust
    commands::import::verify_bundle,
    commands::import::exec_import,
```

- [ ] **Step 3: Build check + commit**

Run: `cargo build 2>&1`

```bash
git add src-tauri/src/commands/import.rs src-tauri/src/lib.rs
git commit -m "feat: add import Tauri commands — verify_bundle, exec_import"
```

---

### Task 12: Tauri commands — merge handlers

**Files:**
- Modify: `src-tauri/src/commands/merge_cmd.rs`
- Modify: `src-tauri/src/lib.rs` (add to invoke_handler)

- [ ] **Step 1: Write commands/merge_cmd.rs**

```rust
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
```

- [ ] **Step 2: Add to invoke_handler in lib.rs**

Append:
```rust
    commands::merge_cmd::get_conflicts,
    commands::merge_cmd::auto_resolve_conflicts,
    commands::merge_cmd::apply_resolution,
    commands::merge_cmd::commit_merge,
    commands::merge_cmd::abort_merge,
```

- [ ] **Step 3: Build check + commit**

Run: `cargo build 2>&1`

```bash
git add src-tauri/src/commands/merge_cmd.rs src-tauri/src/lib.rs
git commit -m "feat: add merge Tauri commands — get_conflicts, auto_resolve, apply, commit, abort"
```

---

### Task 13: Frontend scaffold — HTML, Petite-Vue, CSS, api.js

**Files:**
- Create: `src/index.html`
- Create: `src/js/vendor/petite-vue.js` (downloaded)
- Create: `src/js/api.js`
- Create: `src/js/app.js`
- Create: `src/css/style.css`
- Create: `src/assets/icon.svg`

- [ ] **Step 1: Download Petite-Vue locally**

Run:
```bash
mkdir -p src/js/vendor src/js/views src/css src/assets
curl -L -o src/js/vendor/petite-vue.js https://unpkg.com/petite-vue@0.4.1/dist/petite-vue.iife.js
```

If curl fails (air-gapped dev), create a note to download manually and create a minimal placeholder.

- [ ] **Step 2: Create src/assets/icon.svg**

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect width="128" height="128" rx="24" fill="#1e1e2e"/>
  <text x="64" y="82" text-anchor="middle" font-size="64" font-family="monospace" fill="#89b4fa">GS</text>
</svg>
```

- [ ] **Step 3: Create src/css/style.css**

```css
:root {
    --bg: #1e1e2e;
    --surface: #313244;
    --text: #cdd6f4;
    --subtext: #a6adc8;
    --accent: #89b4fa;
    --green: #a6e3a1;
    --red: #f38ba8;
    --yellow: #f9e2af;
    --border: #45475a;
    --radius: 6px;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace;
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
    line-height: 1.5;
    overflow: hidden;
    height: 100vh;
}

#app { display: flex; flex-direction: column; height: 100vh; }

.app-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag;
}

.app-title { font-size: 16px; font-weight: 700; color: var(--accent); }

.nav-tabs { display: flex; gap: 4px; -webkit-app-region: no-drag; }
.nav-tabs button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--subtext);
    padding: 4px 16px;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 12px;
}
.nav-tabs button.active { background: var(--accent); color: var(--bg); border-color: var(--accent); }

main { flex: 1; overflow-y: auto; padding: 16px; }

.status-bar {
    padding: 4px 16px;
    background: var(--surface);
    border-top: 1px solid var(--border);
    color: var(--subtext);
    font-size: 11px;
}

.panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    margin-bottom: 12px;
}

.panel-header {
    font-size: 12px;
    font-weight: 600;
    color: var(--subtext);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.btn {
    padding: 6px 16px;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
}
.btn-primary { background: var(--accent); color: var(--bg); }
.btn-danger { background: var(--red); color: var(--bg); }
.btn-success { background: var(--green); color: var(--bg); }
.btn-secondary { background: var(--surface); color: var(--text); border: 1px solid var(--border); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }

input[type="text"] {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 6px 10px;
    border-radius: var(--radius);
    font-size: 12px;
    width: 100%;
}

.commit-list { max-height: 300px; overflow-y: auto; }
.commit-item {
    display: flex;
    gap: 8px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
}
.commit-hash { color: var(--yellow); font-family: monospace; min-width: 70px; }
.commit-msg { flex: 1; }
.commit-author { color: var(--subtext); }

.diff-threeway {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 1px;
    background: var(--border);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    margin-bottom: 12px;
}
.diff-panel { background: var(--bg); overflow: auto; max-height: 300px; }
.diff-panel-header {
    padding: 4px 8px;
    background: var(--surface);
    font-size: 11px;
    font-weight: 600;
    color: var(--subtext);
    position: sticky;
    top: 0;
}
.diff-panel-content {
    padding: 8px;
    font-family: monospace;
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-all;
}

.merge-result {
    min-height: 120px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px;
    font-family: monospace;
    font-size: 12px;
    white-space: pre-wrap;
}

.conflict-file-tabs { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 12px; }
.conflict-file-tab {
    padding: 4px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 12px;
}
.conflict-file-tab.active { background: var(--accent); color: var(--bg); }
.conflict-file-tab.resolved { border-color: var(--green); }
.conflict-file-tab.resolved::after { content: ' \2713'; color: var(--green); }

.toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 12px; }
.toolbar .spacer { flex: 1; }

.auto-report {
    background: var(--surface);
    border: 1px solid var(--green);
    border-radius: var(--radius);
    padding: 12px;
    margin-bottom: 12px;
}
.auto-report-header { color: var(--green); font-weight: 600; margin-bottom: 4px; }

.path-display { color: var(--subtext); font-size: 11px; word-break: break-all; }
```

- [ ] **Step 4: Create src/js/api.js**

```javascript
// api.js — Tauri invoke() wrappers
const api = {
    openRepo: (path) => window.__TAURI__.core.invoke('open_repo', { path }),
    getUnpushed: (repoPath) => window.__TAURI__.core.invoke('get_unpushed_commits', { repoPath }),
    getLastSync: (repoPath) => window.__TAURI__.core.invoke('get_last_sync', { repoPath }),

    previewExport: (repoPath) => window.__TAURI__.core.invoke('preview_export', { repoPath }),
    execExport: (repoPath, outputDir) =>
        window.__TAURI__.core.invoke('exec_export', { repoPath, outputDir }),

    verifyBundle: (bundlePath) => window.__TAURI__.core.invoke('verify_bundle', { bundlePath }),
    execImport: (repoPath, bundlePath) =>
        window.__TAURI__.core.invoke('exec_import', { repoPath, bundlePath }),

    getConflicts: (repoPath) => window.__TAURI__.core.invoke('get_conflicts', { repoPath }),
    autoResolve: (repoPath, conflicts) =>
        window.__TAURI__.core.invoke('auto_resolve_conflicts', { repoPath, conflicts }),
    applyResolution: (repoPath, filePath, hunks) =>
        window.__TAURI__.core.invoke('apply_resolution', { repoPath, filePath, hunks }),
    commitMerge: (repoPath, message) =>
        window.__TAURI__.core.invoke('commit_merge', { repoPath, message }),
    abortMerge: (repoPath) => window.__TAURI__.core.invoke('abort_merge', { repoPath }),
};
```

- [ ] **Step 5: Create src/js/app.js**

```javascript
// app.js — shared helpers
const app = {
    setStatus(msg) { this.statusText = msg; },
    showError(err) {
        this.statusText = 'Error: ' + err;
        console.error(err);
    },

    async selectRepo() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: 'Select Git Repository',
            });
            if (selected) {
                this.repoPath = selected;
                this.repoInfo = await api.openRepo(selected);
                this.setStatus('Repository: ' + this.repoInfo.head_branch);
            }
        } catch (e) { this.showError(e); }
    },

    async selectBundleFile() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                filters: [{ name: 'Git Bundle', extensions: ['bundle'] }],
                title: 'Select Bundle File',
            });
            if (selected) {
                this.bundlePath = selected;
                this.bundleInfo = await api.verifyBundle(selected);
                this.setStatus('Bundle loaded: ' + this.bundleInfo.head_commit);
            }
        } catch (e) { this.showError(e); }
    },
};
```

- [ ] **Step 6: Create src/index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>GitSneaker</title>
    <link rel="stylesheet" href="css/style.css">
</head>
<body>
    <div id="app" v-scope>
        <header class="app-header">
            <h1 class="app-title">GitSneaker</h1>
            <nav class="nav-tabs">
                <button @click="currentView = 'export'" :class="{ active: currentView === 'export' }">Export</button>
                <button @click="currentView = 'import'" :class="{ active: currentView === 'import' }">Import</button>
                <button v-if="currentView === 'conflict'" class="active">Conflict</button>
            </nav>
        </header>

        <main v-if="currentView === 'export'" id="export-view"></main>
        <main v-else-if="currentView === 'import'" id="import-view"></main>
        <main v-else-if="currentView === 'conflict'" id="conflict-view"></main>

        <footer class="status-bar">
            <span>{{ statusText }}</span>
        </footer>
    </div>

    <script src="js/vendor/petite-vue.js"></script>
    <script src="js/api.js"></script>
    <script src="js/app.js"></script>
    <script src="js/views/export.js"></script>
    <script src="js/views/import.js"></script>
    <script src="js/views/conflict.js"></script>
    <script>
        PetiteVue.createApp({
            currentView: 'export',
            statusText: 'Ready',
            repoPath: '',
            repoInfo: null,
            exportPreview: null,
            exportState: 'idle',
            bundlePath: '',
            bundleInfo: null,
            importState: 'idle',
            conflicts: [],
            selectedFile: 0,
            selectedHunk: 0,
            autoReport: null,
            resolvedDecisions: {},

            // Mixins
            ...app,
            ...exportView,
            ...importView,
            ...conflictView,
        }).mount('#app');
    </script>
</body>
</html>
```

- [ ] **Step 7: Commit**

```bash
git add src/
git commit -m "feat: add frontend scaffold — HTML shell, Petite-Vue, api.js, Catppuccin CSS"
```

---

### Task 14: Export view component

**Files:**
- Create: `src/js/views/export.js`
- Modify: `src/index.html` (wire export view HTML)

- [ ] **Step 1: Write src/js/views/export.js**

```javascript
const exportView = {
    async doPreviewExport() {
        if (!this.repoPath) { this.showError('Select a repository first'); return; }
        this.exportState = 'loading';
        this.setStatus('Scanning commits...');
        try {
            this.exportPreview = await api.previewExport(this.repoPath);
            this.exportState = 'idle';
            this.setStatus(`${this.exportPreview.commits.length} commits to sync`);
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },

    async doExecExport() {
        if (!this.repoPath || !this.exportPreview) return;
        this.exportState = 'exporting';
        this.setStatus('Creating bundle...');
        try {
            const result = await api.execExport(this.repoPath, this.repoPath);
            this.exportState = 'done';
            this.setStatus(`Bundle: ${result.file_path} (${(result.file_size / 1024).toFixed(1)} KB)`);
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },

    async doSelectExportOutput() {
        try {
            const selected = await window.__TAURI__.dialog.open({
                directory: true,
                title: 'Select Output Directory (e.g., USB drive)',
            });
            if (selected && this.repoPath && this.exportPreview) {
                this.exportState = 'exporting';
                this.setStatus('Creating bundle...');
                const result = await api.execExport(this.repoPath, selected);
                this.exportState = 'done';
                this.setStatus(`Bundle saved: ${result.file_path}`);
                this.repoInfo = await api.openRepo(this.repoPath);
            }
        } catch (e) { this.showError(e); this.exportState = 'error'; }
    },
};
```

- [ ] **Step 2: Update index.html export view**

Replace `<main v-if="currentView === 'export'" id="export-view">` with:
```html
<main v-if="currentView === 'export'">
    <div class="panel">
        <div class="panel-header">Repository</div>
        <div class="toolbar">
            <input type="text" :value="repoPath" readonly placeholder="Select a git repository...">
            <button class="btn btn-secondary" @click="selectRepo">Browse...</button>
        </div>
        <div v-if="repoInfo" class="path-display">
            Branch: {{ repoInfo.head_branch }} | HEAD: {{ repoInfo.head_commit }}
            <span v-if="repoInfo.last_sync"> | Last sync: {{ repoInfo.last_sync.tag_name }}</span>
            <span v-else> | No previous sync</span>
        </div>
    </div>

    <div class="panel" v-if="exportPreview">
        <div class="panel-header">Pending Commits ({{ exportPreview.commits.length }})</div>
        <div class="commit-list">
            <div v-for="c in exportPreview.commits" class="commit-item">
                <span class="commit-hash">{{ c.hash }}</span>
                <span class="commit-msg">{{ c.message }}</span>
                <span class="commit-author">{{ c.author }}</span>
            </div>
        </div>
        <div v-if="exportPreview.commits.length === 0" class="path-display">No new commits since last sync.</div>
    </div>

    <div class="toolbar" v-if="repoPath">
        <button class="btn btn-primary" @click="doPreviewExport" :disabled="exportState === 'loading'">
            {{ exportState === 'loading' ? 'Scanning...' : 'Preview' }}
        </button>
        <button class="btn btn-success" @click="doExecExport" :disabled="!exportPreview || exportState === 'exporting'">
            Export to Repo
        </button>
        <button class="btn btn-secondary" @click="doSelectExportOutput" :disabled="!exportPreview || exportState === 'exporting'">
            Export to USB...
        </button>
    </div>

    <div v-if="exportState === 'done'" class="auto-report">
        <div class="auto-report-header">Export Complete</div>
        Copy the bundle file to your USB drive and transfer to the target machine.
    </div>
</main>
```

- [ ] **Step 3: Commit**

```bash
git add src/js/views/export.js src/index.html
git commit -m "feat: add Export view — repo select, commit preview, bundle creation"
```

---

### Task 15: Import view component

**Files:**
- Create: `src/js/views/import.js`
- Modify: `src/index.html` (wire import view HTML)

- [ ] **Step 1: Write src/js/views/import.js**

```javascript
const importView = {
    async doVerifyBundle() {
        if (!this.bundlePath) { this.showError('Select a bundle file first'); return; }
        this.importState = 'verifying';
        this.setStatus('Verifying bundle...');
        try {
            this.bundleInfo = await api.verifyBundle(this.bundlePath);
            this.importState = 'idle';
            this.setStatus(`Bundle verified: ${this.bundleInfo.head_commit}`);
        } catch (e) { this.showError(e); this.importState = 'error'; }
    },

    async doExecImport() {
        if (!this.repoPath || !this.bundlePath) {
            this.showError('Select both repository and bundle file');
            return;
        }
        this.importState = 'importing';
        this.setStatus('Importing bundle...');
        try {
            const result = await api.execImport(this.repoPath, this.bundlePath);
            if (result.type === 'Success') {
                this.importState = 'done';
                this.setStatus('Import successful - merged cleanly');
            } else if (result.type === 'Conflicted') {
                this.importState = 'conflicted';
                this.setStatus(`Conflicts in ${result.files.length} file(s)`);
                this.conflicts = await api.getConflicts(this.repoPath);
                this.selectedFile = 0;
                this.selectedHunk = 0;
                this.currentView = 'conflict';
            } else if (result.type === 'AlreadyUpToDate') {
                this.importState = 'done';
                this.setStatus('Already up to date - nothing to import');
            }
        } catch (e) { this.showError(e); this.importState = 'error'; }
    },
};
```

- [ ] **Step 2: Update index.html import view**

Replace `<main v-else-if="currentView === 'import'" id="import-view">` with:
```html
<main v-else-if="currentView === 'import'">
    <div class="panel">
        <div class="panel-header">Target Repository</div>
        <div class="toolbar">
            <input type="text" :value="repoPath" readonly placeholder="Select target repository...">
            <button class="btn btn-secondary" @click="selectRepo">Browse...</button>
        </div>
        <div v-if="repoInfo" class="path-display">
            Branch: {{ repoInfo.head_branch }} | HEAD: {{ repoInfo.head_commit }}
        </div>
    </div>

    <div class="panel">
        <div class="panel-header">Bundle File</div>
        <div class="toolbar">
            <input type="text" :value="bundlePath" readonly placeholder="Select a .bundle file...">
            <button class="btn btn-secondary" @click="selectBundleFile">Browse...</button>
        </div>
        <div v-if="bundleInfo" class="path-display">
            Bundle HEAD: {{ bundleInfo.head_commit }} | Size: {{ (bundleInfo.file_size / 1024).toFixed(1) }} KB
        </div>
    </div>

    <div class="toolbar" v-if="repoPath && bundlePath">
        <button class="btn btn-primary" @click="doVerifyBundle" :disabled="importState === 'verifying'">Verify Bundle</button>
        <button class="btn btn-success" @click="doExecImport" :disabled="importState === 'importing' || !bundleInfo">Import</button>
    </div>

    <div v-if="importState === 'done'" class="auto-report">
        <div class="auto-report-header">Import Complete</div>
        The bundle has been merged successfully with no conflicts.
    </div>
</main>
```

- [ ] **Step 3: Commit**

```bash
git add src/js/views/import.js src/index.html
git commit -m "feat: add Import view — bundle select, verify, import execution"
```

---

### Task 16: Conflict resolve view component

**Files:**
- Create: `src/js/views/conflict.js`
- Modify: `src/index.html` (wire conflict view HTML)

- [ ] **Step 1: Write src/js/views/conflict.js**

```javascript
const conflictView = {
    get currentFile() {
        if (!this.conflicts || this.conflicts.length === 0) return null;
        return this.conflicts[this.selectedFile];
    },
    get currentHunk() {
        const f = this.currentFile;
        if (!f || !f.hunks || f.hunks.length === 0) return null;
        return f.hunks[this.selectedHunk];
    },
    get isFileResolved() {
        if (!this.currentFile) return false;
        const d = this.resolvedDecisions[this.currentFile.path];
        return d && d.length === this.currentFile.hunks.length;
    },

    nextHunk() { if (this.selectedHunk < this.currentFile.hunks.length - 1) this.selectedHunk++; },
    prevHunk() { if (this.selectedHunk > 0) this.selectedHunk--; },
    nextFile() {
        if (this.selectedFile < this.conflicts.length - 1) { this.selectedFile++; this.selectedHunk = 0; }
    },
    prevFile() {
        if (this.selectedFile > 0) { this.selectedFile--; this.selectedHunk = 0; }
    },
    selectFile(idx) { this.selectedFile = idx; this.selectedHunk = 0; },

    async doAutoResolve() {
        if (!this.repoPath || !this.conflicts) return;
        this.setStatus('Analyzing conflicts...');
        try {
            this.autoReport = await api.autoResolve(this.repoPath, this.conflicts);
            this.setStatus(this.autoReport.summary);
            for (const r of this.autoReport.resolved) {
                for (const f of this.conflicts) {
                    for (const h of f.hunks) {
                        if (h.id === r.hunk_id) {
                            if (!this.resolvedDecisions[f.path]) this.resolvedDecisions[f.path] = [];
                            const idx = this.resolvedDecisions[f.path].findIndex(d => d.hunk_id === r.hunk_id);
                            if (idx >= 0) {
                                this.resolvedDecisions[f.path][idx] = r;
                            } else {
                                this.resolvedDecisions[f.path].push(r);
                            }
                        }
                    }
                }
            }
        } catch (e) { this.showError(e); }
    },

    takeLocal() {
        if (!this.currentHunk || !this.currentFile) return;
        this._record(this.currentFile.path, this.selectedHunk, { type: 'TakeLocal' }, this.currentHunk.local);
    },
    takeRemote() {
        if (!this.currentHunk || !this.currentFile) return;
        this._record(this.currentFile.path, this.selectedHunk, { type: 'TakeRemote' }, this.currentHunk.remote);
    },

    _record(filePath, hunkId, decision, content) {
        if (!this.resolvedDecisions[filePath]) this.resolvedDecisions[filePath] = [];
        const idx = this.resolvedDecisions[filePath].findIndex(d => d.hunk_id === hunkId);
        const entry = { hunk_id: hunkId, decision, merged_content: content, auto: false, confidence: 1.0 };
        if (idx >= 0) { this.resolvedDecisions[filePath][idx] = entry; }
        else { this.resolvedDecisions[filePath].push(entry); }
    },

    async applyCurrentFile() {
        if (!this.currentFile) return;
        const decisions = this.resolvedDecisions[this.currentFile.path];
        if (!decisions || decisions.length !== this.currentFile.hunks.length) {
            this.showError('Resolve all hunks in this file first');
            return;
        }
        this.setStatus('Applying resolution...');
        try {
            await api.applyResolution(this.repoPath, this.currentFile.path, decisions);
            this.setStatus(`Applied: ${this.currentFile.path}`);
            const allResolved = this.conflicts.every(f => {
                const d = this.resolvedDecisions[f.path];
                return d && d.length === f.hunks.length;
            });
            if (allResolved) this.setStatus('All conflicts resolved. Ready to commit.');
        } catch (e) { this.showError(e); }
    },

    async doCommitMerge() {
        this.setStatus('Committing merge...');
        try {
            await api.commitMerge(this.repoPath, 'Merge from GitSneaker bundle');
            this.setStatus('Merge committed successfully');
            this.currentView = 'export';
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); }
    },

    async doAbortMerge() {
        try {
            await api.abortMerge(this.repoPath);
            this.setStatus('Merge aborted');
            this.currentView = 'export';
            this.conflicts = [];
            this.resolvedDecisions = {};
            this.repoInfo = await api.openRepo(this.repoPath);
        } catch (e) { this.showError(e); }
    },
};
```

- [ ] **Step 2: Update index.html conflict view**

Replace `<main v-else-if="currentView === 'conflict'" id="conflict-view">` with:
```html
<main v-else-if="currentView === 'conflict'">
    <div v-if="autoReport" class="auto-report">
        <div class="auto-report-header">Auto-Resolve Report</div>
        <div>{{ autoReport.summary }}</div>
        <div v-if="autoReport.manual_hunks.length > 0" class="path-display">
            Manual review needed for {{ autoReport.manual_hunks.length }} hunk(s)
        </div>
    </div>

    <div class="conflict-file-tabs">
        <button v-for="(f, i) in conflicts" :key="f.path" @click="selectFile(i)"
            :class="['conflict-file-tab', { active: i === selectedFile }, { resolved: isFileResolved }]">
            {{ f.path }} ({{ f.hunks.length }} hunks)
        </button>
    </div>

    <div v-if="currentFile" class="toolbar">
        <button class="btn btn-secondary" @click="prevFile" :disabled="selectedFile === 0">Prev File</button>
        <span>{{ currentFile.path }}</span>
        <button class="btn btn-secondary" @click="nextFile" :disabled="selectedFile >= conflicts.length - 1">Next File</button>
        <span class="spacer"></span>
        <span>Hunk {{ selectedHunk + 1 }}/{{ currentFile.hunks.length }}</span>
        <button class="btn btn-secondary" @click="prevHunk" :disabled="selectedHunk === 0">Prev</button>
        <button class="btn btn-secondary" @click="nextHunk" :disabled="selectedHunk >= currentFile.hunks.length - 1">Next</button>
    </div>

    <div v-if="currentHunk" class="diff-threeway">
        <div class="diff-panel">
            <div class="diff-panel-header">LOCAL (this machine)</div>
            <div class="diff-panel-content">{{ currentHunk.local }}</div>
        </div>
        <div class="diff-panel">
            <div class="diff-panel-header">BASE (common ancestor)</div>
            <div class="diff-panel-content">{{ currentHunk.base }}</div>
        </div>
        <div class="diff-panel">
            <div class="diff-panel-header">REMOTE (bundle)</div>
            <div class="diff-panel-content">{{ currentHunk.remote }}</div>
        </div>
    </div>

    <div class="panel" v-if="currentHunk">
        <div class="panel-header">Resolution</div>
        <div class="merge-result" contenteditable="true">{{ currentHunk.local }}</div>
    </div>

    <div class="toolbar" v-if="currentHunk">
        <button class="btn btn-primary" @click="takeLocal">Take LOCAL</button>
        <button class="btn btn-primary" @click="takeRemote">Take REMOTE</button>
        <button class="btn btn-success" @click="applyCurrentFile">Apply This File</button>
        <span class="spacer"></span>
        <button class="btn btn-secondary" @click="doAutoResolve">Auto-Resolve All</button>
    </div>

    <div class="toolbar">
        <button class="btn btn-danger" @click="doAbortMerge">Abort Merge</button>
        <span class="spacer"></span>
        <button class="btn btn-success" @click="doCommitMerge">Complete Merge</button>
    </div>
</main>
```

- [ ] **Step 3: Commit**

```bash
git add src/js/views/conflict.js src/index.html
git commit -m "feat: add Conflict view — 3-way diff, auto-resolve, hunk-by-hunk resolution"
```

---

### Task 17: Final build verification and polish

**Files:**
- No new files. Verify everything compiles and tests pass.

- [ ] **Step 1: Full release build**

Run: `cargo build --release 2>&1`
Expected: Compiles with no errors.

- [ ] **Step 2: Run all tests**

Run: `cargo test --lib 2>&1`
Expected: All tests pass (3 repo + 1 commit + 2 bundle + 5 auto_resolve = 11 tests).

- [ ] **Step 3: Check binary size**

Run: `ls -lh target/release/git-sneaker 2>/dev/null; ls -lh target/release/bundle/ 2>/dev/null`
Verify binary < 10MB (or bundle < 10MB).

- [ ] **Step 4: Fix any remaining warnings/errors**

Address any compiler warnings. Ensure no `unused_imports` or `dead_code` remain in non-test code.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: final build verification, fix warnings, all tests pass"
```
