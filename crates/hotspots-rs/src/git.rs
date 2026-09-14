//! Git command helpers.
//!
//! [`SystemGit`] resolves the binary from `GIT_EXECUTABLE` when set, otherwise
//! the `git` name on `PATH` (normal for local developer CLIs).

use std::ffi::OsString;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
#[cfg(test)]
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use hotspots::{Error, Result};

/// Maximum wall time allowed for a single `git` invocation.
const GIT_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(test)]
static TEST_GIT_EXECUTABLE: Mutex<Option<OsString>> = Mutex::new(None);

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
        let session = start_git_session(repo, args)?;
        complete_git_session(session, args)
    }
}

struct GitSession {
    child: Child,
    stdout_handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stderr_handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
}

fn start_git_session(repo: &Path, args: &[&str]) -> Result<GitSession> {
    let mut child = spawn_git(repo, args)?;
    let stdout = take_pipe(child.stdout.take(), "stdout")?;
    let stderr = take_pipe(child.stderr.take(), "stderr")?;
    Ok(GitSession {
        child,
        stdout_handle: spawn_reader(stdout),
        stderr_handle: spawn_reader(stderr),
    })
}

fn complete_git_session(session: GitSession, args: &[&str]) -> Result<String> {
    let GitSession {
        mut child,
        stdout_handle,
        stderr_handle,
    } = session;
    let status = wait_with_timeout(&mut child, args)?;
    let stdout_bytes = join_reader(stdout_handle, "stdout")?;
    let stderr_bytes = join_reader(stderr_handle, "stderr")?;
    finish_git_status(status, args, stdout_bytes, stderr_bytes)
}

fn spawn_git(repo: &Path, args: &[&str]) -> Result<Child> {
    Command::new(git_executable())
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| spawn_git_err(&err))
}

#[inline(never)]
fn spawn_git_err(err: &std::io::Error) -> Error {
    Error::git(format!("failed to run git: {err}"))
}

fn take_pipe<T>(pipe: Option<T>, stream: &str) -> Result<T> {
    pipe.ok_or_else(|| pipe_missing_err(stream))
}

#[inline(never)]
fn pipe_missing_err(stream: &str) -> Error {
    Error::git(format!("git {stream} pipe missing"))
}

fn spawn_reader(
    mut pipe: impl Read + Send + 'static,
) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        pipe.read_to_end(&mut buf).map(|_| buf)
    })
}

fn join_reader(
    handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &str,
) -> Result<Vec<u8>> {
    match handle.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(err)) => Err(Error::git(format!("failed to read git {stream}: {err}"))),
        Err(_) => Err(Error::git(format!("git {stream} reader panicked"))),
    }
}

fn finish_git_status(
    status: ExitStatus,
    args: &[&str],
    stdout_bytes: Vec<u8>,
    stderr_bytes: Vec<u8>,
) -> Result<String> {
    if status.success() {
        return decode_git_bytes(stdout_bytes, "stdout");
    }
    let stderr = decode_git_bytes(stderr_bytes, "stderr")?;
    Err(git_command_failed(args, stderr.trim()))
}

