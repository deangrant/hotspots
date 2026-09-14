use super::*;

#[test]
fn rejects_unsafe_rev_and_path() {
    assert!(validate_rev_path("", "a.rs").is_err());
    assert!(validate_rev_path("abc", "").is_err());
    assert!(validate_rev_path("-evil", "a.rs").is_err());
    assert!(validate_rev_path("a:b", "a.rs").is_err());
    assert!(validate_rev_path("abc", "a:b.rs").is_err());
    assert!(validate_rev_path("abc\0", "a.rs").is_err());
}

#[test]
fn accepts_safe_rev_and_path() {
    assert!(validate_rev_path("abc123", "src/a.rs").is_ok());
    assert!(validate_rev_path("abc", "src/my file.rs").is_ok());
}

#[test]
fn missing_path_uses_error_flag() {
    let missing = git_command_failed(&["show", "r:p"], "path 'p' does not exist in 'r'");
    assert!(is_missing_path_error(&missing));
    let on_disk = git_command_failed(
        &["show", "r:p"],
        "fatal: path 'p' exists on disk, but not in 'r'",
    );
    assert!(is_missing_path_error(&on_disk));
    let other = git_command_failed(&["show", "r:p"], "fatal: bad object");
    assert!(!is_missing_path_error(&other));
    let unrelated = git_command_failed(&["diff-tree", "a", "b"], "path 'p' does not exist in 'a'");
    assert!(!is_missing_path_error(&unrelated));
}

#[test]
fn decode_rejects_invalid_utf8() {
    assert!(decode_git_bytes(vec![0xff, 0xfe], "stdout").is_err());
    assert!(decode_git_bytes(b"ok".to_vec(), "stdout").is_ok_and(|s| s == "ok"));
}

#[test]
fn read_capped_enforces_byte_limit() {
    use std::io::Cursor;
    assert!(read_capped(Cursor::new(vec![b'x'; 5]), 10).is_ok());
    assert!(
        read_capped(Cursor::new(vec![b'x'; 20]), 10)
            .is_err_and(|e| e.to_string().contains("exceeds"))
    );
    let join = thread::spawn(|| read_capped(std::io::Cursor::new(vec![0_u8; 20]), 10));
    assert!(
        join_reader(join, "stdout").is_err_and(|e| e.to_string().contains("git stdout exceeds"))
    );
    let join_err = thread::spawn(|| read_capped(std::io::Cursor::new(vec![0_u8; 20]), 10));
    assert!(
        join_reader(join_err, "stderr")
            .is_err_and(|e| e.to_string().contains("git stderr exceeds"))
    );
    assert!(ensure_stdout_byte_limit("ok").is_ok());
    TEST_STDOUT_LIMIT.with(|cell| cell.set(4));
    let over = ensure_stdout_byte_limit("12345");
    TEST_STDOUT_LIMIT.with(|cell| cell.set(0));
    assert!(over.is_err_and(|e| e.to_string().contains("git stdout exceeds")));
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
    assert!(take_pipe::<()>(None, "stdout").is_err_and(|e| e.to_string().contains("pipe missing")));
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
    assert!(join_reader(join_panic, "stdout").is_err_and(|e| e.to_string().contains("panicked")));
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

    let previous = TEST_GIT_EXECUTABLE
        .lock()
        .ok()
        .and_then(|mut guard| guard.replace(OsString::from("/nonexistent/hotspots-missing-git")));
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
