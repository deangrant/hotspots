//! Tabular analysis results.

/// A named column header plus string cell rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    /// Column names in display order.
    pub headers: Vec<String>,
    /// Row values aligned with [`headers`](Self::headers).
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Creates an empty table with the given headers.
    pub fn with_headers(headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    /// Appends a row of cell values.
    pub fn push_row(&mut self, row: impl IntoIterator<Item = impl Into<String>>) {
        self.rows.push(row.into_iter().map(Into::into).collect());
    }

    /// Keeps at most `limit` rows when a limit is set.
    #[must_use]
    pub fn limit(mut self, limit: Option<usize>) -> Self {
        if let Some(max) = limit {
            self.rows.truncate(max);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_none_keeps_all_rows() {
        let mut table = Table::with_headers(["col"]);
        table.push_row(["a"]);
        table.push_row(["b"]);
        assert_eq!(table.clone().limit(None).rows.len(), 2);
        assert_eq!(table.limit(Some(1)).rows.len(), 1);
    }
}
