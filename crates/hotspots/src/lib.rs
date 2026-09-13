//! Library for mining Git history logs into maintenance metrics.
//!
//! Parse exported Git logs into normalized change events, filter noisy paths,
//! and run analyses such as churn, ownership, coupling, and age.

#![doc(html_no_source)]

pub mod analysis;
pub mod date;
pub mod error;
pub mod filter;
pub mod group;
pub mod index;
pub mod model;
pub mod options;
pub mod output;
pub mod parse;
pub mod pipeline;
pub mod symbols;

#[cfg(test)]
mod coverage_tests;

#[doc(inline)]
pub use analysis::{Table, analysis_names};
#[doc(inline)]
pub use error::{Error, ErrorKind, Result};
#[doc(inline)]
pub use model::Change;
#[doc(inline)]
pub use options::{Grain, MAX_CHANGESET_SIZE_LIMIT, Options, TemporalPeriod};
#[doc(inline)]
pub use output::{OutputFormat, write_json, write_table, write_text};
#[doc(inline)]
pub use pipeline::{analyze_log, analyze_log_with_resolver};
#[doc(inline)]
pub use symbols::{ExpandStats, SymbolResolver};
