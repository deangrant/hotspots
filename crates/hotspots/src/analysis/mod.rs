//! Metric engines that turn change streams into tables.

mod age;
mod authors;
mod churn;
mod communication;
mod coupling;
mod effort;
mod hotspots;
mod identity;
mod ownership;
mod revisions;
mod risk;
mod soc;
mod summary;
mod table;
pub(crate) mod util;

#[cfg(test)]
pub(crate) mod fixtures;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::options::Options;

#[doc(inline)]
pub use table::Table;

type InfallibleAnalysis = fn(&[Change], &Options) -> Table;
type FallibleAnalysis = fn(&[Change], &Options) -> Result<Table>;

const INFALLIBLE: &[(&str, InfallibleAnalysis)] = &[
    ("summary", summary::run),
    ("authors", authors::run),
    ("revisions", revisions::run),
    ("hotspots", hotspots::run),
    ("risk", risk::run),
    ("coupling", coupling::run),
    ("soc", soc::run),
    ("entity-effort", effort::entity_effort),
    ("main-dev-by-revs", effort::main_dev_by_revs),
    ("fragmentation", effort::fragmentation),
    ("communication", communication::run),
    ("identity", identity::run),
];

const FALLIBLE: &[(&str, FallibleAnalysis)] = &[
    ("abs-churn", churn::abs_churn),
    ("author-churn", churn::author_churn),
    ("entity-churn", churn::entity_churn),
    ("entity-ownership", ownership::entity_ownership),
    ("main-dev", ownership::main_dev),
    ("age", age::run),
];

/// Runs the named analysis against `changes`.
///
/// # Errors
///
/// Returns an error when the analysis name is unknown or prerequisites fail.
pub fn run(name: &str, changes: &[Change], opts: &Options) -> Result<Table> {
    if let Some(analysis) = lookup(INFALLIBLE, name) {
        return Ok(analysis(changes, opts));
    }
    if let Some(analysis) = lookup(FALLIBLE, name) {
        return analysis(changes, opts);
    }
    Err(Error::msg(format!("unknown analysis `{name}`")))
}

fn lookup<T: Copy>(table: &[(&str, T)], name: &str) -> Option<T> {
    table.iter().find(|(key, _)| *key == name).map(|(_, f)| *f)
}

/// Names of analyses accepted by [`run`].
#[must_use]
pub const fn analysis_names() -> &'static [&'static str] {
    &[
        "abs-churn",
        "age",
        "author-churn",
        "authors",
        "communication",
        "coupling",
        "entity-churn",
        "entity-effort",
        "entity-ownership",
        "fragmentation",
        "hotspots",
        "identity",
        "main-dev",
        "main-dev-by-revs",
        "revisions",
        "risk",
        "soc",
        "summary",
    ]
}
