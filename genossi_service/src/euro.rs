//! Euro-Formatierungs-Helfer (deutsche Locale).
//!
//! Reine Utility-Funktion für Anzeige/Template-Rendering — kein Runden über die
//! Cent-Genauigkeit hinaus, keine Locale-Bibliothek. Der einzige kanonische
//! Euro-Formatter der Domäne: Tausenderpunkt, Dezimalkomma, `€`-Suffix, korrekte
//! Null-/Negativ-Behandlung. Verwendet vom Antragsteller-Template-Kontext
//! (`open_amount`) und von `send_confirmation_mail`.

/// Formatiert einen Cent-Betrag als deutschen Euro-String.
///
/// Tausenderpunkt (`.`), Dezimalkomma (`,`), ASCII-Leerzeichen vor dem
/// `€`-Zeichen (U+20AC) — passend zur bestehenden Bestätigungs-Mail-Wortwahl
/// („… von X €"). Das Vorzeichen wird auf dem Betrag gebildet: Cents sind nie
/// negativ, ein `-` wird nur dem Gesamt-String vorangestellt.
///
/// Der Betrag wird über `i128` normiert, damit auch `i64::MIN` nicht überläuft
/// (kein Panic bei pathologischer Eingabe).
///
/// Beispiele:
/// - `format_eur_de(0)` → `"0,00 €"`
/// - `format_eur_de(5)` → `"0,05 €"`
/// - `format_eur_de(1234)` → `"12,34 €"`
/// - `format_eur_de(123456)` → `"1.234,56 €"`
/// - `format_eur_de(100000000)` → `"1.000.000,00 €"`
/// - `format_eur_de(-123456)` → `"-1.234,56 €"`
pub fn format_eur_de(_cents: i64) -> String {
    // RED-Stub: noch nicht implementiert.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_eur_de_zero() {
        assert_eq!(format_eur_de(0), "0,00 €");
    }

    #[test]
    fn format_eur_de_single_cent_zero_padded() {
        assert_eq!(format_eur_de(5), "0,05 €");
    }

    #[test]
    fn format_eur_de_below_thousand() {
        assert_eq!(format_eur_de(1234), "12,34 €");
    }

    #[test]
    fn format_eur_de_thousands_separator() {
        assert_eq!(format_eur_de(123456), "1.234,56 €");
    }

    #[test]
    fn format_eur_de_multiple_groups() {
        assert_eq!(format_eur_de(100000000), "1.000.000,00 €");
    }

    #[test]
    fn format_eur_de_large_multi_group() {
        // 1.234.567.890,12 €
        assert_eq!(format_eur_de(123456789012), "1.234.567.890,12 €");
    }

    #[test]
    fn format_eur_de_negative_thousands() {
        assert_eq!(format_eur_de(-123456), "-1.234,56 €");
    }

    #[test]
    fn format_eur_de_negative_single_cent() {
        assert_eq!(format_eur_de(-5), "-0,05 €");
    }

    #[test]
    fn format_eur_de_i64_min_does_not_panic() {
        // Pathologische Eingabe (D-13 / T-30-01-02): kein Overflow-Panic.
        let result = format_eur_de(i64::MIN);
        assert!(result.starts_with('-'));
        assert!(result.ends_with(" €"));
    }
}
