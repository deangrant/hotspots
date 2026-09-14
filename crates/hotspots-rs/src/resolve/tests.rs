use super::*;

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

use hotspots::Change;
use hotspots::symbols::SymbolResolver;

#[derive(Default)]
struct MapGit {
    ok: HashMap<String, String>,
    err: HashMap<String, String>,
}

impl GitRunner for MapGit {
    fn run(&self, _repo: &Path, args: &[&str]) -> Result<String> {
        let key = args.join(" ");
        if let Some(out) = self.ok.get(&key) {
            return Ok(out.clone());
        }
        if let Some(msg) = self.err.get(&key) {
            if msg.to_ascii_lowercase().contains("does not exist") {
                return Err(Error::git_missing_path(msg.clone()));
            }
            return Err(Error::git(msg.clone()));
        }
        Err(Error::git(format!("unexpected git args: {key}")))
    }
}

struct CountingGit {
    inner: MapGit,
    shows: Cell<usize>,
}

impl GitRunner for CountingGit {
    fn run(&self, repo: &Path, args: &[&str]) -> Result<String> {
        if args.first() == Some(&"show") && args.len() == 2 && args[1].contains(':') {
            self.shows.set(self.shows.get() + 1);
        }
        self.inner.run(repo, args)
    }
}

fn work_tree_ok() -> HashMap<String, String> {
    HashMap::from([(
        String::from("rev-parse --is-inside-work-tree"),
        String::from("true\n"),
    )])
}

fn change(rev: &str, added: Option<u64>, deleted: Option<u64>) -> Change {
    Change::new(rev, "Ada", "2024-01-01", "a.rs", added, deleted)
}

