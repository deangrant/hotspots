//! Aligned terminal-table writer for analysis results.

use std::io::Write;

use crate::analysis::Table;
use crate::error::Result;

/// Writes a table as an aligned text report.
///
/// # Errors
///
/// Returns an error when writing to `out` fails.
pub fn write_text(out: &mut dyn Write, table: &Table) -> Result<()> {
    let widths = col_widths(table);
    write_header(out, table, &widths)?;
    write_body(out, table, &widths)?;
    writeln!(out)?;
    writeln!(out, "{} rows.", table.rows.len())?;
    Ok(())
}

fn col_widths(table: &Table) -> Vec<usize> {
    let mut widths: Vec<usize> = table
        .headers
        .iter()
        .map(|header| display_width(&header.to_uppercase()))
        .collect();
    for row in &table.rows {
        for (col, width) in widths.iter_mut().enumerate() {
            let cell = row.get(col).map_or("", String::as_str);
            *width = (*width).max(display_width(cell));
        }
    }
    widths
}

fn write_header(out: &mut dyn Write, table: &Table, widths: &[usize]) -> Result<()> {
    let labels: Vec<String> = table.headers.iter().map(|h| h.to_uppercase()).collect();
    write_line(out, &labels, widths)?;
    Ok(())
}

fn write_body(out: &mut dyn Write, table: &Table, widths: &[usize]) -> Result<()> {
    for row in &table.rows {
        let cells: Vec<&str> = (0..table.headers.len())
            .map(|col| row.get(col).map_or("", String::as_str))
            .collect();
        write_line(out, &cells, widths)?;
    }
    Ok(())
}

fn write_line<S: AsRef<str>>(out: &mut dyn Write, cells: &[S], widths: &[usize]) -> Result<()> {
    write!(out, "  ")?;
    write_cells(out, cells, widths)?;
    writeln!(out)?;
    Ok(())
}

fn write_cells<S: AsRef<str>>(out: &mut dyn Write, cells: &[S], widths: &[usize]) -> Result<()> {
    for (col, cell) in cells.iter().enumerate() {
        write_one_cell(out, cell.as_ref(), cell_width(widths, col), col)?;
    }
    Ok(())
}

fn cell_width(widths: &[usize], col: usize) -> usize {
    widths.get(col).copied().unwrap_or(0)
}

fn write_one_cell(out: &mut dyn Write, value: &str, width: usize, col: usize) -> Result<()> {
    if col > 0 {
        write!(out, "  ")?;
    }
    write!(out, "{}", align_cell(value, width, col == 0))?;
    Ok(())
}

fn align_cell(value: &str, width: usize, left: bool) -> String {
    if left {
        format!("{value:<width$}")
    } else {
        format!("{value:>width$}")
    }
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_prints_header_and_footer() {
        let table = Table::with_headers(["entity", "risk"]);
        let mut buf = Vec::new();
        assert!(write_text(&mut buf, &table).is_ok());
        let text = String::from_utf8(buf).unwrap_or_default();
        assert!(text.contains("ENTITY"));
        assert!(text.contains("RISK"));
        assert!(text.contains("0 rows."));
    }

    #[test]
    fn aligns_first_left_and_others_right() {
        let mut table = Table::with_headers(["entity", "revs"]);
        table.push_row(["a.rs", "12"]);
        table.push_row(["longer/path.rs", "2"]);
        let mut buf = Vec::new();
        assert!(write_text(&mut buf, &table).is_ok());
        let text = String::from_utf8(buf).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("  ENTITY"));
        assert!(lines[1].starts_with("  a.rs"));
        assert!(lines[1].contains("12"));
        assert!(text.contains("2 rows."));
    }

    #[test]
    fn cell_width_falls_back_when_missing() {
        assert_eq!(cell_width(&[3, 4], 0), 3);
        assert_eq!(cell_width(&[3], 5), 0);
    }
}
