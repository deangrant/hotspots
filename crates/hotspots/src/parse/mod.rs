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
    if let Some(parser) = known_parser(vcs) {
        return Ok(parser);
    }
    Err(unsupported_vcs(vcs))
}

fn known_parser(vcs: &str) -> Option<Box<dyn VcsParser>> {
    match vcs {
        "git" => Some(Box::new(GitLegacyParser)),
        "git2" => Some(Box::new(GitNumstatParser)),
        _ => None,
    }
}

fn unsupported_vcs(vcs: &str) -> Error {
    if is_planned_vcs(vcs) {
        return Error::msg(format!(
            "vcs `{vcs}` is not supported yet; use `git` or `git2`"
        ));
    }
    Error::msg(format!("unknown vcs `{vcs}`; expected `git` or `git2`"))
}

fn is_planned_vcs(vcs: &str) -> bool {
    ["svn", "hg", "p4", "tfs"].contains(&vcs)
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
    let (added_raw, deleted_raw, path) = numstat_fields(trimmed)?;
    Ok(Some((
        parse_numstat_count(added_raw)?,
        parse_numstat_count(deleted_raw)?,
        path.to_owned(),
    )))
}

fn numstat_fields(line: &str) -> Result<(&str, &str, &str)> {
    let mut parts = line.splitn(3, '\t');
    let added = next_field(&mut parts, "added count")?;
    let deleted = next_field(&mut parts, "deleted count")?;
    let path = next_field(&mut parts, "path")?;
    Ok((added, deleted, path))
}

fn next_field<'a>(parts: &mut impl Iterator<Item = &'a str>, label: &str) -> Result<&'a str> {
    parts.next().ok_or_else(|| Error::msg(format!("numstat line missing {label}")))
}

/// Shared header identity: revision, author, and date.
type CommitHeader = (String, String, String);

/// Parses a numstat log stream using a format-specific header recognizer.
///
/// The header callback returns `(rev, author, date)` when a commit header is
/// recognized.
pub(crate) fn parse_numstat_stream(
    input: &mut dyn BufRead,
    parse_header: impl Fn(&str) -> Result<Option<CommitHeader>>,
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    let mut current: Option<CommitHeader> = None;
    let mut line = String::new();
    while read_line(input, &mut line)? {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        process_stream_line(&mut changes, &mut current, &parse_header, trimmed)?;
    }
    Ok(changes)
}

fn read_line(input: &mut dyn BufRead, line: &mut String) -> Result<bool> {
    line.clear();
    Ok(input.read_line(line)? > 0)
}

fn process_stream_line(
    changes: &mut Vec<Change>,
    current: &mut Option<CommitHeader>,
    parse_header: &impl Fn(&str) -> Result<Option<CommitHeader>>,
    trimmed: &str,
) -> Result<()> {
    if let Some(header) = parse_header(trimmed)? {
        *current = Some(header);
        return Ok(());
    }
    append_under_header(changes, current.as_ref(), trimmed)
}

fn append_under_header(
    changes: &mut Vec<Change>,
    current: Option<&CommitHeader>,
    trimmed: &str,
) -> Result<()> {
    let Some((rev, author, date)) = current else {
        return reject_orphan_line(trimmed);
    };
    push_numstat_change(changes, rev, author, date, trimmed)
}

fn reject_orphan_line(trimmed: &str) -> Result<()> {
    if trimmed.is_empty() {
        return Ok(());
    }
    Err(Error::msg(format!("numstat line before header: {trimmed}")))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::parse::GitNumstatParser;

    #[test]
    fn parser_for_known_and_unknown() {
        assert!(parser_for("git").is_ok());
        assert!(parser_for("git2").is_ok());
        let svn = unsupported_vcs("svn").to_string();
        assert!(svn.contains("not supported yet"));
        let weird = unsupported_vcs("zzz").to_string();
        assert!(weird.contains("unknown vcs"));
    }

    #[test]
    fn parse_numstat_count_edges() {
        assert_eq!(parse_numstat_count("-").unwrap_or(1), 0);
        assert!(parse_numstat_count("3").is_ok());
        assert!(parse_numstat_count("x").is_err());
    }

    #[test]
    fn split_numstat_line_edges() {
        assert!(split_numstat_line("").unwrap_or(Some((0, 0, String::new()))).is_none());
        assert!(split_numstat_line("1\t2\tpath.rs").is_ok());
        assert!(split_numstat_line("1\t2").is_err());
    }

    #[test]
    fn orphan_line_rules() {
        assert!(reject_orphan_line("").is_ok());
        assert!(reject_orphan_line("1\t0\ta.rs").is_err());
    }

    #[test]
    fn blank_lines_under_header_are_skipped() {
        let log = b"--a--2024-01-01--Ada\n\n1\t0\ta.rs\n";
        let parsed = GitNumstatParser.parse(&mut Cursor::new(log));
        assert!(parsed.is_ok(), "{:?}", parsed.err());
        assert_eq!(parsed.unwrap_or_default().len(), 1);
    }
}
