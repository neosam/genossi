---
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
reviewed: 2026-06-27T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - migrations/sqlite/20260626000000_create_digest_state_table.sql
  - genossi_mail/src/dao.rs
  - genossi_mail/src/dao_sqlite.rs
  - genossi_mail/src/digest.rs
  - genossi_mail/src/lib.rs
  - genossi_bin/src/lib.rs
  - genossi_bin/src/main.rs
  - genossi-frontend/src/page/config_page.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-06-27
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Die Implementierung des täglichen Posteingangs-Digest-Workers ist handwerklich sauber und folgt
konsequent den Codebase-Konventionen (timestamp_worker-Muster, DigestStateDao-Trait, mockall,
parametrisierte SQL-Queries). Die Kern-Anforderungen — ein-Versand-pro-Tag-Garantie (D-03), Catch-up
(D-01), leerer-Posteingang-Skip (DIGEST-04), Worker-Loop-Stabilität und Dioxus-Button-Reload-Pattern —
sind korrekt implementiert. Sicherheitskritische Pfade (SQL-Injection, Empfänger-Parsing) sind sauber.

Drei Warnungen betreffen: (1) eine semantisch falsche Verhaltens-Entscheidung beim leeren Posteingang
die DIGEST-04 erfüllt, aber eine unerwartete Nebenkonsequenz hat; (2) einen deutschen Grammatikfehler
im Betreff bei genau 1 Mail; (3) eine inkonsistente Validierung zwischen Frontend und Worker.
Keine Blocker gefunden.

---

## Warnings

### WR-01: Leerer Posteingang — kontinuierliches Inbox-Polling nach Versandzeit

**File:** `genossi_mail/src/digest.rs:149-151`

**Issue:** Wenn der Posteingang leer ist (alle Mails archiviert oder keine Mails), setzt der Worker
bewusst kein `last_sent_date` (DIGEST-04: "leerer Tag gilt nicht als erledigt"). Das hat eine
Nebenkonsequenz: `is_due()` gibt für den Rest des Tages bei jedem Poll `true` zurück, sodass
`inbox_service.list()` und der nachgelagerte `!m.archived`-Filter nach der konfigurierten Versandzeit
jede Minute aufgerufen werden — bis entweder Mitternacht oder eine Mail eintrifft.

Das ist laut Plan ausdrücklich gewollt ("leerer Tag gilt nicht als erledigt"), aber der Code enthält
keinen Kommentar, der diese Nebenkonsequenz als bewusste Entscheidung dokumentiert. Das erschwert
künftige Wartung: Ein Entwickler könnte versehentlich einen "Tag-erledigt"-State für den leeren Fall
ergänzen und DIGEST-04 brechen.

**Fix:** Inline-Kommentar ergänzen, der die kontinuierliche Inbox-Polling-Nebenkonsequenz als
bewusstes Trade-off benennt:

```rust
if offen.is_empty() {
    // KEIN set_last_sent_date — leerer Tag gilt nicht als erledigt (DIGEST-04).
    // Nebeneffekt: is_due() bleibt bis Mitternacht true → inbox_service.list()
    // wird nach der Versandzeit jede Minute aufgerufen. Das ist absichtlich:
    // sobald eine Mail eintrifft, wird sie noch am selben Tag versendet.
    tracing::debug!("Digest worker: inbox empty, not sending (DIGEST-04)");
}
```

---

### WR-02: Grammatikfehler im Betreff bei genau 1 offener Mail

**File:** `genossi_mail/src/digest.rs:80-82`

**Issue:** `build_digest_subject(1)` ergibt `"Posteingang: 1 offene Mails"`. Das ist im Deutschen
grammatisch falsch (Singular: "1 offene Mail", Plural: "2 offene Mails"). Der Test auf Zeile 363–365
prüft nur `.contains('1')` und fängt diesen Fehler nicht ab.

```rust
pub(crate) fn build_digest_subject(count: usize) -> String {
    format!("Posteingang: {} offene Mails", count)
    // ↑ Gibt "1 offene Mails" aus — grammatisch falsch.
}
```

**Fix:** Singular/Plural differenzieren:

```rust
pub(crate) fn build_digest_subject(count: usize) -> String {
    if count == 1 {
        "Posteingang: 1 offene Mail".to_string()
    } else {
        format!("Posteingang: {} offene Mails", count)
    }
}
```

Und den Test präzisieren:

```rust
#[test]
fn build_digest_subject_single_correct_grammar() {
    assert_eq!(build_digest_subject(1), "Posteingang: 1 offene Mail");
}
```

---

### WR-03: Validierungsasymmetrie — Frontend akzeptiert Leerzeichen-Adressen, Worker filtert sie heraus

