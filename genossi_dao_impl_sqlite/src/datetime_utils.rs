//! Zentrale Datetime-Helper für den SQLite-DAO-Layer. Vorher waren
//! `parse_datetime` (8x), `format_dt` (6x) und `parse_date`/`format_date` (2x)
//! identisch über die DAO-Dateien kopiert — ein Format-Bug hätte an jeder Kopie
//! gefixt werden müssen (dedup-datetime-helpers).

use genossi_dao::DaoError;
use std::sync::Arc;
use time::PrimitiveDateTime;

/// Parse a datetime string written by either our ISO8601 ser code or the SQLite
/// `CURRENT_TIMESTAMP`/default text format.
pub(crate) fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    if let Ok(dt) =
        PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
    {
        return Ok(dt);
    }
    let sqlite_format =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]")
            .unwrap();
    if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
        return Ok(dt);
    }
    let sqlite_simple =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    PrimitiveDateTime::parse(s, &sqlite_simple)
}

/// Format a datetime as ISO8601 (UTC), matching our serialization format.
pub(crate) fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}

/// Parse a `YYYY-MM-DD` date string.
pub(crate) fn parse_date(s: &str) -> Result<time::Date, time::error::Parse> {
    let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
    time::Date::parse(s, &format)
}

/// Format a date as `YYYY-MM-DD`.
pub(crate) fn format_date(d: &time::Date) -> String {
    let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
    d.format(&format).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_iso8601() {
        let dt = parse_datetime("2026-06-25T08:06:09.716000000Z").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month() as u8, 6);
        assert_eq!(dt.day(), 25);
    }

    #[test]
    fn test_parse_datetime_sqlite_with_subsecond() {
        let dt = parse_datetime("2026-06-25 08:06:09.716").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.minute(), 6);
    }

    #[test]
    fn test_parse_datetime_sqlite_simple() {
        let dt = parse_datetime("2026-06-25 08:06:09").unwrap();
        assert_eq!(dt.second(), 9);
    }

    #[test]
    fn test_format_dt_roundtrip() {
        let dt = parse_datetime("2026-06-25T08:06:09.716000000Z").unwrap();
        let s = format_dt(&dt).unwrap();
        let reparsed = parse_datetime(&s).unwrap();
        assert_eq!(dt, reparsed);
    }

    #[test]
    fn test_parse_and_format_date_roundtrip() {
        let d = parse_date("2026-06-25").unwrap();
        assert_eq!(format_date(&d), "2026-06-25");
    }
}