#[test]
fn expands_rust_change_via_mock_git() {
    let src = "fn alpha() {}\nfn beta() {}\n";
    let patch = "@@ -0,0 +1,2 @@\n+fn alpha() {}\n+fn beta() {}\n";
    let mut ok = work_tree_ok();
    ok.insert(String::from("show abc:a.rs"), String::from(src));
    ok.insert(
        String::from("show -U0 --format= abc -- a.rs"),
        String::from(patch),
    );
    let err = HashMap::from([
        (String::from("show abc^:a.rs"), String::from("missing")),
        (
            String::from("diff-tree -U0 abc^ abc -- a.rs"),
            String::from("no parent"),
        ),
    ]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    let expanded = resolver.expand(&[change("abc", Some(2), Some(0))], Path::new("/repo"));
    assert!(expanded.is_ok_and(|(rows, _)| rows.iter().any(|c| c.entity == "a.rs::alpha")));
}

#[test]
fn rejects_non_work_tree() {
    let ok = HashMap::from([(
        String::from("rev-parse --is-inside-work-tree"),
        String::from("false\n"),
    )]);
    let resolver = RustGitSynResolver::with_git(MapGit {
        ok,
        err: HashMap::new(),
    });
    assert!(
        resolver
            .expand(&[change("abc", Some(1), Some(0))], &PathBuf::from("/tmp"))
            .is_err()
    );
}

#[test]
fn expands_delete_only_via_parent_blob() {
    let old_src = "fn alpha() {}\nfn beta() {}\n";
    let patch = "@@ -1,2 +0,0 @@\n-fn alpha() {}\n-fn beta() {}\n";
    let mut ok = work_tree_ok();
    ok.insert(String::from("show abc^:a.rs"), String::from(old_src));
    ok.insert(
        String::from("diff-tree -U0 abc^ abc -- a.rs"),
        String::from(patch),
    );
    let err = HashMap::from([(
        String::from("show abc:a.rs"),
        String::from("fatal: path 'a.rs' does not exist in 'abc'"),
    )]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    let expanded = resolver.expand(&[change("abc", Some(0), Some(2))], Path::new("/repo"));
    assert!(expanded.is_ok_and(|(rows, _)| {
        rows.iter().any(|c| c.entity == "a.rs::alpha" && c.deleted == Some(1))
    }));
}

#[test]
fn cached_blob_error_replay() {
    let ok = work_tree_ok();
    let err = HashMap::from([
        (String::from("show abc:a.rs"), String::from("bad object")),
        (
            String::from("show def:a.rs"),
            String::from("path 'a.rs' does not exist in 'def'"),
        ),
        (String::from("show def^:a.rs"), String::from("also missing")),
    ]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    let bad = [change("abc", Some(1), Some(0))];
    assert!(resolver.expand(&bad, Path::new("/repo")).is_err());
    assert!(resolver.expand(&bad, Path::new("/repo")).is_err());
    assert!(resolver.expand(&[change("def", Some(0), Some(1))], Path::new("/repo")).is_err());
}

#[test]
fn drops_empty_hunk_patch() {
    let mut ok = work_tree_ok();
    ok.insert(
        String::from("show xyz:a.rs"),
        String::from("fn alpha() {}\n"),
    );
    ok.insert(
        String::from("show xyz^:a.rs"),
        String::from("fn alpha() {}\n"),
    );
    ok.insert(
        String::from("show -U0 --format= xyz -- a.rs"),
        String::from("no hunk headers here\n"),
    );
    let err = HashMap::from([(
        String::from("diff-tree -U0 xyz^ xyz -- a.rs"),
        String::from("no parent"),
    )]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    assert!(
        resolver
            .expand(&[change("xyz", Some(1), Some(0))], Path::new("/repo"))
            .is_ok_and(|(rows, stats)| rows.is_empty() && stats.dropped_no_overlap == 1)
    );
}

#[test]
fn cached_blob_error_helpers() {
    let missing = CachedBlobErr {
        message: String::from("gone"),
        missing_path: true,
    };
    let other = CachedBlobErr {
        message: String::from("boom"),
        missing_path: false,
    };
    assert!(cached_blob_error(&missing).is_git_missing_path());
    assert!(!cached_blob_error(&other).is_git_missing_path());
    assert!(cached_symbols_result(&Err(missing)).is_err());
    assert!(
        cached_symbols_result(&Ok(vec![hotspots::symbols::SymbolFact {
            path: String::from("a.rs"),
            start_line: 1,
            end_line: 1,
            name: String::from("alpha"),
        }]))
        .is_ok()
    );
}

#[test]
fn blob_cache_reuses_shared_rev_path() {
    let src = "fn alpha() {}\n";
    let patch = "@@ -0,0 +1,1 @@\n+fn alpha() {}\n";
    let mut ok = work_tree_ok();
    ok.insert(String::from("show base:a.rs"), String::from(src));
    ok.insert(String::from("show base^:a.rs"), String::from(src));
    ok.insert(
        String::from("show -U0 --format= base -- a.rs"),
        String::from(patch),
    );
    ok.insert(
        String::from("show -U0 --format= base^ -- a.rs"),
        String::from(patch),
    );
    let err = HashMap::from([
        (
            String::from("diff-tree -U0 base^ base -- a.rs"),
            String::from("no parent"),
        ),
        (
            String::from("diff-tree -U0 base^^ base^ -- a.rs"),
            String::from("no parent"),
        ),
        (String::from("show base^^:a.rs"), String::from("missing")),
    ]);
    let git = CountingGit {
        inner: MapGit { ok, err },
        shows: Cell::new(0),
    };
    let resolver = RustGitSynResolver::with_git(git);
    let changes = [
        Change::new("base", "Ada", "2024-01-01", "a.rs", Some(1), Some(0)),
        Change::new("base^", "Ada", "2024-01-02", "a.rs", Some(1), Some(0)),
    ];
    assert!(resolver.expand(&changes, Path::new("/repo")).is_ok());
    assert_eq!(resolver.git.shows.get(), 3);
}

#[test]
fn rejects_blob_over_stdout_byte_limit() {
    crate::git::set_test_stdout_limit(32);
    let mut ok = work_tree_ok();
    ok.insert(String::from("show huge:a.rs"), "fn alpha() {}\n".repeat(8));
    ok.insert(
        String::from("show huge^:a.rs"),
        String::from("fn alpha() {}\n"),
    );
    let err = HashMap::from([(
        String::from("diff-tree -U0 huge^ huge -- a.rs"),
        String::from("no parent"),
    )]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    let result = resolver.expand(&[change("huge", Some(1), Some(0))], Path::new("/repo"));
    crate::git::set_test_stdout_limit(0);
    assert!(result.is_err_and(|e| e.to_string().contains("exceeds")));
}

#[test]
fn map_git_unexpected_args_error() {
    let git = MapGit::default();
    assert!(git.run(Path::new("/repo"), &["mystery"]).is_err());
}

#[test]
fn hunk_load_and_syn_parse_errors() {
    assert!(
        hunks_load_err("a.rs", "abc", &Error::git("boom"))
            .to_string()
            .contains("cannot load hunks")
    );
    assert!(parse_symbols("a.rs", "fn not rust {{{").is_err());

    let mut ok = work_tree_ok();
    ok.insert(String::from("show bad:a.rs"), String::from("fn broken {{{"));
    ok.insert(
        String::from("show bad^:a.rs"),
        String::from("fn alpha() {}\n"),
    );
    let err = HashMap::from([
        (
            String::from("diff-tree -U0 bad^ bad -- a.rs"),
            String::from("no parent"),
        ),
        (
            String::from("show -U0 --format= bad -- a.rs"),
            String::from("also fail"),
        ),
    ]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    assert!(resolver.expand(&[change("bad", Some(1), Some(0))], Path::new("/repo")).is_err());

    let mut ok = work_tree_ok();
    ok.insert(
        String::from("show syn:a.rs"),
        String::from("fn alpha() {}\n"),
    );
    let err = HashMap::from([
        (
            String::from("diff-tree -U0 syn^ syn -- a.rs"),
            String::from("fail"),
        ),
        (
            String::from("show -U0 --format= syn -- a.rs"),
            String::from("fail"),
        ),
    ]);
    let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
    assert!(
        resolver
            .expand(&[change("syn", Some(1), Some(0))], Path::new("/repo"))
            .is_err_and(|e| e.to_string().contains("cannot load hunks"))
    );
}