fn git_executable() -> OsString {
    #[cfg(test)]
    if let Ok(guard) = TEST_GIT_EXECUTABLE.lock()
        && let Some(exe) = guard.as_ref()
    {
        return exe.clone();
    }
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

fn wait_with_timeout(child: &mut Child, args: &[&str]) -> Result<ExitStatus> {
    wait_child(child, args, GIT_TIMEOUT)
}

fn wait_child(child: &mut Child, args: &[&str], timeout: Duration) -> Result<ExitStatus> {
    let started = Instant::now();
    loop {
        let poll = classify_try_wait(child.try_wait(), started.elapsed() >= timeout);
        if let Some(result) = apply_wait_poll(child, args, timeout, poll) {
            return result;
        }
    }
}

enum WaitPoll {
    Done(ExitStatus),
    TimedOut,
    Pending,
    Failed(String),
}

fn classify_try_wait(result: std::io::Result<Option<ExitStatus>>, timed_out: bool) -> WaitPoll {
    match result {
        Ok(status) => classify_ok_wait(status, timed_out),
        Err(e) => WaitPoll::Failed(format!("failed to wait for git: {e}")),
    }
}

const fn classify_ok_wait(status: Option<ExitStatus>, timed_out: bool) -> WaitPoll {
    match status {
        Some(status) => WaitPoll::Done(status),
        None if timed_out => WaitPoll::TimedOut,
        None => WaitPoll::Pending,
    }
}

fn apply_wait_poll(
    child: &mut Child,
    args: &[&str],
    timeout: Duration,
    poll: WaitPoll,
) -> Option<Result<ExitStatus>> {
    match poll {
        WaitPoll::Done(status) => Some(Ok(status)),
        WaitPoll::TimedOut => Some(kill_timed_out(child, args, timeout)),
        WaitPoll::Pending => {
            thread::sleep(Duration::from_millis(50));
            None
        }
        WaitPoll::Failed(message) => Some(Err(Error::git(message))),
    }
}

#[inline(never)]
fn kill_timed_out(child: &mut Child, args: &[&str], timeout: Duration) -> Result<ExitStatus> {
    let _ = child.kill();
    let _ = child.wait();
    Err(Error::git(format!(
        "git {} timed out after {}s",
        args.join(" "),
        timeout.as_secs()
    )))
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

    #[test]
    fn classify_wait_poll_states() {
        assert!(matches!(
            classify_try_wait(Ok(None), false),
            WaitPoll::Pending
        ));
        assert!(matches!(
            classify_try_wait(Ok(None), true),
            WaitPoll::TimedOut
        ));
        assert!(matches!(
            classify_try_wait(Err(std::io::Error::other("boom")), false),
            WaitPoll::Failed(_)
        ));
    }

    #[test]
    fn apply_wait_poll_failed_maps_error() {
        let spawned = Command::new("true").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        assert!(spawned.is_ok(), "spawn true failed");
        let mut children: Vec<_> = spawned.ok().into_iter().collect();
        assert_eq!(children.len(), 1);
        let mut child = children.remove(0);
        let _ = child.wait();
        let result = apply_wait_poll(
            &mut child,
            &["true"],
            Duration::from_secs(1),
            WaitPoll::Failed(String::from("failed to wait for git: boom")),
        );
        assert!(result.is_some_and(|r| r.is_err_and(|e| e.to_string().contains("boom"))));
    }

    #[test]
    fn wait_child_times_out() {
        let spawned = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        assert!(spawned.is_ok(), "spawn sleep failed");
        let mut children: Vec<_> = spawned.ok().into_iter().collect();
        assert_eq!(children.len(), 1);
        let mut child = children.remove(0);
        let err = wait_child(&mut child, &["sleep"], Duration::from_millis(50));
        assert!(err.is_err_and(|e| e.to_string().contains("timed out")));
    }

    #[test]
    fn apply_wait_poll_timeout_kills_child() {
        let spawned = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        assert!(spawned.is_ok(), "spawn sleep failed");
        let mut children: Vec<_> = spawned.ok().into_iter().collect();
        assert_eq!(children.len(), 1);
        let mut child = children.remove(0);
        let result = apply_wait_poll(
            &mut child,
            &["sleep"],
            Duration::from_secs(1),
            WaitPoll::TimedOut,
        );
        assert!(result.is_some_and(|r| r.is_err_and(|e| e.to_string().contains("timed out"))));
    }

    #[test]
    fn take_pipe_and_spawn_error_paths() {
        assert!(
            take_pipe::<()>(None, "stdout").is_err_and(|e| e.to_string().contains("pipe missing"))
        );
        assert!(take_pipe(Some(()), "stdout").is_ok());
        assert!(pipe_missing_err("stderr").to_string().contains("pipe missing"));
        assert!(
            spawn_git_err(&std::io::Error::other("boom"))
                .to_string()
                .contains("failed to run git")
        );
        let join_ok = thread::spawn(|| Ok(b"ok".to_vec()));
        assert!(join_reader(join_ok, "stdout").is_ok_and(|b| b == b"ok"));
        let join_io = thread::spawn(|| Err(std::io::Error::other("read fail")));
        assert!(
            join_reader(join_io, "stderr").is_err_and(|e| e.to_string().contains("failed to read"))
        );
        let join_panic = thread::spawn(|| -> std::io::Result<Vec<u8>> {
            #[expect(clippy::panic, reason = "intentional panic to cover join Err arm")]
            {
                panic!("reader boom");
            }
        });
        assert!(
            join_reader(join_panic, "stdout").is_err_and(|e| e.to_string().contains("panicked"))
        );
    }

    #[test]
    fn system_git_session_and_failure_status() {
        let status = Command::new("false").status();
        assert!(status.as_ref().is_ok_and(|s| !s.success()));
        let mut statuses: Vec<_> = status.ok().into_iter().collect();
        assert_eq!(statuses.len(), 1);
        let status = statuses.remove(0);
        assert!(
            finish_git_status(status, &["false"], Vec::new(), b"fatal: boom".to_vec())
                .is_err_and(|e| e.to_string().contains("boom"))
        );
        assert!(
            finish_git_status(status, &["false"], Vec::new(), vec![0xff, 0xfe])
                .is_err_and(|e| e.to_string().contains("not valid UTF-8"))
        );
        let outside = SystemGit.run(Path::new("/tmp"), &["rev-parse", "--is-inside-work-tree"]);
        assert!(outside.is_err());

        let previous = TEST_GIT_EXECUTABLE.lock().ok().and_then(|mut guard| {
            guard.replace(OsString::from("/nonexistent/hotspots-missing-git"))
        });
        let spawn_err = SystemGit.run(Path::new("/tmp"), &["status"]);
        if let Ok(mut guard) = TEST_GIT_EXECUTABLE.lock() {
            *guard = previous;
        }
        assert!(spawn_err.is_err_and(|e| e.to_string().contains("failed to run git")));
    }

    #[test]
    fn covers_expand_missing_diff_in_dep_crate() {
        let change = hotspots::Change::new("1", "Ada", "2024-01-01", "a.rs", Some(1), Some(0));
        let expanded = hotspots::symbols::expand_with_diffs(
            std::slice::from_ref(&change),
            &std::collections::BTreeMap::new(),
        );
        assert!(expanded.is_err_and(|e| e.to_string().contains("missing symbol diff")));
    }
}
