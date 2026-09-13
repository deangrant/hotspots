//! End-to-end pipeline from a log reader to a result table.

use std::io::BufRead;
use std::path::Path;

use crate::analysis::{self, Table};
use crate::error::Result;
use crate::filter::PathFilter;
use crate::group::LayerMap;
use crate::options::Options;
use crate::parse::VcsParser;
use crate::symbols::{ExpandStats, SymbolResolver, apply_grain};

/// Parses, filters, optionally expands grain, groups, and analyzes a VCS log.
///
/// # Errors
///
/// Returns an error when parsing, grouping, resolution, or analysis fails.
pub fn analyze_log(input: &mut dyn BufRead, vcs: &str, opts: &Options) -> Result<Table> {
    analyze_log_with_resolver(input, vcs, opts, None).map(|(table, _)| table)
}

/// Like [`analyze_log`], but accepts a symbol resolver for function grain.
///
/// Returns the result table and expansion stats (non-Rust drops, etc.).
///
/// # Errors
///
/// Returns an error when parsing, grouping, resolution, or analysis fails.
pub fn analyze_log_with_resolver(
    input: &mut dyn BufRead,
    vcs: &str,
    opts: &Options,
    resolver: Option<&dyn SymbolResolver>,
) -> Result<(Table, ExpandStats)> {
    let parser = crate::parse::parser_for(vcs)?;
    analyze_with_parser(parser.as_ref(), input, opts, resolver)
}

fn analyze_with_parser(
    parser: &dyn VcsParser,
    input: &mut dyn BufRead,
    opts: &Options,
    resolver: Option<&dyn SymbolResolver>,
) -> Result<(Table, ExpandStats)> {
    let changes = parser.parse(input)?;
    let filtered = PathFilter::new(opts.include.clone(), opts.exclude.clone()).apply(changes);
    let repo = opts.repo.as_deref().map(Path::new);
    let (expanded, stats) = apply_grain(filtered, opts.grain, repo, resolver)?;
    let grouped = apply_grouping(expanded, opts)?;
    let table = analysis::run(&opts.analysis, &grouped, opts)?;
    Ok((table, stats))
}

fn apply_grouping(
    changes: Vec<crate::model::Change>,
    opts: &Options,
) -> Result<Vec<crate::model::Change>> {
    let Some(path) = &opts.group_file else {
        return Ok(changes);
    };
    let map = LayerMap::load(Path::new(path))?;
    Ok(map.apply(changes))
}
