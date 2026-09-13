//! Git command helpers.

use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use hotspots::{Error, Result};

/// Maximum wall time allowed for a single `git` invocation.
const GIT_TIMEOUT: Duration = Duration::from_secs(30);

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
        let mut child = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::msg(format!("failed to run git: {e}")))?;
        let mut stdout =
            child.stdout.take().ok_or_else(|| Error::msg("git stdout pipe missing"))?;
        let mut stderr =
            child.stderr.take().ok_or_else(|| Error::msg("git stderr pipe missing"))?;
        let stdout_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).map(|_| buf)
        });
        let stderr_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf).map(|_| buf)
        });
        let status = wait_with_timeout(&mut child, args)?;
        let stdout_bytes = stdout_handle
            .join()
            .map_err(|_| Error::msg("git stdout reader panicked"))?
            .map_err(|e| Error::msg(format!("failed to read git stdout: {e}")))?;
        let stderr_bytes = stderr_handle
            .join()
            .map_err(|_| Error::msg("git stderr reader panicked"))?
            .map_err(|e| Error::msg(format!("failed to read git stderr: {e}")))?;
        if status.success() {
            return Ok(String::from_utf8_lossy(&stdout_bytes).into_owned());
        }
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        Err(Error::msg(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )))
    }
}

fn wait_with_timeout(child: &mut std::process::Child, args: &[&str]) -> Result<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() >= GIT_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::msg(format!(
                    "git {} timed out after {}s",
                    args.join(" "),
                    GIT_TIMEOUT.as_secs()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                return Err(Error::msg(format!("failed to wait for git: {e}")));
            }
        }
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

/// Returns whether `err` looks like Git's missing-path message for `show REV:PATH`.
#[must_use]
pub fn is_missing_path_error(err: &Error) -> bool {
    err.to_string().to_ascii_lowercase().contains("does not exist")
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
