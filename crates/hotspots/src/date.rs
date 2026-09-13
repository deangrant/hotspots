//! Calendar date helpers shared by parsers and age metrics.

use crate::error::{Error, Result};

/// Parses `YYYY-MM-DD` into year, month, and day parts.
///
/// # Errors
///
/// Returns an error when the date is not `YYYY-MM-DD` with numeric parts.
pub fn parse_date(date: &str) -> Result<(i64, i64, i64)> {
    let mut parts = date.split('-');
    let year = parse_part(parts.next(), date)?;
    let month = parse_part(parts.next(), date)?;
    let day = parse_part(parts.next(), date)?;
    if parts.next().is_some() {
        return Err(Error::msg(format!("invalid date `{date}`")));
    }
    Ok((year, month, day))
}

fn parse_part(part: Option<&str>, date: &str) -> Result<i64> {
    let raw = part.ok_or_else(|| Error::msg(format!("invalid date `{date}`")))?;
    raw.parse::<i64>().map_err(|_| Error::msg(format!("invalid date `{date}`")))
}

/// Approximate day ordinal for age differences (proleptic Gregorian).
///
/// # Errors
///
/// Returns an error when the date string cannot be parsed.
pub fn date_ordinal(date: &str) -> Result<i64> {
    let (y, m, d) = parse_date(date)?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_and_ordinal_edges() {
        assert!(parse_date("2024-01-02").is_ok());
        assert!(parse_date("bad").is_err());
        assert!(parse_date("2024").is_err());
        assert!(parse_date("2024-01-02-extra").is_err());
        assert!(parse_date("2024-xx-01").is_err());
        assert!(date_ordinal("2024-01-02").is_ok());
        assert!(date_ordinal("2024-03-01").is_ok());
    }
}
