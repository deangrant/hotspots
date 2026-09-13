//! `SymbolResolver` implementation using Git and `syn`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use hotspots::symbols::{ExpandStats, FileDiff, SymbolResolver, expand_with_diffs, is_rust_path};
use hotspots::{Change, Error, Result};

use crate::diff::parse_hunks;
use crate::git::{
    GitRunner, SystemGit, ensure_work_tree, is_missing_path_error, show_blob, show_hunks,
};
use crate::parse::symbols_from_source;

/// Resolves Rust file changes to `path::symbol` entities via git + syn.
#[derive(Debug, Clone, Copy, Default)]
pub struct RustGitSynResolver<G = SystemGit> {
    git: G,
}

impl RustGitSynResolver<SystemGit> {
    /// Builds a resolver that shells out to `git`.
    #[must_use]
    pub const fn new() -> Self {
        Self { git: SystemGit }
    }
}

impl<G: GitRunner> RustGitSynResolver<G> {
    /// Builds a resolver with a custom git runner (tests).
    #[must_use]
    pub const fn with_git(git: G) -> Self {
        Self { git }
    }
}

impl<G: GitRunner> SymbolResolver for RustGitSynResolver<G> {
    fn expand(&self, changes: &[Change], repo: &Path) -> Result<(Vec<Change>, ExpandStats)> {
        ensure_work_tree(&self.git, repo)?;
        let keys = rust_keys(changes);
        let mut cache = BlobCache::new(&self.git, repo);
        let mut diffs = BTreeMap::new();
        for (rev, path) in keys {
            let diff = build_file_diff(&mut cache, &rev, &path)?;
            diffs.insert((rev, path), diff);
        }
        expand_with_diffs(changes, &diffs)
    }
}

fn rust_keys(changes: &[Change]) -> BTreeSet<(String, String)> {
    changes
        .iter()
        .filter(|c| is_rust_path(&c.entity))
        .map(|c| (c.rev.clone(), c.entity.clone()))
        .collect()
}

struct CachedBlobErr {
    message: String,
    missing_path: bool,
}

struct BlobCache<'a> {
    git: &'a dyn GitRunner,
    repo: &'a Path,
    blobs: HashMap<(String, String), std::result::Result<String, CachedBlobErr>>,
}

impl<'a> BlobCache<'a> {
    fn new(git: &'a dyn GitRunner, repo: &'a Path) -> Self {
        Self {
            git,
            repo,
            blobs: HashMap::new(),
        }
    }

    fn show(&mut self, rev: &str, path: &str) -> Result<String> {
        let key = (rev.to_owned(), path.to_owned());
        if let Some(cached) = self.blobs.get(&key) {
            return cached_show_result(cached);
        }
        fetch_and_store_blob(self, key, rev, path)
    }
}

fn cached_show_result(cached: &std::result::Result<String, CachedBlobErr>) -> Result<String> {
    match cached {
        Ok(src) => Ok(src.clone()),
        Err(err) => Err(cached_blob_error(err)),
    }
}

fn fetch_and_store_blob(
    cache: &mut BlobCache<'_>,
    key: (String, String),
    rev: &str,
    path: &str,
) -> Result<String> {
    let result = show_blob(cache.git, cache.repo, rev, path);
    let stored = match &result {
        Ok(src) => Ok(src.clone()),
        Err(err) => Err(CachedBlobErr {
            message: err.to_string(),
            missing_path: err.is_git_missing_path(),
        }),
    };
    cache.blobs.insert(key, stored);
    result
}

fn cached_blob_error(err: &CachedBlobErr) -> Error {
    if err.missing_path {
        Error::git_missing_path(err.message.clone())
    } else {
        Error::git(err.message.clone())
    }
}

fn build_file_diff(cache: &mut BlobCache<'_>, rev: &str, path: &str) -> Result<FileDiff> {
    let (symbols_new, symbols_old) = load_symbol_sides(cache, rev, path)?;
    let hunks = load_parsed_hunks(cache, rev, path)?;
    Ok(FileDiff {
        rev: rev.to_owned(),
        path: path.to_owned(),
        symbols_new,
        symbols_old,
        hunks,
    })
}

fn load_parsed_hunks(
    cache: &BlobCache<'_>,
    rev: &str,
    path: &str,
) -> Result<Vec<hotspots::symbols::Hunk>> {
    let unified = show_hunks(cache.git, cache.repo, rev, path)
        .map_err(|e| Error::msg(format!("cannot load hunks for `{path}` at `{rev}`: {e}")))?;
    let hunks = parse_hunks(&unified);
    if hunks.is_empty() {
        return Err(Error::msg(format!("no hunks for `{path}` at `{rev}`")));
    }
    Ok(hunks)
}

/// Loads new/old symbol tables, falling back to parent-only for deletes.
fn load_symbol_sides(
    cache: &mut BlobCache<'_>,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    match cache.show(rev, path) {
        Ok(new_src) => load_both_symbol_sides(cache, rev, path, &new_src),
        Err(e) if is_missing_path_error(&e) => load_delete_only_symbols(cache, rev, path),
        Err(e) => Err(Error::msg(format!("cannot load `{path}` at `{rev}`: {e}"))),
    }
}

fn load_both_symbol_sides(
    cache: &mut BlobCache<'_>,
    rev: &str,
    path: &str,
    new_src: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    let symbols_new = symbols_from_source(path, new_src)?;
    let symbols_old = load_old_symbols(cache, rev, path)?;
    Ok((symbols_new, symbols_old))
}

fn load_delete_only_symbols(
    cache: &mut BlobCache<'_>,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    let parent = format!("{rev}^");
    let old_src = cache.show(&parent, path).map_err(|e| {
        Error::msg(format!(
            "cannot load `{path}` at `{rev}` or `{parent}`: {e}"
        ))
    })?;
    let symbols_old = symbols_from_source(path, &old_src)?;
    Ok((Vec::new(), symbols_old))
}

fn load_old_symbols(
    cache: &mut BlobCache<'_>,
    rev: &str,
    path: &str,
) -> Result<Vec<hotspots::symbols::SymbolFact>> {
    let parent = format!("{rev}^");
    let Ok(src) = cache.show(&parent, path) else {
        return Ok(Vec::new());
    };
    symbols_from_source(path, &src)
}

#[cfg(test)]
mod tests {
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
    fn rejects_empty_hunk_patch() {
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
                .is_err_and(|e| e.to_string().contains("no hunks"))
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
        assert!(cached_show_result(&Err(missing)).is_err());
        assert!(cached_show_result(&Ok(String::from("src"))).is_ok());
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
    fn map_git_unexpected_args_error() {
        let git = MapGit::default();
        assert!(git.run(Path::new("/repo"), &["mystery"]).is_err());
    }
}
