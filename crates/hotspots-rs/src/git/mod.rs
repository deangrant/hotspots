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

/// Hard cap on captured `git` stdout (blobs and patches).
const MAX_GIT_STDOUT_BYTES: usize = 16 * 1024 * 1024;

/// Hard cap on captured `git` stderr.
const MAX_GIT_STDERR_BYTES: usize = 1024 * 1024;

#[cfg(test)]
static TEST_GIT_EXECUTABLE: Mutex<Option<OsString>> = Mutex::new(None);

#[cfg(test)]
static TEST_STDOUT_LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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
        stdout_handle: spawn_reader(stdout, stdout_byte_limit()),
        stderr_handle: spawn_reader(stderr, MAX_GIT_STDERR_BYTES),
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
    pipe: impl Read + Send + 'static,
    max_bytes: usize,
) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
    thread::spawn(move || read_capped(pipe, max_bytes))
}

fn read_capped(mut pipe: impl Read, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let n = pipe.read(&mut chunk)?;
        if n == 0 {
            return Ok(buf);
        }
        if buf.len().saturating_add(n) > max_bytes {
            return Err(cap_exceeded_io(max_bytes));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn cap_exceeded_io(max_bytes: usize) -> std::io::Error {
    std::io::Error::other(format!("exceeds {max_bytes} bytes"))
}

fn join_reader(
    handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &str,
) -> Result<Vec<u8>> {
    match handle.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(err)) => Err(map_reader_io_err(stream, &err)),
        Err(_) => Err(Error::git(format!("git {stream} reader panicked"))),
    }
}

fn map_reader_io_err(stream: &str, err: &std::io::Error) -> Error {
    let detail = err.to_string();
    if detail.starts_with("exceeds ") {
        return Error::git(format!("git {stream} {detail}"));
    }
    Error::git(format!("failed to read git {stream}: {err}"))
}

#[cfg(test)]
fn stdout_byte_limit() -> usize {
    let override_limit = TEST_STDOUT_LIMIT.load(std::sync::atomic::Ordering::Relaxed);
    if override_limit > 0 {
        return override_limit;
    }
    MAX_GIT_STDOUT_BYTES
}

#[cfg(not(test))]
const fn stdout_byte_limit() -> usize {
    MAX_GIT_STDOUT_BYTES
}

#[cfg(test)]
pub fn set_test_stdout_limit(limit: usize) {
    TEST_STDOUT_LIMIT.store(limit, std::sync::atomic::Ordering::Relaxed);
}

/// Rejects git stdout payloads larger than the configured byte cap.
fn ensure_stdout_byte_limit(text: &str) -> Result<()> {
    let max = stdout_byte_limit();
    if text.len() > max {
        return Err(Error::git(format!("git stdout exceeds {max} bytes")));
    }
    Ok(())
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
/// Returns an error when `rev`/`path` are unsafe, `git show` fails, or the blob
/// exceeds the git stdout byte cap.
pub fn show_blob(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    validate_rev_path(rev, path)?;
    let spec = format!("{rev}:{path}");
    let text = git.run(repo, &["show", &spec])?;
    ensure_stdout_byte_limit(&text)?;
    Ok(text)
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
/// Returns an error when `rev`/`path` are unsafe, git fails, or the patch
/// exceeds the git stdout byte cap.
pub fn show_hunks(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<String> {
    validate_rev_path(rev, path)?;
    let parent = format!("{rev}^");
    let text = git
        .run(repo, &["diff-tree", "-U0", &parent, rev, "--", path])
        .or_else(|_| git.run(repo, &["show", "-U0", "--format=", rev, "--", path]))?;
    ensure_stdout_byte_limit(&text)?;
    Ok(text)
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
mod tests;
