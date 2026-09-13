//! Writers for analysis tables (terminal text and JSON).

mod json;
mod text;

use std::io::Write;

use crate::analysis::Table;
use crate::error::Result;

pub use json::write_json;
pub use text::write_text;

/// Supported stdout encodings for analysis results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Aligned terminal table (default).
    #[default]
    Text,
    /// JSON array of flat objects.
    Json,
}

impl OutputFormat {
    /// Parses `text` or `json`.
    ///
    /// # Errors
    ///
    /// Returns an error when `raw` is not a known format name.
    pub fn parse(raw: &str) -> std::result::Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "invalid format `{other}`; expected `text` or `json`"
            )),
        }
    }
}

/// Writes a table using the selected output format.
///
/// # Errors
///
/// Returns an error when writing to `out` fails.
pub fn write_table(out: &mut dyn Write, table: &Table, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Text => write_text(out, table),
        OutputFormat::Json => write_json(out, table),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_table_dispatches_formats() {
        let mut table = Table::with_headers(["a"]);
        table.push_row(["x"]);
        let mut text_buf = Vec::new();
        assert!(write_table(&mut text_buf, &table, OutputFormat::Text).is_ok());
        assert!(String::from_utf8(text_buf).unwrap_or_default().contains("1 rows."));
        let mut json_buf = Vec::new();
        assert!(write_table(&mut json_buf, &table, OutputFormat::Json).is_ok());
        assert!(String::from_utf8(json_buf).unwrap_or_default().starts_with('['));
    }

    #[test]
    fn parse_format_names() {
        assert_eq!(OutputFormat::parse("text").ok(), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::parse("json").ok(), Some(OutputFormat::Json));
        assert!(OutputFormat::parse("csv").is_err());
    }
}
