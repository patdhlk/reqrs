//! Datetime helpers for the ReqIF date/time format.
//!
//! ReqIF uses ISO-8601 datetimes. Examples found in the corpus:
//! - `2021-07-01T01:12:06.749Z` (UTC with millisecond fraction)
//! - `2021-01-02T03:04:05+00:00` (explicit zero offset)
//! - `2013-01-01T00:00:00Z` (UTC, no fraction)
//!
//! Model fields like `ReqIfHeader::creation_time` stay as `Option<String>`
//! to preserve byte-for-byte round-trip — datetimes vary in subtle formatting
//! (millisecond precision, `Z` vs `+00:00`) and we don't want to normalize
//! on unparse. Use [`parse`] when you need a typed value for filtering,
//! comparison, or arithmetic.

use chrono::{DateTime, FixedOffset};

use crate::error::ReqIfError;

/// Parse a ReqIF datetime string into a typed [`chrono::DateTime`].
///
/// Accepts any RFC 3339 / ISO-8601 datetime, including the trailing-`Z` UTC
/// form and explicit timezone offsets.
pub fn parse(s: &str) -> Result<DateTime<FixedOffset>, ReqIfError> {
    DateTime::parse_from_rfc3339(s).map_err(|e| ReqIfError::Datetime {
        value: format!("{s} ({e})"),
    })
}

/// Format a [`chrono::DateTime`] into the canonical ReqIF form
/// (RFC 3339 with `Z` for UTC, millisecond precision).
pub fn format(dt: &DateTime<FixedOffset>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, /* use_z */ true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_utc_z_form() {
        let dt = parse("2021-07-01T01:12:06.749Z").unwrap();
        assert_eq!(dt.timestamp_millis(), 1625101926749);
    }

    #[test]
    fn parse_explicit_offset() {
        let dt = parse("2021-01-02T03:04:05+00:00").unwrap();
        assert_eq!(dt.timestamp(), 1609556645);
    }

    #[test]
    fn parse_non_utc_offset() {
        let dt = parse("2021-07-01T01:12:06+02:00").unwrap();
        assert_eq!(dt.timestamp(), 1625094726);
    }

    #[test]
    fn parse_returns_datetime_error_on_garbage() {
        let err = parse("not a date").unwrap_err();
        assert!(matches!(err, ReqIfError::Datetime { .. }));
    }

    #[test]
    fn format_round_trips_through_parse() {
        let original = "2021-07-01T01:12:06.749Z";
        let dt = parse(original).unwrap();
        assert_eq!(format(&dt), original);
    }
}
