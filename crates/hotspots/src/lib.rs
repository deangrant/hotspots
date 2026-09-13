//! Library for mining Git history logs into maintenance metrics.
//!
//! Parse exported Git logs into normalized change events, filter noisy paths,
//! and run analyses such as churn, ownership, coupling, and age.

#![doc(html_no_source)]

pub mod analysis;
pub mod error;
pub mod filter;
pub mod group;
pub mod index;
pub mod model;
pub mod options;
pub mod output;
pub mod parse;
pub mod pipeline;

#[doc(inline)]
pub use analysis::{Table, analysis_names};
#[doc(inline)]
pub use error::{Error, Result};
#[doc(inline)]
pub use model::Change;
#[doc(inline)]
pub use options::{Options, TemporalPeriod};
#[doc(inline)]
pub use output::write_table;
#[doc(inline)]
pub use pipeline::analyze_log;
