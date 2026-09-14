//! `SymbolResolver` implementation using Git and `syn`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use hotspots::symbols::{ExpandStats, FileDiff, SymbolResolver, expand_with_diffs, is_rust_path};
use hotspots::{Change, Error, Result};

use crate::diff::parse_hunks;
use crate::git::{
    GitRunner, SystemGit, ensure_work_tree, is_missing_path_error, show_blob, show_hunks,
};
use crate::parse::symbols_from_source;

/// Upper bound on concurrent git workers for function-grain expansion.
const MAX_GIT_JOBS: usize = 8;

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
        let diffs = expand_keys_parallel(&self.git, repo, rust_keys(changes))?;
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

fn git_job_count(key_count: usize) -> usize {
    if key_count == 0 {
        return 1;
    }
    thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1)
        .min(MAX_GIT_JOBS)
        .min(key_count)
        .max(1)
}

fn expand_keys_parallel(
    git: &dyn GitRunner,
    repo: &Path,
    keys: BTreeSet<(String, String)>,
) -> Result<BTreeMap<(String, String), FileDiff>> {
    let keys: Vec<(String, String)> = keys.into_iter().collect();
    if keys.is_empty() {
        return Ok(BTreeMap::new());
    }
    let slots = Mutex::new((0..keys.len()).map(|_| None).collect::<Vec<_>>());
    let next = AtomicUsize::new(0);
    let jobs = git_job_count(keys.len());
    // One job stays on the caller thread so test overrides (thread-local limits) apply.
    if jobs == 1 {
        fill_diff_slots(git, repo, &keys, &next, &slots);
    } else {
        thread::scope(|scope| {
            for _ in 0..jobs {
                scope.spawn(|| {
                    fill_diff_slots(git, repo, &keys, &next, &slots);
                });
            }
        });
    }
    collect_ordered_diffs(
        keys,
        slots.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

fn fill_diff_slots(
    git: &dyn GitRunner,
    repo: &Path,
    keys: &[(String, String)],
    next: &AtomicUsize,
    slots: &Mutex<Vec<Option<Result<FileDiff>>>>,
) {
    loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        if index >= keys.len() {
            return;
        }
        let (rev, path) = &keys[index];
        let mut cache = SymbolCache::new(git, repo);
        let built = build_file_diff(&mut cache, rev, path);
        if let Ok(mut guard) = slots.lock()
            && let Some(slot) = guard.get_mut(index)
        {
            *slot = Some(built);
        }
    }
}

fn collect_ordered_diffs(
    keys: Vec<(String, String)>,
    slots: Vec<Option<Result<FileDiff>>>,
) -> Result<BTreeMap<(String, String), FileDiff>> {
    let mut diffs = BTreeMap::new();
    for (key, slot) in keys.into_iter().zip(slots) {
        let diff = slot.ok_or_else(|| Error::msg("internal: missing parallel diff slot"))?;
        diffs.insert(key, diff?);
    }
    Ok(diffs)
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
