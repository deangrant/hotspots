//! Preferred Git log format using `--hash--date--author` headers.

use std::io::BufRead;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::parse::{VcsParser, parse_numstat_stream};

/// Parser for the preferred Git numstat log format.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitNumstatParser;

impl VcsParser for GitNumstatParser {
    fn parse(&self, input: &mut dyn BufRead) -> Result<Vec<Change>> {
        parse_numstat_stream(input, parse_header)
    }
}

fn parse_header(line: &str) -> Result<Option<(String, String, String)>> {
    let Some(rest) = line.strip_prefix("--") else {
        return Ok(None);
    };
    parse_git2_fields(rest, line)
}

fn parse_git2_fields(rest: &str, line: &str) -> Result<Option<(String, String, String)>> {
    let mut parts = rest.splitn(3, "--");
    let rev = require_part(parts.next(), "rev")?;
    let date = require_part(parts.next(), "date")?;
    let author = require_part(parts.next(), "author")?;
    ensure_git2_fields(rev, date, author, line)?;
    Ok(Some((rev.to_owned(), author.to_owned(), date.to_owned())))
}

fn require_part<'a>(part: Option<&'a str>, label: &str) -> Result<&'a str> {
    part.ok_or_else(|| Error::msg(format!("git2 header missing {label}")))
}

fn ensure_git2_fields(rev: &str, date: &str, author: &str, line: &str) -> Result<()> {
    if rev.is_empty() || date.is_empty() || author.is_empty() {
        return Err(Error::msg(format!("malformed git2 header: {line}")));
    }
    crate::date::parse_date(date).map_err(|_| {
        Error::msg(format!(
            "malformed git2 header date (expected YYYY-MM-DD): {line}"
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_preferred_git_log() {
        let log = "\
--abc123--2024-01-02--Ada
10\t2\tsrc/a.rs
--def456--2024-01-03--Bea
-\t-\tbin/tool
";
        let parsed = GitNumstatParser.parse(&mut Cursor::new(log.as_bytes()));
        assert!(matches!(
            &parsed,
            Ok(c) if c.len() == 2
                && c[0].entity == "src/a.rs"
                && c[0].added == Some(10)
                && c[1].added == Some(0)
                && c[1].deleted == Some(0)
        ));
    }

    #[test]
    fn rejects_incomplete_git2_headers() {
        assert!(GitNumstatParser.parse(&mut Cursor::new("--onlyrev")).is_err());
        assert!(GitNumstatParser.parse(&mut Cursor::new("--rev----author")).is_err());
        assert!(GitNumstatParser.parse(&mut Cursor::new("----2024-01-01--Ada")).is_err());
        assert!(
            GitNumstatParser
                .parse(&mut Cursor::new("--abc--2024-01-01--\n1\t0\ta.rs\n"))
                .is_err()
        );
        assert!(
            GitNumstatParser
                .parse(&mut Cursor::new("--abc--not-a-date--Ada\n1\t0\ta.rs\n"))
                .is_err()
        );
    }
}
