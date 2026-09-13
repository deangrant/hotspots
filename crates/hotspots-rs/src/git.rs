//! Git command helpers.
//!
//! [`SystemGit`] resolves the binary from `GIT_EXECUTABLE` when set, otherwise
//! the `git` name on `PATH` (normal for local developer CLIs).

use std::ffi::OsString;
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

/// Default runner that invokes the `git` binary from `PATH` (or `GIT_EXECUTABLE`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemGit;

impl GitRunner for SystemGit {
    fn run(&self, repo: &Path, args: &[&str]) -> Result<String> {
        let mut child = Command::new(git_executable())
            .arg("-C")
            .arg(repo)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::git(format!("failed to run git: {e}")))?;
        let mut stdout =
            child.stdout.take().ok_or_else(|| Error::git("git stdout pipe missing"))?;
        let mut stderr =
            child.stderr.take().ok_or_else(|| Error::git("git stderr pipe missing"))?;
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
            .map_err(|_| Error::git("git stdout reader panicked"))?
            .map_err(|e| Error::git(format!("failed to read git stdout: {e}")))?;
        let stderr_bytes = stderr_handle
            .join()
            .map_err(|_| Error::git("git stderr reader panicked"))?
            .map_err(|e| Error::git(format!("failed to read git stderr: {e}")))?;
        if status.success() {
            return decode_git_bytes(stdout_bytes, "stdout");
        }
        let stderr = decode_git_bytes(stderr_bytes, "stderr")?;
        Err(git_command_failed(args, stderr.trim()))
    }
}

fn git_executable() -> OsString {
    std::env::var_os("GIT_EXECUTABLE").unwrap_or_else(|| OsString::from("git"))
}

fn decode_git_bytes(bytes: Vec<u8>, stream: &str) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| Error::git(format!("git {stream} is not valid UTF-8")))
}

fn git_command_failed(args: &[&str], stderr: &str) -> Error {
    let message = format!("git {} failed: {stderr}", args.join(" "));
    if stderr.to_ascii_lowercase().contains("does not exist") {
        Error::git_missing_path(message)
    } else {
        Error::git(message)
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
                return Err(Error::git(format!(
                    "git {} timed out after {}s",
                    args.join(" "),
                    GIT_TIMEOUT.as_secs()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                return Err(Error::git(format!("failed to wait for git: {e}")));
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
    Err(Error::git(format!(
        "`{}` is not a git work tree",
        repo.display()
    )))
}

/// Loads blob text at `rev:path`.
///
/// # Errors
///
/// Returns an error when `rev`/`path` are unsafe or `git show` fails.
pub fn show_blob(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    validate_rev_path(rev, path)?;
    let spec = format!("{rev}:{path}");
    git.run(repo, &["show", &spec])
}

/// Returns whether `err` is Git's missing-path signal for `show REV:PATH`.
#[must_use]
pub const fn is_missing_path_error(err: &Error) -> bool {
    err.is_git_missing_path()
}

/// Loads a zero-context patch for `path` at `rev` against its first parent.
///
/// # Errors
///
/// Returns an error when `rev`/`path` are unsafe or git fails.
pub fn show_hunks(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    validate_rev_path(rev, path)?;
    let parent = format!("{rev}^");
    git.run(repo, &["diff-tree", "-U0", &parent, rev, "--", path])
        .or_else(|_| git.run(repo, &["show", "-U0", "--format=", rev, "--", path]))
}

fn validate_rev_path(rev: &str, path: &str) -> Result<()> {
    validate_git_token(rev, "rev")?;
    validate_git_token(path, "path")?;
    if rev.starts_with('-') {
        return Err(Error::git(format!("unsafe git rev `{rev}`")));
    }
    Ok(())
}

fn validate_git_token(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::git(format!("empty git {label}")));
    }
    if value.contains(':') {
        return Err(Error::git(format!("unsafe git {label} `{value}`")));
    }
    if value.chars().any(char::is_control) {
        return Err(Error::git(format!("unsafe git {label} `{value}`")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_rev_and_path() {
        assert!(validate_rev_path("", "a.rs").is_err());
        assert!(validate_rev_path("abc", "").is_err());
        assert!(validate_rev_path("-evil", "a.rs").is_err());
        assert!(validate_rev_path("a:b", "a.rs").is_err());
        assert!(validate_rev_path("abc", "a:b.rs").is_err());
        assert!(validate_rev_path("abc\0", "a.rs").is_err());
        assert!(validate_rev_path("abc123", "src/a.rs").is_ok());
    }

    #[test]
    fn missing_path_uses_error_flag() {
        let err = git_command_failed(&["show", "r:p"], "path 'p' does not exist in 'r'");
        assert!(is_missing_path_error(&err));
        let other = git_command_failed(&["show"], "fatal: bad object");
        assert!(!is_missing_path_error(&other));
    }

    #[test]
    fn decode_rejects_invalid_utf8() {
        assert!(decode_git_bytes(vec![0xff, 0xfe], "stdout").is_err());
        assert!(decode_git_bytes(b"ok".to_vec(), "stdout").is_ok_and(|s| s == "ok"));
    }
}
