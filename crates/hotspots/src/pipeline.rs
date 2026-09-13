//! End-to-end pipeline from a log reader to a result table.

use std::io::BufRead;
use std::path::Path;

use crate::analysis::{self, Table};
use crate::error::Result;
use crate::filter::PathFilter;
use crate::group::LayerMap;
use crate::options::Options;
use crate::parse::VcsParser;

/// Parses, filters, optionally groups, and analyzes a VCS log.
///
/// # Errors
///
/// Returns an error when parsing, grouping, or analysis fails.
pub fn analyze_log(input: &mut dyn BufRead, vcs: &str, opts: &Options) -> Result<Table> {
    let parser = crate::parse::parser_for(vcs)?;
    analyze_with_parser(parser.as_ref(), input, opts)
}

fn analyze_with_parser(
    parser: &dyn VcsParser,
    input: &mut dyn BufRead,
    opts: &Options,
) -> Result<Table> {
    let changes = parser.parse(input)?;
    let filtered = PathFilter::new(opts.include.clone(), opts.exclude.clone()).apply(changes);
    let grouped = apply_grouping(filtered, opts)?;
    analysis::run(&opts.analysis, &grouped, opts)
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
