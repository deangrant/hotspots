//! Legacy Git log format using `[hash] author date subject` headers.

use std::io::BufRead;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::parse::{VcsParser, parse_numstat_stream};

/// Parser for the legacy bracketed Git log format.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitLegacyParser;

impl VcsParser for GitLegacyParser {
    fn parse(&self, input: &mut dyn BufRead) -> Result<Vec<Change>> {
        parse_numstat_stream(input, parse_header)
    }
}

fn parse_header(line: &str) -> Result<Option<(String, String, String)>> {
    if !line.starts_with('[') {
        return Ok(None);
    }
    let close = line
        .find(']')
        .ok_or_else(|| Error::msg(format!("legacy header missing `]`: {line}")))?;
    let rev = &line[1..close];
    if rev.is_empty() {
        return Err(Error::msg(format!("legacy header missing rev: {line}")));
    }
    let rest = line[close + 1..].trim_start();
    let date_at = find_iso_date(rest)
        .ok_or_else(|| Error::msg(format!("legacy header missing date: {line}")))?;
    let author = rest[..date_at].trim();
    if author.is_empty() {
        return Err(Error::msg(format!("legacy header missing author: {line}")));
    }
    let date = &rest[date_at..date_at + 10];
    Ok(Some((rev.to_owned(), author.to_owned(), date.to_owned())))
}

fn find_iso_date(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let last = bytes.len() - 10;
    (0..=last).find(|&start| is_iso_date_at(bytes, start) && is_date_token(text, start))
}

fn is_iso_date_at(bytes: &[u8], start: usize) -> bool {
    let slice = &bytes[start..start + 10];
    slice[4] == b'-'
        && slice[7] == b'-'
        && slice[0..4].iter().all(u8::is_ascii_digit)
        && slice[5..7].iter().all(u8::is_ascii_digit)
        && slice[8..10].iter().all(u8::is_ascii_digit)
}

fn is_date_token(text: &str, start: usize) -> bool {
    let before_ok = start == 0 || text.as_bytes()[start - 1].is_ascii_whitespace();
    let end = start + 10;
    let after_ok = end == text.len() || text.as_bytes()[end].is_ascii_whitespace();
    before_ok && after_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_legacy_git_log() {
        let log = "\
[abc123] Ada Lovelace 2024-01-02 fix bug
4\t1\tsrc/a.rs
[def456] Bea 2024-01-03 docs
0\t0\tREADME.md
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(parsed.is_ok(), "{:?}", parsed.err());
        let changes = parsed.unwrap_or_else(|_| Vec::new());
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].author, "Ada Lovelace");
        assert_eq!(changes[0].date, "2024-01-02");
        assert_eq!(changes[1].entity, "README.md");
    }
}
