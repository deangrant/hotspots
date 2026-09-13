//! Rust Git+`syn` symbol resolver for function-grain analysis.

#![doc(html_no_source)]

mod diff;
mod git;
mod parse;
mod resolve;

pub use resolve::RustGitSynResolver;
