//! Calendar date helpers shared by parsers and age metrics.

use crate::error::{Error, Result};

/// Parses `YYYY-MM-DD` into year, month, and day parts.
///
/// # Errors
///
/// Returns an error when the date is not a valid calendar `YYYY-MM-DD`.
pub fn parse_date(date: &str) -> Result<(i64, i64, i64)> {
    let mut parts = date.split('-');
    let year = parse_part(parts.next(), date)?;
    let month = parse_part(parts.next(), date)?;
    let day = parse_part(parts.next(), date)?;
    if parts.next().is_some() {
        return Err(invalid_date(date));
    }
    ensure_calendar_date(year, month, day, date)?;
    Ok((year, month, day))
}

fn parse_part(part: Option<&str>, date: &str) -> Result<i64> {
    let raw = part.ok_or_else(|| invalid_date(date))?;
    raw.parse::<i64>().map_err(|_| invalid_date(date))
}

fn ensure_calendar_date(year: i64, month: i64, day: i64, date: &str) -> Result<()> {
    if day < 1 || day > days_in_month(year, month) {
        return Err(invalid_date(date));
    }
    Ok(())
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn invalid_date(date: &str) -> Error {
    Error::msg(format!("invalid date `{date}`"))
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

    #[test]
    fn parse_date_rejects_invalid_calendar_days() {
        assert!(parse_date("2024-13-40").is_err());
        assert!(parse_date("9999-99-99").is_err());
        assert!(parse_date("2024-02-30").is_err());
        assert!(parse_date("2024-00-01").is_err());
        assert!(parse_date("2024-04-31").is_err());
    }

    #[test]
    fn parse_date_accepts_leap_and_thirty_day_months() {
        assert!(parse_date("2024-04-30").is_ok());
        assert!(parse_date("2024-02-29").is_ok());
        assert!(parse_date("2023-02-29").is_err());
    }
}
