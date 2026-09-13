//! JSON writer for analysis tables.

use std::io::Write;

use crate::analysis::Table;
use crate::error::Result;

/// Writes a table as a JSON array of flat objects.
///
/// # Errors
///
/// Returns an error when writing to `out` fails.
pub fn write_table(out: &mut dyn Write, table: &Table) -> Result<()> {
    writeln!(out, "[")?;
    write_rows(out, table)?;
    writeln!(out, "]")?;
    Ok(())
}

fn write_rows(out: &mut dyn Write, table: &Table) -> Result<()> {
    let last = table.rows.len().saturating_sub(1);
    for (index, row) in table.rows.iter().enumerate() {
        write_object(out, &table.headers, row, index == last)?;
    }
    Ok(())
}

fn write_object(
    out: &mut dyn Write,
    headers: &[String],
    row: &[String],
    is_last: bool,
) -> Result<()> {
    write!(out, "  {{")?;
    write_fields(out, headers, row)?;
    write_object_end(out, is_last)
}

fn write_fields(out: &mut dyn Write, headers: &[String], row: &[String]) -> Result<()> {
    for (col, header) in headers.iter().enumerate() {
        write_field(
            out,
            header,
            row.get(col).map_or("", String::as_str),
            col > 0,
        )?;
    }
    Ok(())
}

fn write_field(out: &mut dyn Write, header: &str, value: &str, leading_comma: bool) -> Result<()> {
    if leading_comma {
        write!(out, ", ")?;
    }
    write!(
        out,
        "\"{}\": \"{}\"",
        escape_json(header),
        escape_json(value)
    )?;
    Ok(())
}

fn write_object_end(out: &mut dyn Write, is_last: bool) -> Result<()> {
    if is_last {
        writeln!(out, "}}")?;
    } else {
        writeln!(out, "}},")?;
    }
    Ok(())
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        append_escaped(&mut out, ch);
    }
    out
}

fn append_escaped(out: &mut String, ch: char) {
    if let Some(escaped) = simple_escape(ch) {
        out.push_str(escaped);
        return;
    }
    if ch.is_control() {
        push_unicode_escape(out, ch);
        return;
    }
    out.push(ch);
}

fn simple_escape(ch: char) -> Option<&'static str> {
    const TABLE: &[(char, &str)] = &[
        ('"', "\\\""),
        ('\\', "\\\\"),
        ('\n', "\\n"),
        ('\r', "\\r"),
        ('\t', "\\t"),
    ];
    TABLE.iter().find(|(c, _)| *c == ch).map(|(_, s)| *s)
}

fn push_unicode_escape(out: &mut String, ch: char) {
    use std::fmt::Write as _;
    let _ = write!(out, "\\u{:04x}", u32::from(ch));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_quotes() {
        let mut table = Table::with_headers(["a", "b"]);
        table.push_row(["x\"y", "z"]);
        let mut buf = Vec::new();
        let written = write_table(&mut buf, &table);
        assert!(written.is_ok(), "{:?}", written.err());
        let text = String::from_utf8(buf).unwrap_or_default();
        assert!(text.contains("\\\""));
        assert!(text.starts_with('['));
    }

    #[test]
    fn empty_table_is_empty_array() {
        let table = Table::with_headers(["a"]);
        let mut buf = Vec::new();
        assert!(write_table(&mut buf, &table).is_ok());
        assert_eq!(String::from_utf8(buf).unwrap_or_default(), "[\n]\n");
    }

    #[test]
    fn escapes_backslash_tab_cr_and_control() {
        let mut table = Table::with_headers(["h"]);
        table.push_row(["\\\t\r\u{0001}"]);
        let mut buf = Vec::new();
        assert!(write_table(&mut buf, &table).is_ok());
        let text = String::from_utf8(buf).unwrap_or_default();
        assert!(text.contains("\\\\"));
        assert!(text.contains("\\t"));
        assert!(text.contains("\\r"));
        assert!(text.contains("\\u0001"));
    }

    #[test]
    fn short_row_and_multi_row_terminators() {
        let mut table = Table::with_headers(["a", "b"]);
        table.rows.push(vec![String::from("only")]);
        table.push_row(["x", "y"]);
        let mut buf = Vec::new();
        assert!(write_table(&mut buf, &table).is_ok());
        let text = String::from_utf8(buf).unwrap_or_default();
        assert!(text.contains("\"b\": \"\""));
        assert!(text.contains("},"));
    }

    #[test]
    fn write_errors_propagate() {
        struct FailAfter {
            ok_writes: usize,
        }
        impl Write for FailAfter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.ok_writes == 0 {
                    return Err(std::io::Error::other("fail"));
                }
                self.ok_writes -= 1;
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut table = Table::with_headers(["a"]);
        table.push_row(["x"]);
        let mut fail = FailAfter { ok_writes: 0 };
        assert!(fail.flush().is_ok());
        assert!(write_table(&mut fail, &table).is_err());
        assert!(write_table(&mut FailAfter { ok_writes: 2 }, &table).is_err());
        assert!(write_table(&mut FailAfter { ok_writes: 5 }, &table).is_err());
    }
}
