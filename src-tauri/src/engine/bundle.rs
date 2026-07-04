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

pub fn get_branch_name(bundle_path: &Path) -> Result<String, SneakerError> {
    let output = ShellCommand::new("git")
        .args(["bundle", "list-heads", &bundle_path.display().to_string()])
        .output()
        .map_err(|e| SneakerError::BundleVerifyFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut refs = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let ref_name = parts[1].to_string();
            refs.push(ref_name);
        }
    }

    // Find branch ref (refs/heads/xxx)
    for ref_name in &refs {
        if let Some(branch) = ref_name.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
    }

    // Try to find HEAD
    for ref_name in &refs {
        if ref_name == "HEAD" {
            // HEAD exists, try to find what it points to
            continue;
        }
    }

    // If only one ref, use it
    if refs.len() == 1 {
        let ref_name = &refs[0];
        if ref_name == "HEAD" {
            return Ok("HEAD".to_string());
        }
        if let Some(branch) = ref_name.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
        return Ok(ref_name.clone());
    }

    // Default to HEAD
    Ok("HEAD".to_string())
}

pub fn verify(bundle_path: &Path, _repo_path: Option<&Path>) -> Result<BundleInfo, SneakerError> {
    if !bundle_path.exists() {
        return Err(SneakerError::FileNotFound(bundle_path.display().to_string()));
    }

    // Get head commit and refs from list-heads
    let heads_output = ShellCommand::new("git")
        .args(["bundle", "list-heads", &bundle_path.display().to_string()])
        .output()
        .map_err(|e| SneakerError::BundleVerifyFailed(e.to_string()))?;

    if !heads_output.status.success() {
        let stderr = String::from_utf8_lossy(&heads_output.stderr);
        return Err(SneakerError::BundleVerifyFailed(stderr.to_string()));
    }

    let heads_stdout = String::from_utf8_lossy(&heads_output.stdout);
    let mut refs: Vec<(String, String)> = Vec::new();

    for line in heads_stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].to_string();
            let ref_name = parts[1..].join(" ");
            refs.push((hash, ref_name));
        }
    }

    // Find HEAD ref or use first ref
    let (head_hash, head_ref) = refs
        .iter()
        .find(|(_, r)| r == "HEAD" || r.ends_with("/HEAD"))
        .or_else(|| refs.first())
        .map(|(h, r)| (h.clone(), r.clone()))
        .unwrap_or_else(|| ("unknown".to_string(), "HEAD".to_string()));

    let head_commit = head_hash[..7.min(head_hash.len())].to_string();

    let metadata = std::fs::metadata(bundle_path)
        .map_err(|e| SneakerError::FileNotFound(e.to_string()))?;

    Ok(BundleInfo {
        head_commit,
        head_message: head_ref,
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
        assert!(
            info.head_commit.len() == 7
                && info.head_commit.chars().all(|c| c.is_ascii_hexdigit()),
            "head_commit should be a 7-char hex hash, got: {}",
            info.head_commit
        );
    }

    #[test]
    fn test_verify_nonexistent_bundle() {
        let result = verify(Path::new("/no/such/bundle"));
        assert!(result.is_err());
    }
}
