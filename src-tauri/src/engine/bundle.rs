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
