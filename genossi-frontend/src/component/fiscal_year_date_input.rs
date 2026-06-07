//! Phase 18 D-18-09..11 + UI-03 — Wiederverwendbarer Datepicker mit Geschaeftsjahr-Bounds.
//!
//! Erlaubter Bereich: aktuelles Kalenderjahr UND naechstes Kalenderjahr (heuristic mirror
//! des Backend-Validators `genossi_service_impl/src/membership_adjust.rs:739-756`).
//! Frontend-Validation ist Defense-in-Depth — Backend ist Single-Source-of-Truth.
//!
//! Default-Today-Pattern (SC-2 "default today()"): Die Default-today()-Verantwortung liegt
//! beim CALLER (Modal-Body initialisiert `use_signal(|| Some(today))`). Diese Component
//! akzeptiert deshalb `value: Signal<Option<Date>>` ohne separaten `default`-Prop —
//! der Caller kontrolliert den Initialwert explizit.

use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

/// Phase 18 — Mirror of page/member_details.rs:30-32 (minimal duplication per PATTERNS L-7).
fn format_date_input(d: &time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// Phase 18 — Mirror of page/member_details.rs:63-79.
fn parse_date_input(s: &str) -> Option<time::Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;
    let month: time::Month = month.try_into().ok()?;
    time::Date::from_calendar_date(year, month, day).ok()
}

/// Phase 18 D-18-11 — Pure helper, frontend mirror of backend `is_valid_fiscal_year_date`
/// (genossi_service_impl/src/membership_adjust.rs:739-756).
/// Returns `true` if `date.year()` equals `today.year()` or `today.year() + 1`.
pub fn is_valid_fiscal_year_date(date: time::Date, today: time::Date) -> bool {
    let current_fy = today.year();
    date.year() == current_fy || date.year() == current_fy + 1
}

#[component]
pub fn FiscalYearDateInput(
    value: Signal<Option<time::Date>>,
    on_change: EventHandler<time::Date>,
    today: time::Date,
) -> Element {
    let i18n = use_i18n();
    let min_year = today.year();
    let max_year = today.year() + 1;
    let min_str = format!("{:04}-01-01", min_year);
    let max_str = format!("{:04}-12-31", max_year);

    let value_read = value.read();
    let current_value_str = value_read.as_ref().map(format_date_input).unwrap_or_default();
    let is_oor = value_read
        .as_ref()
        .map_or(false, |d| !is_valid_fiscal_year_date(*d, today));
    drop(value_read);

    let border_class = if is_oor {
        "border-red-500"
    } else {
        "border-gray-300"
    };

    // L-4 Mitigation: i18n.t() returns Rc<str>; format-args via .replace()
    let helper_template = i18n.t(Key::FiscalYearDateInputHelper);
    let helper_text = helper_template
        .replace("{min_year}", &min_year.to_string())
        .replace("{max_year}", &max_year.to_string());
    let oor_text = i18n.t(Key::FiscalYearDateOutOfRange).to_string();

    rsx! {
        div { class: "flex flex-col gap-1",
            input {
                r#type: "date",
                min: "{min_str}",
                max: "{max_str}",
                value: "{current_value_str}",
                class: "w-full px-3 py-2 border {border_class} rounded focus:ring-2 focus:ring-blue-500",
                oninput: move |e| {
                    let s = e.value();
                    if s.is_empty() {
                        value.set(None);
                    } else if let Some(d) = parse_date_input(&s) {
                        value.set(Some(d));
                        on_change.call(d);
                    }
                }
            }
            if is_oor {
                span { class: "text-red-600 text-sm", "{oor_text}" }
            }
            span { class: "text-gray-500 text-xs", "{helper_text}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month};

    fn d(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).unwrap()
    }

    #[test]
    fn is_valid_fiscal_year_date_current_year() {
        let today = d(2026, Month::June, 15);
        assert!(is_valid_fiscal_year_date(d(2026, Month::August, 1), today));
        assert!(is_valid_fiscal_year_date(d(2026, Month::January, 1), today));
        assert!(is_valid_fiscal_year_date(
            d(2026, Month::December, 31),
            today
        ));
    }

    #[test]
    fn is_valid_fiscal_year_date_next_year() {
        let today = d(2026, Month::June, 15);
        assert!(is_valid_fiscal_year_date(d(2027, Month::March, 1), today));
        assert!(is_valid_fiscal_year_date(
            d(2027, Month::December, 31),
            today
        ));
    }

    #[test]
    fn is_valid_fiscal_year_date_prev_year_rejected() {
        let today = d(2026, Month::June, 15);
        assert!(!is_valid_fiscal_year_date(
            d(2025, Month::December, 31),
            today
        ));
        assert!(!is_valid_fiscal_year_date(
            d(2025, Month::January, 1),
            today
        ));
    }

    #[test]
    fn is_valid_fiscal_year_date_year_after_next_rejected() {
        let today = d(2026, Month::June, 15);
        assert!(!is_valid_fiscal_year_date(
            d(2028, Month::January, 1),
            today
        ));
        assert!(!is_valid_fiscal_year_date(d(2030, Month::June, 15), today));
    }

    #[test]
    fn parse_date_input_round_trip() {
        let d_in = d(2026, Month::June, 15);
        let s = format_date_input(&d_in);
        assert_eq!(s, "2026-06-15");
        let d_out = parse_date_input(&s).unwrap();
        assert_eq!(d_out, d_in);
    }

    #[test]
    fn parse_date_input_rejects_garbage() {
        assert!(parse_date_input("").is_none());
        assert!(parse_date_input("not-a-date").is_none());
        assert!(parse_date_input("2026-13-01").is_none());
    }
}
