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
        let mut cache = SymbolCache::new(&self.git, repo);
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

type CachedSymbols = std::result::Result<Vec<hotspots::symbols::SymbolFact>, CachedBlobErr>;

struct SymbolCache<'a> {
    git: &'a dyn GitRunner,
    repo: &'a Path,
    symbols: HashMap<(String, String), CachedSymbols>,
}

impl<'a> SymbolCache<'a> {
    fn new(git: &'a dyn GitRunner, repo: &'a Path) -> Self {
        Self {
            git,
            repo,
            symbols: HashMap::new(),
        }
    }

    fn symbols_for(&mut self, rev: &str, path: &str) -> Result<Vec<hotspots::symbols::SymbolFact>> {
        let key = (rev.to_owned(), path.to_owned());
        if let Some(cached) = self.symbols.get(&key) {
            return cached_symbols_result(cached);
        }
        fetch_and_store_symbols(self, key, rev, path)
    }
}

fn cached_symbols_result(cached: &CachedSymbols) -> Result<Vec<hotspots::symbols::SymbolFact>> {
    match cached {
        Ok(facts) => Ok(facts.clone()),
        Err(err) => Err(cached_blob_error(err)),
    }
}

fn fetch_and_store_symbols(
    cache: &mut SymbolCache<'_>,
    key: (String, String),
    rev: &str,
    path: &str,
) -> Result<Vec<hotspots::symbols::SymbolFact>> {
    let source = match show_blob(cache.git, cache.repo, rev, path) {
        Ok(src) => src,
        Err(err) => {
            cache.symbols.insert(key, Err(cached_err_from(&err)));
            return Err(err);
        }
    };
    let facts = match parse_symbols(path, &source) {
        Ok(facts) => facts,
        Err(err) => {
            cache.symbols.insert(key, Err(cached_err_from(&err)));
            return Err(err);
        }
    };
    cache.symbols.insert(key, Ok(facts.clone()));
    Ok(facts)
}

fn cached_err_from(err: &Error) -> CachedBlobErr {
    CachedBlobErr {
        message: err.to_string(),
        missing_path: err.is_git_missing_path(),
    }
}

fn cached_blob_error(err: &CachedBlobErr) -> Error {
    if err.missing_path {
        Error::git_missing_path(err.message.clone())
    } else {
        Error::git(err.message.clone())
    }
}

fn build_file_diff(cache: &mut SymbolCache<'_>, rev: &str, path: &str) -> Result<FileDiff> {
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
    cache: &SymbolCache<'_>,
    rev: &str,
    path: &str,
) -> Result<Vec<hotspots::symbols::Hunk>> {
    let unified = match show_hunks(cache.git, cache.repo, rev, path) {
        Ok(unified) => unified,
        Err(err) => return Err(hunks_load_err(path, rev, &err)),
    };
    Ok(parse_hunks(&unified))
}

fn hunks_load_err(path: &str, rev: &str, err: &Error) -> Error {
    Error::msg(format!("cannot load hunks for `{path}` at `{rev}`: {err}"))
}

/// Loads new/old symbol tables, falling back to parent-only for deletes.
fn load_symbol_sides(
    cache: &mut SymbolCache<'_>,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    match cache.symbols_for(rev, path) {
        Ok(symbols_new) => Ok((symbols_new, load_old_symbols(cache, rev, path))),
        Err(e) if is_missing_path_error(&e) => load_delete_only_symbols(cache, rev, path),
        Err(e) => Err(Error::msg(format!("cannot load `{path}` at `{rev}`: {e}"))),
    }
}

fn load_delete_only_symbols(
    cache: &mut SymbolCache<'_>,
    rev: &str,
    path: &str,
) -> Result<(
    Vec<hotspots::symbols::SymbolFact>,
    Vec<hotspots::symbols::SymbolFact>,
)> {
    let parent = format!("{rev}^");
    match cache.symbols_for(&parent, path) {
        Ok(symbols_old) => Ok((Vec::new(), symbols_old)),
        Err(err) => Err(Error::msg(format!(
            "cannot load `{path}` at `{rev}` or `{parent}`: {err}"
        ))),
    }
}

fn load_old_symbols(
    cache: &mut SymbolCache<'_>,
    rev: &str,
    path: &str,
) -> Vec<hotspots::symbols::SymbolFact> {
    let parent = format!("{rev}^");
    cache.symbols_for(&parent, path).unwrap_or_default()
}

fn parse_symbols(path: &str, source: &str) -> Result<Vec<hotspots::symbols::SymbolFact>> {
    symbols_from_source(path, source)
}

#[cfg(test)]
mod tests;
