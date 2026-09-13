//! CSV and JSON writers for analysis tables.

use std::io::Write;

use crate::analysis::Table;
use crate::error::Result;
use crate::options::OutputFormat;

/// Writes a table using the selected output format.
///
/// # Errors
///
/// Returns an error when writing to `out` fails.
pub fn write_table(out: &mut dyn Write, table: &Table, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Csv => write_csv(out, table),
        OutputFormat::Json => write_json(out, table),
    }
}

fn write_csv(out: &mut dyn Write, table: &Table) -> Result<()> {
    writeln!(out, "{}", join_csv(&table.headers))?;
    for row in &table.rows {
        writeln!(out, "{}", join_csv(row))?;
    }
    Ok(())
}

fn join_csv(fields: &[String]) -> String {
    fields.iter().map(|field| escape_csv(field)).collect::<Vec<_>>().join(",")
}

fn escape_csv(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_owned()
    }
}

fn write_json(out: &mut dyn Write, table: &Table) -> Result<()> {
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
    fn csv_escapes_commas() {
        let mut table = Table::with_headers(["a", "b"]);
        table.push_row(["x,y", "z"]);
        let mut buf = Vec::new();
        let written = write_csv(&mut buf, &table);
        assert!(written.is_ok(), "{:?}", written.err());
        let text = String::from_utf8(buf).unwrap_or_default();
        assert!(text.contains("\"x,y\""));
    }
}
