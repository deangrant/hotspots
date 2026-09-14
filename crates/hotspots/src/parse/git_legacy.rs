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
    let Some((rev, rest)) = split_legacy_prefix(line)? else {
        return Ok(None);
    };
    parse_legacy_body(rev, rest, line)
}

fn split_legacy_prefix(line: &str) -> Result<Option<(&str, &str)>> {
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
    Ok(Some((rev, line[close + 1..].trim_start())))
}

fn parse_legacy_body(
    rev: &str,
    rest: &str,
    line: &str,
) -> Result<Option<(String, String, String)>> {
    if let Some((author, date)) = split_delimited_date(rest) {
        return finish_header(rev, author, date, line);
    }
    let date_at = find_iso_date(rest)
        .ok_or_else(|| Error::msg(format!("legacy header missing date: {line}")))?;
    let author = rest[..date_at].trim();
    let date = &rest[date_at..date_at + 10];
    finish_header(rev, author, date, line)
}

fn finish_header(
    rev: &str,
    author: &str,
    date: &str,
    line: &str,
) -> Result<Option<(String, String, String)>> {
    if author.is_empty() {
        return Err(Error::msg(format!("legacy header missing author: {line}")));
    }
    crate::date::parse_date(date)?;
    Ok(Some((rev.to_owned(), author.to_owned(), date.to_owned())))
}

/// Splits `author <YYYY-MM-DD> subject` when an angle-bracket ISO date is present.
fn split_delimited_date(rest: &str) -> Option<(&str, &str)> {
    let open = rest.find('<')?;
    let after_open = open + 1;
    let close = rest[after_open..].find('>')? + after_open;
    let date = &rest[after_open..close];
    if date.len() != 10 || !is_iso_date_at(date.as_bytes(), 0) {
        return None;
    }
    let before = &rest.as_bytes()[..open];
    let after = rest.as_bytes().get(close + 1).copied();
    if !boundary_byte(before.last().copied()) || !boundary_byte(after) {
        return None;
    }
    Some((rest[..open].trim(), date))
}

fn boundary_byte(byte: Option<u8>) -> bool {
    byte.is_none_or(|b| b.is_ascii_whitespace())
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
    has_date_separators(slice) && has_numeric_date_parts(slice)
}

fn has_date_separators(slice: &[u8]) -> bool {
    slice[4] == b'-' && slice[7] == b'-'
}

fn has_numeric_date_parts(slice: &[u8]) -> bool {
    digits(slice, 0, 4) && digits(slice, 5, 7) && digits(slice, 8, 10)
}

fn digits(slice: &[u8], from: usize, to: usize) -> bool {
    slice[from..to].iter().all(u8::is_ascii_digit)
}

fn is_date_token(text: &str, start: usize) -> bool {
    boundary_before(text, start) && boundary_after(text, start + 10)
}

fn boundary_before(text: &str, start: usize) -> bool {
    start == 0 || text.as_bytes()[start - 1].is_ascii_whitespace()
}

fn boundary_after(text: &str, end: usize) -> bool {
    end == text.len() || text.as_bytes()[end].is_ascii_whitespace()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_legacy_git_log() {
        // dry-rs:ignore. Parallel legacy parse/assert fixtures; structural twins are intentional.
        let log = "\
[abc123] Ada Lovelace 2024-01-02 fix bug
4\t1\tsrc/a.rs
[def456] Bea 2024-01-03 docs
0\t0\tREADME.md
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 2
                && c[0].author == "Ada Lovelace"
                && c[0].date == "2024-01-02"
                && c[1].entity == "README.md"
        ));
    }

    #[test]
    fn rejects_malformed_headers() {
        // dry-rs:ignore. Parallel is_err header fixtures; structural twins are intentional.
        assert!(GitLegacyParser.parse(&mut Cursor::new("[abc] subject without date")).is_err());
        assert!(GitLegacyParser.parse(&mut Cursor::new("[abc]Ada2024-01-01 no-space")).is_err());
        assert!(GitLegacyParser.parse(&mut Cursor::new("[abc] short")).is_err());
        assert!(GitLegacyParser.parse(&mut Cursor::new("[abc] Ada 2024-13-40 subject")).is_err());
    }

    #[test]
    fn delimited_date_keeps_date_like_author_names() {
        // dry-rs:ignore. Parallel legacy parse/assert fixtures; structural twins are intentional.
        let log = "\
[abc123] Ada 2024-01-01 Fan <2024-01-02> subject here
1\t0\ta.rs
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 1
                && c[0].author == "Ada 2024-01-01 Fan"
                && c[0].date == "2024-01-02"
                && c[0].entity == "a.rs"
        ));
    }

    #[test]
    fn undelimited_subject_date_keeps_first_iso_token() {
        // dry-rs:ignore. Parallel legacy parse/assert fixtures; structural twins are intentional.
        let log = "\
[abc123] Ada Lovelace 2024-01-02 fixed 2023-12-01 regression
1\t0\ta.rs
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 1
                && c[0].author == "Ada Lovelace"
                && c[0].date == "2024-01-02"
        ));
    }

    #[test]
    fn ignores_non_iso_angle_brackets() {
        // dry-rs:ignore. Parallel legacy parse/assert fixtures; structural twins are intentional.
        let log = "\
[abc123] Ada <not-a-date> 2024-01-02 subject
1\t0\ta.rs
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 1
                && c[0].author == "Ada <not-a-date>"
                && c[0].date == "2024-01-02"
        ));
    }

    #[test]
    fn glued_angle_date_falls_back_to_bare_iso() {
        // dry-rs:ignore. Parallel legacy parse/assert fixtures; structural twins are intentional.
        let log = "\
[abc123] Ada<2024-01-01>Fan 2024-01-02 subject
1\t0\ta.rs
";
        let parsed = GitLegacyParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 1
                && c[0].author == "Ada<2024-01-01>Fan"
                && c[0].date == "2024-01-02"
        ));
    }
}