**File:** `genossi-frontend/src/page/config_page.rs:34-38` / `genossi_mail/src/digest.rs:35-37`

**Issue:** `validate_digest_recipients` im Frontend prüft jede komma-getrennte Adresse nach `.trim()`.
Eine Eingabe wie `"  "` (nur Leerzeichen zwischen Kommas, z.B. `"a@b.de,  ,c@d.de"`) wird nach dem
Frontend-Split in `" "` aufgeteilt, das nach `.trim()` leer ist und deshalb kein `split('@')` durchläuft
— die leere Adresse wird als valide akzeptiert und gespeichert. Der Worker-seitige `parse_recipients`
filtert sie korrekt mit `.filter(|s| !s.is_empty())` heraus. Das Verhalten ist letztlich korrekt
(gespeicherte Leerzeichen-Adressen werden im Worker ignoriert), aber die Frontend-Validierung
kommuniziert dem Nutzer nicht, dass Einträge wie `"a@b.de,,c@d.de"` oder `"a@b.de, ,c@d.de"`
stillschweigend bereinigt werden.

**Fix:** Im Frontend-Validator leere Tokens nach dem Trim explizit überspringen (wie der Worker):

```rust
fn validate_digest_recipients(recipients: &str) -> bool {
    let trimmed = recipients.trim();
    if trimmed.is_empty() {
        return true;
    }
    trimmed.split(',').all(|addr| {
        let a = addr.trim();
        if a.is_empty() {
            return true; // Leerzeichen-Tokens werden vom Worker herausgefiltert — kein Fehler.
        }
        let parts: Vec<&str> = a.split('@').collect();
        parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() && parts[1].contains('.')
    })
}
```

---

## Info

### IN-01: Kommentar zu `InboxService::list()` ist unvollständig

**File:** `genossi_mail/src/digest.rs:144`

**Issue:** Der Kommentar `// InboxService::list() liefert bereits ORDER BY received_at DESC (D-10)`
trifft für die Sortierung zu, dokumentiert aber nicht, dass `list()` → `list_active()` **alle** Mails
zurückgibt (einschließlich archivierter), und der Worker das anschließende `!m.archived`-Filter selbst
anwendet. Das kann zur falschen Annahme verleiten, `list()` filtere bereits archivierte Mails heraus.

**Fix:**

```rust
// InboxService::list() → list_active(): alle Mails, ORDER BY received_at DESC (D-10).
// Archivierte Mails werden unten per !m.archived herausgefiltert.
let offen: Vec<&crate::dao::InboundMail> =
    mails.iter().filter(|m| !m.archived).collect();
```

---

### IN-02: Partieller Save möglich wenn der zweite Config-Eintrag fehlschlägt

**File:** `genossi-frontend/src/page/config_page.rs:892-898`

**Issue:** Der Save-Loop bricht beim ersten Fehler ab (`break`). Schlägt das Speichern von
`digest_recipients` fehl, wird `digest_send_time` nicht gespeichert (und umgekehrt). Das führt zu
inkonsistentem Zustand in der DB. Dieses Pattern ist im gesamten Config-Frontend so verwendet (SMTP,
IMAP usw.), also keine Regression — aber das Risiko besteht. Der Nutzer sieht eine Fehlermeldung und
kann manuell nochmal speichern.

**Fix:** Hinweis-Kommentar, kein Code-Change erforderlich (Pattern ist projektweiter Standard).

---

### IN-03: `build_digest_body` nutzt Display-Default von `PrimitiveDateTime` für `received_at`

**File:** `genossi_mail/src/digest.rs:90-93`

**Issue:** `m.received_at` (ein `time::PrimitiveDateTime`) wird über `{}` formatiert. Das Default-Display
von `PrimitiveDateTime` gibt z.B. `"2026-06-26 9:15:00.0"` aus — ohne Zeitzone und mit variablen
Dezimalstellen je nach Subsekunden. Die Ausgabe ist lesbar, hat aber kein definiertes stabiles Format.
Wenn in Zukunft ein exaktes Format erwartet wird (z.B. für Tests), könnte das zu Überraschungen führen.

**Fix:** Kein sofortiger Handlungsbedarf. Optional kann ein expliziter Formatter eingesetzt werden:

```rust
let fmt = time::format_description::parse("[day].[month].[year] [hour]:[minute]").unwrap();
let ts = m.received_at.format(&fmt).unwrap_or_else(|_| m.received_at.to_string());
body.push_str(&format!("- {} (von: {}, eingegangen: {})\n", m.subject, m.from_address, ts));
```

---

_Reviewed: 2026-06-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
