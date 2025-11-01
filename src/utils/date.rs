use chrono::{
    DateTime, NaiveDateTime, Utc,
    format::{DelayedFormat, StrftimeItems},
};
use thiserror::Error;

/// Turn the provided DateTime into a f64 representing the seconds with fractional
/// seconds for the sub-second milliseconds
pub fn datetime_to_f64(dt: DateTime<Utc>) -> f64 {
    let seconds = dt.timestamp() as f64;
    let millis = dt.timestamp_subsec_millis() as f64 / 1000.0;
    seconds + millis
}

#[derive(Debug, Error)]
pub enum AmzDateError {
    #[error(transparent)]
    Parse(#[from] chrono::ParseError),

    #[error("invalid date")]
    Invalid,
}

/// Format a date for the X-Amz-Date
pub fn format_amz_date(date: &DateTime<Utc>) -> String {
    format!("{}Z", date.format("%Y%m%dT%H%M%S"))
}

/// Format a date in the yyyymmdd format for credentials scopes
pub fn format_date_yyyymmdd(date: &DateTime<Utc>) -> String {
    date.format("%Y%m%d").to_string()
}

/// Parses the date value from the X-Amz-Date header
pub fn parse_amz_date(value: &str) -> Result<DateTime<Utc>, AmzDateError> {
    let value = value.strip_suffix('Z').ok_or(AmzDateError::Invalid)?;
    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")?;

    // Convert to UTC
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

const IMF_FIXDATE_PATTERN: &str = "%a, %d %b %Y %T GMT";
const RFC850_DATE_PATTERN: &str = "%A, %d-%b-%y %T GMT";
const ASCTIME_DATE_PATTERN: &str = "%a %b %e %T %Y";

pub struct InvalidHttpDate;

pub fn parse_http_date(value: &str) -> Result<DateTime<Utc>, InvalidHttpDate> {
    let naive = NaiveDateTime::parse_from_str(value, IMF_FIXDATE_PATTERN)
        .or_else(|_| NaiveDateTime::parse_from_str(value, RFC850_DATE_PATTERN))
        .or_else(|_| NaiveDateTime::parse_from_str(value, ASCTIME_DATE_PATTERN))
        .map_err(|_| InvalidHttpDate)?;

    Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike, Utc};

    #[test]
    fn test_whole_second() {
        let dt = Utc.with_ymd_and_hms(2025, 10, 31, 12, 0, 0).unwrap();
        let result = datetime_to_f64(dt);
        assert_eq!(result, dt.timestamp() as f64);
    }

    #[test]
    fn test_with_milliseconds() {
        let dt = Utc
            .with_ymd_and_hms(2025, 10, 31, 12, 0, 0)
            .unwrap()
            .with_nanosecond(123_000_000)
            .unwrap(); // 123 ms
        let result = datetime_to_f64(dt);
        let expected = dt.timestamp() as f64 + 0.123;
        assert!((result - expected).abs() < 1e-9);
    }

    #[test]
    fn test_near_second_rollover() {
        let dt = Utc
            .with_ymd_and_hms(2025, 10, 31, 12, 0, 59)
            .unwrap()
            .with_nanosecond(999_000_000)
            .unwrap(); // 999 ms
        let result = datetime_to_f64(dt);
        let expected = dt.timestamp() as f64 + 0.999;
        assert!((result - expected).abs() < 1e-9);
    }

    #[test]
    fn test_epoch() {
        let dt = Utc.timestamp_opt(0, 0).unwrap();
        let result = datetime_to_f64(dt);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_before_epoch() {
        let dt = Utc.timestamp_opt(-1, 500_000_000).unwrap(); // 0.5 seconds before epoch
        let result = datetime_to_f64(dt);
        let expected = -1.0 + 0.5; // should equal -0.5
        assert!((result - expected).abs() < 1e-9);
    }

    #[test]
    fn test_precision_check() {
        // A date far in the future with sub-second component
        let dt = Utc
            .with_ymd_and_hms(3000, 1, 1, 0, 0, 0)
            .unwrap()
            .with_nanosecond(987_000_000)
            .unwrap();
        let result = datetime_to_f64(dt);
        let expected = dt.timestamp() as f64 + 0.987;
        assert!((result - expected).abs() < 1e-9);
    }
}
