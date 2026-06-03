//! IBAN-Formatierungs-Helfer.
//!
//! Reine Utility-Funktionen für Anzeige/Template-Rendering — keine Validierung
//! der IBAN (kein Checksum-Check, kein Längen-Check pro Ländercode). Verwendet
//! von `genossi_mail` (E-Mail-Templates) und `genossi_service_impl`
//! (Typst-PDF-Inputs).

const MASK_CHAR: char = '\u{2022}';
const VISIBLE_PREFIX: usize = 2;
const VISIBLE_SUFFIX: usize = 4;

/// Entfernt sämtliche Whitespace-Zeichen aus `input`.
fn strip_whitespace(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Gruppiert eine IBAN in 4er-Blöcke, getrennt durch ein einzelnes Leerzeichen.
///
/// Vorhandene Whitespaces im Input werden zunächst entfernt — die Funktion ist
/// damit idempotent (`group_iban(group_iban(x)) == group_iban(x)`).
///
/// Beispiele:
/// - `"DE89370400440532013000"` → `"DE89 3704 0044 0532 0130 00"`
/// - `"DE89 3704 0044 0532 0130 00"` → `"DE89 3704 0044 0532 0130 00"` (idempotent)
/// - `""` → `""`
pub fn group_iban(input: &str) -> String {
    let stripped = strip_whitespace(input);
    if stripped.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(stripped.len() + stripped.len() / 4);
    for (i, c) in stripped.chars().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Maskiert eine IBAN für die Anzeige.
///
/// Erste 2 + letzte 4 Zeichen bleiben sichtbar, die Mitte wird durch den
/// Bullet-Character (`•`, U+2022) ersetzt. Anschließend wird das Ergebnis in
/// 4er-Gruppen formatiert (siehe `group_iban`).
///
/// Edge Cases:
/// - Leerer Input → leerer String.
/// - IBAN mit Länge < `VISIBLE_PREFIX + VISIBLE_SUFFIX` (≤ 6 Zeichen ohne
///   Whitespace) → komplett maskiert (defensive Variante, gibt keine Klartext-
///   Bestandteile preis, wenn nicht beide Anker greifen können).
/// - Whitespaces im Input werden vor der Maskierung entfernt; das Ergebnis ist
///   damit unabhängig von der Eingabe-Formatierung.
///
/// Beispiel:
/// - `"DE89370400440532013000"` → `"DE•• •••• •••• •••• ••30 00"`
pub fn mask_iban(input: &str) -> String {
    let stripped = strip_whitespace(input);
    let len = stripped.chars().count();

    if len == 0 {
        return String::new();
    }

    // Bei sehr kurzen Eingaben (≤ 6 Zeichen) gibt es nicht genug Material, um
    // sowohl Präfix als auch Suffix vollständig sichtbar zu zeigen, ohne dass
    // gar nichts maskiert wird. In diesem Fall maskieren wir alles.
    if len <= VISIBLE_PREFIX + VISIBLE_SUFFIX {
        let masked: String = std::iter::repeat_n(MASK_CHAR, len).collect();
        return group_iban(&masked);
    }

    let mut masked = String::with_capacity(len);
    for (i, c) in stripped.chars().enumerate() {
        if i < VISIBLE_PREFIX || i >= len - VISIBLE_SUFFIX {
            masked.push(c);
        } else {
            masked.push(MASK_CHAR);
        }
    }

    group_iban(&masked)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- group_iban -----------------------------------------------------

    #[test]
    fn group_iban_empty_input() {
        assert_eq!(group_iban(""), "");
    }

    #[test]
    fn group_iban_whitespace_only() {
        assert_eq!(group_iban("   "), "");
    }

    #[test]
    fn group_iban_no_spaces() {
        assert_eq!(
            group_iban("DE89370400440532013000"),
            "DE89 3704 0044 0532 0130 00"
        );
    }

    #[test]
    fn group_iban_with_spaces() {
        assert_eq!(
            group_iban("DE89 3704 0044 0532 0130 00"),
            "DE89 3704 0044 0532 0130 00"
        );
    }

    #[test]
    fn group_iban_mixed_whitespace() {
        // Tabs und mehrfache Leerzeichen werden auch korrekt entfernt.
        assert_eq!(
            group_iban("DE89\t3704  0044\n0532013000"),
            "DE89 3704 0044 0532 0130 00"
        );
    }

    #[test]
    fn group_iban_is_idempotent() {
        let input = "AT611904300234573201";
        let once = group_iban(input);
        let twice = group_iban(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn group_iban_short_input() {
        assert_eq!(group_iban("DE12"), "DE12");
        assert_eq!(group_iban("DE123"), "DE12 3");
    }

    // ---- mask_iban ------------------------------------------------------

    #[test]
    fn mask_iban_empty_input() {
        assert_eq!(mask_iban(""), "");
    }

    #[test]
    fn mask_iban_whitespace_only() {
        assert_eq!(mask_iban("    "), "");
    }

    #[test]
    fn mask_iban_german_iban_22_chars() {
        // 22 Zeichen total = 2 prefix (DE) + 16 maskiert + 4 suffix (3000).
        // Joined: "DE" + 16x"•" + "3000".
        // Gruppiert in 4er-Blöcken (22 mod 4 = 2 → letzte Gruppe 2 Zeichen):
        //   "DE••" "••••" "••••" "••••" "••30" "00"
        let result = mask_iban("DE89370400440532013000");
        assert_eq!(
            result,
            "DE\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} \
             \u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} \
             \u{2022}\u{2022}30 00"
        );
    }

    #[test]
    fn mask_iban_preserves_country_code() {
        let result = mask_iban("AT611904300234573201");
        assert!(
            result.starts_with("AT"),
            "expected AT prefix, got: {result}"
        );
    }

    #[test]
    fn mask_iban_preserves_last_four() {
        let result = mask_iban("AT611904300234573201");
        // Letzte 4 Zeichen der bereinigten IBAN sind "3201".
        assert!(
            result.ends_with("3201"),
            "expected 3201 suffix, got: {result}"
        );
    }

    #[test]
    fn mask_iban_strips_input_whitespace() {
        // Input mit Leerzeichen liefert dasselbe Ergebnis wie ohne.
        let with_spaces = mask_iban("DE89 3704 0044 0532 0130 00");
        let without_spaces = mask_iban("DE89370400440532013000");
        assert_eq!(with_spaces, without_spaces);
    }

    #[test]
    fn mask_iban_short_input_fully_masked() {
        // Länge ≤ 6: alles maskieren (defensive Variante).
        // "ABCDEF" → 6× • → gruppiert: "•••• ••"
        assert_eq!(
            mask_iban("ABCDEF"),
            "\u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}"
        );
    }

    #[test]
    fn mask_iban_two_chars_fully_masked() {
        assert_eq!(mask_iban("DE"), "\u{2022}\u{2022}");
    }

    #[test]
    fn mask_iban_seven_chars_has_one_masked() {
        // 7 Zeichen: 2 prefix + 1 maskiert + 4 suffix → "AB•DEFG"
        // Gruppiert: "AB•D EFG"
        let result = mask_iban("ABCDEFG");
        assert_eq!(result, "AB\u{2022}D EFG");
    }

    #[test]
    fn mask_iban_middle_chars_are_bullets() {
        let result = mask_iban("DE89370400440532013000");
        // Zähle Bullet-Zeichen im Ergebnis — sollten 16 sein.
        let bullet_count = result.chars().filter(|c| *c == '\u{2022}').count();
        assert_eq!(bullet_count, 22 - VISIBLE_PREFIX - VISIBLE_SUFFIX);
    }

    #[test]
    fn mask_iban_only_country_code_visible_in_first_group() {
        // Erste Gruppe (4 Zeichen) besteht aus 2 Buchstaben + 2 Bullets.
        let result = mask_iban("DE89370400440532013000");
        let first_group = result.split(' ').next().unwrap();
        assert_eq!(first_group, "DE\u{2022}\u{2022}");
    }
}
