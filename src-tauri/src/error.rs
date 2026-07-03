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
