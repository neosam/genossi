//! Foundation für v1.2 Mitgliedschafts-Anpassungen (Kuendigung, Teil-Rueckgabe, Uebertrag, Aufstockung).
//!
//! Phase 14 liefert ausschliesslich die Pure-Function `compute_effective_date`. Phase 15-17
//! wird dieses Modul mit Service-Methoden + `MembershipAdjustService`-Trait erweitern.

use genossi_service::ValidationFailureItem;
use std::sync::Arc;
use time::Date;

/// Berechnet den Wirksamkeits-Stichtag nach Verbands-Konvention H1/H2.
///
/// **Konvention** (Verbands-Vorgabe, siehe `.planning/REQUIREMENTS.md` §CANC-02):
/// - H1 (Monat 1-6): Stichtag = 31.12. des laufenden Geschaeftsjahres, `fiscal_year` = aktuelles Jahr
/// - H2 (Monat 7-12): Stichtag = 31.12. des folgenden Geschaeftsjahres, `fiscal_year` = aktuelles Jahr + 1
///
/// Grenzwerte (siehe D-14-04..06):
/// - 30.06. zaehlt zu H1 (`month <= 6`)
/// - 01.07. zaehlt zu H2
/// - 31.12. zaehlt zu H2 -> Stichtag = 31.12. naechstes Jahr
/// - 29.02. (Schaltjahr) zaehlt zu H1 -> 31.12. desselben Jahres
///
/// Edge-Cases werden im `tests`-Submodul abgedeckt (D-14-14).
pub(crate) fn compute_effective_date(willensbekundung: Date) -> EffectiveDate {
    let fiscal_year = if (willensbekundung.month() as u8) <= 6 {
        willensbekundung.year()
    } else {
        willensbekundung.year() + 1
    };
    let effective_date = Date::from_calendar_date(fiscal_year, time::Month::December, 31)
        .expect("31. Dezember ist in jedem Jahr ein gueltiges Datum (kein Schalttag)");
    EffectiveDate {
        fiscal_year,
        effective_date,
    }
}

/// Ergebnis der H1/H2-Stichtagsberechnung (D-14-01).
///
/// `Copy`-able, weil `i32` und `time::Date` beide `Copy` sind. Vereinfacht
/// Call-Site-Pattern wie `let r = compute_effective_date(d); use r.fiscal_year`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EffectiveDate {
    pub fiscal_year: i32,
    pub effective_date: Date,
}

/// Validiert das Willensbekundungs-Datum gegen die Kalender-Jahr-Bounds (D-15-06, PERM-02).
///
/// Erlaubt sind nur das aktuelle und das naechste Kalender-Jahr (relativ zu `today`).
/// Die Funktion ist pure (kein clock-bezogener Aufruf wie `now_utc`, D-15-07), damit der
/// Aufrufer (Service-Layer in Plan 02/03) `today` kontrolliert testbar uebergeben kann.
pub(crate) fn validate_willensbekundung_date(
    date: Date,
    today: Date,
) -> Vec<ValidationFailureItem> {
    let current_fy = today.year();
    let next_fy = current_fy + 1;
    if date.year() == current_fy || date.year() == next_fy {
        Vec::new()
    } else {
        vec![ValidationFailureItem {
            field: Arc::from("willensbekundung_date"),
            message: Arc::from(format!(
                "must be in fiscal year {} or {}",
                current_fy, next_fy
            )),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_compute_effective_date_30_juni_is_h1() {
        let input = Date::from_calendar_date(2026, Month::June, 30).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_01_juli_is_h2() {
        let input = Date::from_calendar_date(2026, Month::July, 1).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2027);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2027, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_31_dezember_is_h2_next_year() {
        let input = Date::from_calendar_date(2026, Month::December, 31).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2027);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2027, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_01_januar_is_h1() {
        let input = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_schaltjahr_29_februar_is_h1() {
        let input = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2024);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2024, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_compute_effective_date_mittiges_datum_15_maerz_is_h1() {
        let input = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let result = compute_effective_date(input);
        assert_eq!(result.fiscal_year, 2026);
        assert_eq!(
            result.effective_date,
            Date::from_calendar_date(2026, Month::December, 31).unwrap()
        );
    }

    #[test]
    fn test_validate_willensbekundung_aktuelles_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_naechstes_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2027, Month::June, 15).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_vorjahr_invalid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2025, Month::December, 31).unwrap();
        let errors = validate_willensbekundung_date(date, today);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "willensbekundung_date");
        assert!(errors[0].message.contains("2026"));
        assert!(errors[0].message.contains("2027"));
    }

    #[test]
    fn test_validate_willensbekundung_uebernaechstes_jahr_invalid() {
        let today = Date::from_calendar_date(2026, Month::March, 15).unwrap();
        let date = Date::from_calendar_date(2028, Month::January, 1).unwrap();
        let errors = validate_willensbekundung_date(date, today);
        assert_eq!(errors.len(), 1);
        assert_eq!(&*errors[0].field, "willensbekundung_date");
    }

    #[test]
    fn test_validate_willensbekundung_today_31_dezember_naechstes_jahr_valid() {
        let today = Date::from_calendar_date(2026, Month::December, 31).unwrap();
        let date = Date::from_calendar_date(2027, Month::December, 31).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }

    #[test]
    fn test_validate_willensbekundung_schaltjahr_29_februar_valid() {
        let today = Date::from_calendar_date(2024, Month::January, 15).unwrap();
        let date = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        assert!(validate_willensbekundung_date(date, today).is_empty());
    }
}
