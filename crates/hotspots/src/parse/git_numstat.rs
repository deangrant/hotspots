//! Preferred Git log format using `--hash--date--author` headers.

use std::io::BufRead;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::parse::{VcsParser, push_numstat_change};

/// Parser for the preferred Git numstat log format.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitNumstatParser;

impl VcsParser for GitNumstatParser {
    fn parse(&self, input: &mut dyn BufRead) -> Result<Vec<Change>> {
        let mut changes = Vec::new();
        let mut current: Option<Header> = None;
        let mut line = String::new();
        loop {
            line.clear();
            let read = input.read_line(&mut line)?;
            if read == 0 {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(header) = parse_header(trimmed)? {
                current = Some(header);
                continue;
            }
            let Some(header) = current.as_ref() else {
                if trimmed.is_empty() {
                    continue;
                }
                return Err(Error::msg(format!("numstat line before header: {trimmed}")));
            };
            push_numstat_change(
                &mut changes,
                &header.rev,
                &header.author,
                &header.date,
                trimmed,
            )?;
        }
        Ok(changes)
    }
}

#[derive(Debug)]
struct Header {
    rev: String,
    date: String,
    author: String,
}

fn parse_header(line: &str) -> Result<Option<Header>> {
    if !line.starts_with("--") {
        return Ok(None);
    }
    let Some(rest) = line.strip_prefix("--") else {
        return Ok(None);
    };
    let mut parts = rest.splitn(3, "--");
    let rev = parts.next().ok_or_else(|| Error::msg("git2 header missing rev"))?;
    let date = parts.next().ok_or_else(|| Error::msg("git2 header missing date"))?;
    let author = parts.next().ok_or_else(|| Error::msg("git2 header missing author"))?;
    if rev.is_empty() || date.is_empty() {
        return Err(Error::msg(format!("malformed git2 header: {line}")));
    }
    Ok(Some(Header {
        rev: rev.to_owned(),
        date: date.to_owned(),
        author: author.to_owned(),
    }))
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
        assert!(parsed.is_ok(), "{:?}", parsed.err());
        let changes = parsed.unwrap_or_else(|_| Vec::new());
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].entity, "src/a.rs");
        assert_eq!(changes[0].added, Some(10));
        assert_eq!(changes[1].added, Some(0));
        assert_eq!(changes[1].deleted, Some(0));
    }
}
