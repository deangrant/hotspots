//! Git command helpers.

use std::path::Path;
use std::process::Command;

use hotspots::{Error, Result};

/// Runs git commands in a repository.
pub trait GitRunner {
    /// Runs `git -C repo …` and returns stdout on success.
    ///
    /// # Errors
    ///
    /// Returns an error when git exits non-zero or cannot be spawned.
    fn run(&self, repo: &Path, args: &[&str]) -> Result<String>;
}

/// Default runner that invokes the `git` binary.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemGit;

impl GitRunner for SystemGit {
    fn run(&self, repo: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .map_err(|e| Error::msg(format!("failed to run git: {e}")))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(Error::msg(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )))
    }
}

/// Ensures `repo` is a Git work tree.
///
/// # Errors
///
/// Returns an error when the path is not a work tree.
pub fn ensure_work_tree(git: &dyn GitRunner, repo: &Path) -> Result<()> {
    let out = git.run(repo, &["rev-parse", "--is-inside-work-tree"])?;
    if out.trim() == "true" {
        return Ok(());
    }
    Err(Error::msg(format!(
        "`{}` is not a git work tree",
        repo.display()
    )))
}

/// Loads blob text at `rev:path`.
///
/// # Errors
///
/// Returns an error when `git show` fails.
pub fn show_blob(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    let spec = format!("{rev}:{path}");
    git.run(repo, &["show", &spec])
}

/// Loads a zero-context patch for `path` at `rev` against its first parent.
///
/// # Errors
///
/// Returns an error when `git diff-tree` fails (except empty parent for roots).
pub fn show_hunks(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    let parent = format!("{rev}^");
    git.run(repo, &["diff-tree", "-U0", &parent, rev, "--", path])
        .or_else(|_| git.run(repo, &["show", "-U0", "--format=", rev, "--", path]))
}
