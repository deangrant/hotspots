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
    for (index, row) in table.rows.iter().enumerate() {
        write!(out, "  {{")?;
        for (col, header) in table.headers.iter().enumerate() {
            if col > 0 {
                write!(out, ", ")?;
            }
            let value = row.get(col).map_or("", String::as_str);
            write!(
                out,
                "\"{}\": \"{}\"",
                escape_json(header),
                escape_json(value)
            )?;
        }
        if index + 1 == table.rows.len() {
            writeln!(out, "}}")?;
        } else {
            writeln!(out, "}},")?;
        }
    }
    writeln!(out, "]")?;
    Ok(())
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
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
}
