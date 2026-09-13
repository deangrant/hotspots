//! Git log parsers that emit normalized [`Change`] rows.

mod git_legacy;
mod git_numstat;

use std::io::BufRead;

use crate::error::{Error, Result};
use crate::model::Change;

#[doc(inline)]
pub use git_legacy::GitLegacyParser;
#[doc(inline)]
pub use git_numstat::GitNumstatParser;

/// Parses a VCS log into normalized change events.
pub trait VcsParser {
    /// Reads the full log from `input` and returns change rows.
    ///
    /// # Errors
    ///
    /// Returns an error when the log cannot be read or is malformed.
    fn parse(&self, input: &mut dyn BufRead) -> Result<Vec<Change>>;
}

/// Selects a parser for a supported VCS name.
///
/// # Errors
///
/// Returns an error when the VCS name is unknown or not yet supported.
pub fn parser_for(vcs: &str) -> Result<Box<dyn VcsParser>> {
    match vcs {
        "git" => Ok(Box::new(GitLegacyParser)),
        "git2" => Ok(Box::new(GitNumstatParser)),
        "svn" | "hg" | "p4" | "tfs" => Err(Error::msg(format!(
            "vcs `{vcs}` is not supported yet; use `git` or `git2`"
        ))),
        other => Err(Error::msg(format!(
            "unknown vcs `{other}`; expected `git` or `git2`"
        ))),
    }
}

/// Parses a numstat added or deleted field.
///
/// A dash (`-`) means a binary file and becomes zero.
fn parse_numstat_count(raw: &str) -> Result<u64> {
    if raw == "-" {
        return Ok(0);
    }
    raw.parse::<u64>()
        .map_err(|_| Error::msg(format!("invalid numstat count `{raw}`")))
}

/// Splits a numstat line into added, deleted, and path.
fn split_numstat_line(line: &str) -> Result<Option<(u64, u64, String)>> {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut parts = trimmed.splitn(3, '\t');
    let added_raw = parts.next().ok_or_else(|| Error::msg("numstat line missing added count"))?;
    let deleted_raw =
        parts.next().ok_or_else(|| Error::msg("numstat line missing deleted count"))?;
    let path = parts.next().ok_or_else(|| Error::msg("numstat line missing path"))?;
    let added = parse_numstat_count(added_raw)?;
    let deleted = parse_numstat_count(deleted_raw)?;
    Ok(Some((added, deleted, path.to_owned())))
}

fn push_numstat_change(
    out: &mut Vec<Change>,
    rev: &str,
    author: &str,
    date: &str,
    line: &str,
) -> Result<()> {
    let Some((added, deleted, entity)) = split_numstat_line(line)? else {
        return Ok(());
    };
    out.push(Change::new(
        rev,
        author,
        date,
        entity,
        Some(added),
        Some(deleted),
    ));
    Ok(())
}
