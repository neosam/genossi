//! Phase 12 — Format- und Parse-Helper für Auszahlungs-Beträge.
//!
//! `format_payout_eur` liefert deutsche Formatierung mit Euro-Symbol
//! „60,00 €" wie D-10 verlangt. `i18n::format_price` liefert „60,00 EUR"
//! — daher dieser eigene Helper.
//!
//! `parse_euro_to_cents` ist die Umkehrung für User-Inputs: akzeptiert
//! Komma- ODER Punkt-Dezimaltrennung, trimmt Whitespace, lehnt 0 und
//! negative Werte ab (Phase-7 D-12: share_value > 0 ist Backend-CHECK).
//!
//! Beide Funktionen werden in Plan 12-04 (Create-Modal), Plan 12-06
//! (share_value-Inline-Edit) und Plan 12-08 (Betrag-Spalte) verwendet —
//! KEINE lokalen Duplikate.
//!
//! ## Berechnung
//!
//! `total_cents = share_count × share_value_cents`. Bei Multiplikation
//! kann ein i32-Cast zu i64 sich nicht überlaufen, da i32::MAX × i64
//! Bit-Width klein genug bleibt. Negative Werte werden mit Vorzeichen
//! formatiert (z.B. `format_payout_eur(-1, 100)` = `"-1,00 €"`).

/// Formats a share-count × share-value-in-cents combination as German
/// EUR string (e.g. `format_payout_eur(60, 100) == "60,00 €"`).
///
/// Always two decimal digits, comma as decimal separator, trailing
/// non-breaking-space-and-€ glyph.
pub fn format_payout_eur(share_count: i32, share_value_cents: i64) -> String {
    let total_cents = (share_count as i64) * share_value_cents;
    let euros = total_cents / 100;
    let cents_rem = (total_cents.abs() % 100) as u32;
    format!("{},{:02} €", euros, cents_rem)
}

/// Parst einen User-Input (60,00 / 60.00 / 60) als Euro-Wert und liefert
/// den Cent-Wert für die API. Akzeptiert sowohl Komma als auch Punkt als
/// Dezimaltrennzeichen. Liefert None bei:
/// - leerem oder nicht-numerischem Input
/// - Werten mit Suffix (z.B. "60,00 EUR")
/// - Wert kleiner-gleich 0 (Backend Phase 7 D-12 erfordert share_value > 0)
///
/// Verwendung: Plan 12-04 Create-Modal, Plan 12-06 share_value-Inline-Edit.
pub fn parse_euro_to_cents(input: &str) -> Option<i64> {
    let trimmed = input.trim().replace(',', ".");
    let euros: f64 = trimmed.parse().ok()?;
    if !(euros > 0.0) {
        return None;
    }
    let cents = (euros * 100.0).round() as i64;
    if cents <= 0 {
        None
    } else {
        Some(cents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── format_payout_eur ────────────────────────────────────────────

    #[test]
    fn format_payout_zero() {
        assert_eq!(format_payout_eur(0, 0), "0,00 €");
        assert_eq!(format_payout_eur(0, 1234567), "0,00 €");
        assert_eq!(format_payout_eur(5, 0), "0,00 €");
    }

    #[test]
    fn format_payout_basic() {
        assert_eq!(format_payout_eur(1, 100), "1,00 €");
        assert_eq!(format_payout_eur(60, 100), "60,00 €");
        assert_eq!(format_payout_eur(1, 6000), "60,00 €");
        assert_eq!(format_payout_eur(3, 1500), "45,00 €");
    }

    #[test]
    fn format_payout_cent_padding() {
        assert_eq!(format_payout_eur(2, 99), "1,98 €");
        assert_eq!(format_payout_eur(1, 5), "0,05 €");
        assert_eq!(format_payout_eur(1, 105), "1,05 €");
        assert_eq!(format_payout_eur(1, 50), "0,50 €");
    }

    #[test]
    fn format_payout_large() {
        // 1.000 Anteile × 100 EUR = 100.000 EUR
        assert_eq!(format_payout_eur(1_000, 10_000), "100000,00 €");
    }

    // ── parse_euro_to_cents ──────────────────────────────────────────

    #[test]
    fn parse_euro_basic() {
        assert_eq!(parse_euro_to_cents("60,00"), Some(6000));
        assert_eq!(parse_euro_to_cents("60.00"), Some(6000));
        assert_eq!(parse_euro_to_cents("60"), Some(6000));
    }

    #[test]
    fn parse_euro_fractional() {
        assert_eq!(parse_euro_to_cents("1,5"), Some(150));
        assert_eq!(parse_euro_to_cents("0,01"), Some(1));
        assert_eq!(parse_euro_to_cents("0,99"), Some(99));
    }

    #[test]
    fn parse_euro_trim_whitespace() {
        assert_eq!(parse_euro_to_cents(" 60,00 "), Some(6000));
        assert_eq!(parse_euro_to_cents("\t60\n"), Some(6000));
    }

    #[test]
    fn parse_euro_rejects_zero_and_negative() {
        assert_eq!(parse_euro_to_cents("0"), None);
        assert_eq!(parse_euro_to_cents("0,00"), None);
        assert_eq!(parse_euro_to_cents("-5,00"), None);
    }

    #[test]
    fn parse_euro_rejects_garbage() {
        assert_eq!(parse_euro_to_cents(""), None);
        assert_eq!(parse_euro_to_cents("abc"), None);
        assert_eq!(parse_euro_to_cents("60,00 EUR"), None);
    }
}
