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
mod soc;
mod summary;
mod table;
mod util;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::options::Options;

#[doc(inline)]
pub use table::Table;

/// Runs the named analysis against `changes`.
///
/// # Errors
///
/// Returns an error when the analysis name is unknown or prerequisites fail.
pub fn run(name: &str, changes: &[Change], opts: &Options) -> Result<Table> {
    match name {
        "summary" => Ok(summary::run(changes, opts)),
        "authors" => Ok(authors::run(changes, opts)),
        "revisions" => Ok(revisions::run(changes, opts)),
        "hotspots" => Ok(hotspots::run(changes, opts)),
        "coupling" => Ok(coupling::run(changes, opts)),
        "soc" => Ok(soc::run(changes, opts)),
        "abs-churn" => churn::abs_churn(changes, opts),
        "author-churn" => churn::author_churn(changes, opts),
        "entity-churn" => churn::entity_churn(changes, opts),
        "entity-ownership" => ownership::entity_ownership(changes, opts),
        "entity-effort" => Ok(effort::entity_effort(changes, opts)),
        "main-dev" => ownership::main_dev(changes, opts),
        "main-dev-by-revs" => Ok(effort::main_dev_by_revs(changes, opts)),
        "fragmentation" => Ok(effort::fragmentation(changes, opts)),
        "communication" => Ok(communication::run(changes, opts)),
        "age" => age::run(changes, opts),
        "identity" => Ok(identity::run(changes, opts)),
        other => Err(Error::msg(format!("unknown analysis `{other}`"))),
    }
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
        "soc",
        "summary",
    ]
}
