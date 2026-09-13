//! `SymbolResolver` implementation using Git and `syn`.

use std::collections::{BTreeMap, BTreeSet};
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
        let mut diffs = BTreeMap::new();
        for (rev, path) in keys {
            let diff = build_file_diff(&self.git, repo, &rev, &path)?;
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

fn build_file_diff(git: &dyn GitRunner, repo: &Path, rev: &str, path: &str) -> Result<FileDiff> {
    let (symbols_new, symbols_old) = load_symbol_sides(git, repo, rev, path)?;
    let unified = show_hunks(git, repo, rev, path)
        .map_err(|e| Error::msg(format!("cannot load hunks for `{path}` at `{rev}`: {e}")))?;
    let hunks = parse_hunks(&unified);
    if hunks.is_empty() {
        return Err(Error::msg(format!("no hunks for `{path}` at `{rev}`")));
    }
    Ok(FileDiff {
        rev: rev.to_owned(),
        path: path.to_owned(),
        symbols_new,
        symbols_old,
        hunks,
    })
}

/// Loads new/old symbol tables, falling back to parent-only for deletes.
fn load_symbol_sides(
    git: &dyn GitRunner,
    repo: &Path,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    match show_blob(git, repo, rev, path) {
        Ok(new_src) => {
            let symbols_new = symbols_from_source(path, &new_src)?;
            let symbols_old = load_old_symbols(git, repo, rev, path)?;
            Ok((symbols_new, symbols_old))
        }
        Err(e) if is_missing_path_error(&e) => load_delete_only_symbols(git, repo, rev, path),
        Err(e) => Err(Error::msg(format!("cannot load `{path}` at `{rev}`: {e}"))),
    }
}

fn load_delete_only_symbols(
    git: &dyn GitRunner,
    repo: &Path,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    let parent = format!("{rev}^");
    let old_src = show_blob(git, repo, &parent, path).map_err(|e| {
        Error::msg(format!(
            "cannot load `{path}` at `{rev}` or `{parent}`: {e}"
        ))
    })?;
    let symbols_old = symbols_from_source(path, &old_src)?;
    Ok((Vec::new(), symbols_old))
}

fn load_old_symbols(
    git: &dyn GitRunner,
    repo: &Path,
    rev: &str,
    path: &str,
) -> Result<Vec<hotspots::symbols::SymbolFact>> {
    let parent = format!("{rev}^");
    let Ok(src) = show_blob(git, repo, &parent, path) else {
        return Ok(Vec::new());
    };
    symbols_from_source(path, &src)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                return Err(Error::msg(msg.clone()));
            }
            Err(Error::msg(format!("unexpected git args: {key}")))
        }
    }

    #[test]
    fn expands_rust_change_via_mock_git() {
        let src = "fn alpha() {}\nfn beta() {}\n";
        let patch = "@@ -0,0 +1,2 @@\n+fn alpha() {}\n+fn beta() {}\n";
        let mut ok = HashMap::new();
        ok.insert(
            String::from("rev-parse --is-inside-work-tree"),
            String::from("true\n"),
        );
        ok.insert(String::from("show abc:a.rs"), String::from(src));
        ok.insert(
            String::from("show -U0 --format= abc -- a.rs"),
            String::from(patch),
        );
        let mut err = HashMap::new();
        err.insert(String::from("show abc^:a.rs"), String::from("missing"));
        err.insert(
            String::from("diff-tree -U0 abc^ abc -- a.rs"),
            String::from("no parent"),
        );
        let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
        let changes = [Change::new(
            "abc",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(2),
            Some(0),
        )];
        let expanded = resolver.expand(&changes, Path::new("/repo"));
        assert!(
            expanded
                .as_ref()
                .is_ok_and(|(rows, _)| rows.iter().any(|c| c.entity == "a.rs::alpha"))
        );
    }

    #[test]
    fn rejects_non_work_tree() {
        let mut ok = HashMap::new();
        ok.insert(
            String::from("rev-parse --is-inside-work-tree"),
            String::from("false\n"),
        );
        let resolver = RustGitSynResolver::with_git(MapGit {
            ok,
            err: HashMap::new(),
        });
        let changes = [Change::new(
            "abc",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(1),
            Some(0),
        )];
        assert!(resolver.expand(&changes, &PathBuf::from("/tmp")).is_err());
    }

    #[test]
    fn expands_delete_only_via_parent_blob() {
        let old_src = "fn alpha() {}\nfn beta() {}\n";
        let patch = "@@ -1,2 +0,0 @@\n-fn alpha() {}\n-fn beta() {}\n";
        let mut ok = HashMap::new();
        ok.insert(
            String::from("rev-parse --is-inside-work-tree"),
            String::from("true\n"),
        );
        ok.insert(String::from("show abc^:a.rs"), String::from(old_src));
        ok.insert(
            String::from("diff-tree -U0 abc^ abc -- a.rs"),
            String::from(patch),
        );
        let mut err = HashMap::new();
        err.insert(
            String::from("show abc:a.rs"),
            String::from("fatal: path 'a.rs' does not exist in 'abc'"),
        );
        let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
        let changes = [Change::new(
            "abc",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(0),
            Some(2),
        )];
        let expanded = resolver.expand(&changes, Path::new("/repo"));
        assert!(expanded.as_ref().is_ok_and(|(rows, _)| {
            rows.iter().any(|c| c.entity == "a.rs::alpha" && c.deleted == Some(1))
        }));
    }

    #[test]
    fn rejects_non_missing_new_side_show_error() {
        let old_src = "fn alpha() {}\n";
        let mut ok = HashMap::new();
        ok.insert(
            String::from("rev-parse --is-inside-work-tree"),
            String::from("true\n"),
        );
        ok.insert(String::from("show abc^:a.rs"), String::from(old_src));
        let mut err = HashMap::new();
        err.insert(String::from("show abc:a.rs"), String::from("bad object"));
        let resolver = RustGitSynResolver::with_git(MapGit { ok, err });
        let changes = [Change::new(
            "abc",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(0),
            Some(1),
        )];
        let expanded = resolver.expand(&changes, Path::new("/repo"));
        assert!(expanded.is_err());
    }
}
