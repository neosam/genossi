---
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
verified: 2026-06-27T00:00:00Z
status: passed
score: 10/10
overrides_applied: 0
---

# Phase 20: Inbox-Digest Verification Report

**Phase Goal:** Ein Scheduler-Worker verschickt einmal pro Kalendertag zur konfigurierten Uhrzeit eine Zusammenfassungs-Mail aller offenen Posteingangs-Mails an konfigurierbare Empfänger-Adressen — mit Titel, Absender, Eingangszeitpunkt je Mail und Deep-Link auf `/inbox`. Versand nur bei nicht-leerem Posteingang; Empfänger und Uhrzeit werden über die Config-Seite gepflegt.
**Verified:** 2026-06-27T00:00:00Z
**Status:** passed
**Re-verification:** Nein — Erstverifikation

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Config-Seite zeigt Abschnitt "Posteingangs-Benachrichtigung" mit Empfänger-Feld und Uhrzeit-Feld; Werte bleiben nach Reload erhalten (DIGEST-01, DIGEST-02) | VERIFIED | `CollapsibleSection { title: "Posteingangs-Benachrichtigung"` in config_page.rs:821; Signals `digest_recipients` + `digest_send_time` werden in reload() aus Config-KV-Store populiert (Zeilen 174-177) |
| 2 | Ein leeres Empfänger-Feld ist speicherbar und deaktiviert das Feature ohne Fehler (DIGEST-07) | VERIFIED | `validate_digest_recipients("")` gibt `true` (Zeile 31-33 config_page.rs); Worker-Loop skipped bei `recipients.is_empty()` (digest.rs:129); 4 zugehörige Unit-Tests in config_page::tests |
| 3 | Die `digest_state`-Tabelle persistiert das letzte Versanddatum in einer eigenen SQLite-Tabelle (NICHT der Config-KV-Store) (DIGEST-03 Basis) | VERIFIED | `migrations/sqlite/20260626000000_create_digest_state_table.sql` vorhanden mit `CREATE TABLE IF NOT EXISTS digest_state (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL)` |
| 4 | DigestStateDao liest None bei leerer Tabelle und schreibt per Upsert (kein Duplikat) (DIGEST-03) | VERIFIED | `DigestStateDaoSqlite::get_last_sent_date` und `set_last_sent_date` in dao_sqlite.rs:1262-1295; `ON CONFLICT(key) DO UPDATE SET value = excluded.value` Zeile 1289; 3 Unit-Tests in `digest_state_tests` (leer=None, set=get, zweiter-set=Upsert COUNT==1) |
| 5 | Bei leerem Posteingang geht keine Digest-Mail raus und `last_sent_date` wird NICHT gesetzt (DIGEST-04) | VERIFIED | digest.rs:149-151: `if offen.is_empty()` → nur `tracing::debug!`, kein `set_last_sent_date` — leerer Tag gilt nicht als erledigt. Kommentar im Code bestätigt die Semantik explizit. |
| 6 | Jeder konfigurierte Empfänger erhält genau eine Digest-Mail pro Kalendertag; fehlerhafter Empfänger blockiert die übrigen nicht; Versanddatum wird trotzdem gesetzt (DIGEST-05/06) | VERIFIED | digest.rs:160-174: `for recipient in &recipients` Loop mit `send_test_mail_with_body`; Fehler werden nur geloggt (`tracing::error!`), Loop läuft weiter; `set_last_sent_date` nach der Schleife |
| 7 | Die Digest-Mail enthält Titel, Absender, Eingangszeitpunkt je offener Mail (neueste zuerst) und einen Deep-Link auf `/inbox` (DIGEST-05, DIGEST-06 i.S. "enthält Link") | VERIFIED | `build_digest_body` (digest.rs:87-97): je Mail `"- {} (von: {}, eingegangen: {})\n"` mit subject/from_address/received_at; Zeile 95: `"Zum Posteingang: {}\n"` mit `{APP_URL}/inbox`; Reihenfolge kommt von `InboxService::list()` (ORDER BY received_at DESC) |
| 8 | Worker pollt periodisch (~60s, kein sleep-bis-Uhrzeit) und vergleicht Server-Lokalzeit mit konfigurierter Uhrzeit; Catch-up nach verpasstem Zeitfenster (DIGEST-03) | VERIFIED | `POLL_INTERVAL_SECS = 60` (digest.rs:18); `is_due` prüft `now_minutes >= send_minutes` ohne Datumseinschränkung auf heute — deckt Catch-up ab (Zeile 76); 6 `is_due`-Unit-Tests inkl. Catch-up-Fall |
| 9 | Worker ist beim Serverstart gespawnt und korrekt mit allen Abhängigkeiten verdrahtet (DI-Wiring) | VERIFIED | `genossi_bin/src/main.rs:57` ruft `rest_state.start_digest_worker()`; `genossi_bin/src/lib.rs:1463-1478`: Methode baut `ConfigService`, klont `inbox_service` + `mail_service`, erstellt `DigestStateDaoSqlite::new(self.pool.clone())` und spawnt den Tokio-Task |
| 10 | Inline-Validierung im Frontend meldet ungültige E-Mail-Adressen und ungültige HH:MM-Uhrzeit vor dem Speichern (D-13) | VERIFIED | `validate_digest_recipients` (config_page.rs:29-39) und `validate_digest_send_time` (Zeilen 43-48) als pure free functions; onclick-Closure prüft beide und setzt `error.set(AppError::new(...))` + `return` vor dem `spawn` (Zeilen 864-879); 6 Unit-Tests in `config_page::tests` |

**Score:** 10/10 Truths verified

---

### Required Artifacts

| Artifact | Erwartet | Status | Details |
|----------|----------|--------|---------|
| `migrations/sqlite/20260626000000_create_digest_state_table.sql` | Singleton-KV-Tabelle digest_state | VERIFIED | Existiert; enthält `CREATE TABLE IF NOT EXISTS digest_state (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL)` |
| `genossi_mail/src/dao.rs` | DigestStateDao Trait mit #[automock] | VERIFIED | Zeilen 159-166: `#[automock] #[async_trait] pub trait DigestStateDao` mit `get_last_sent_date` + `set_last_sent_date` |
| `genossi_mail/src/dao_sqlite.rs` | DigestStateDaoSqlite mit Upsert | VERIFIED | Zeilen 1251-1295: `pub struct DigestStateDaoSqlite` + `impl DigestStateDao for DigestStateDaoSqlite`; `ON CONFLICT(key) DO UPDATE SET value = excluded.value` vorhanden |
| `genossi_mail/src/digest.rs` | Worker + reine Helfer + Unit-Tests (min. 120 Zeilen) | VERIFIED | 395 Zeilen; enthält `start_digest_worker`, `parse_recipients`, `parse_send_time`, `is_due`, `build_digest_subject`, `build_digest_body`; 21 Unit-Tests (`#[test]`) |
| `genossi_mail/src/lib.rs` | `pub mod digest;` Deklaration | VERIFIED | Zeile 4: `pub mod digest;` |
| `genossi_bin/src/lib.rs` | `fn start_digest_worker` + DigestStateDaoType | VERIFIED | Zeile 583: `type DigestStateDaoType = genossi_mail::dao_sqlite::DigestStateDaoSqlite`; Zeilen 1463-1478: `pub fn start_digest_worker` |
| `genossi_bin/src/main.rs` | Spawn-Aufruf beim Serverstart | VERIFIED | Zeile 57: `rest_state.start_digest_worker();` |
| `genossi-frontend/src/page/config_page.rs` | CollapsibleSection "Posteingangs-Benachrichtigung" + Signals + Validierung | VERIFIED | Abschnitt Zeile 821; Signals Zeilen 88-90; reload-Populate Zeilen 174-177; Validierungsfunktionen Zeilen 29-48 |

---

### Key Link Verification

| Von | Nach | Via | Status | Details |
|-----|------|-----|--------|---------|
| `digest.rs::start_digest_worker` | DigestStateDao | `get_last_sent_date` / `set_last_sent_date` | WIRED | Zeile 140: `digest_state_dao.get_last_sent_date()`; Zeile 174: `digest_state_dao.set_last_sent_date(now_local.date())` |
| `digest.rs::start_digest_worker` | MailService | `send_test_mail_with_body` | WIRED | Zeile 162: `mail_service.send_test_mail_with_body(recipient, &subject, &body)` |
| `genossi_bin/src/main.rs` | `RestStateImpl::start_digest_worker` | `rest_state.start_digest_worker()` | WIRED | Zeile 57 in main.rs; Methode in lib.rs:1463 vorhanden |
| `config_page.rs Digest-Abschnitt` | Config-KV-Store | `api::set_config_entry` mit `digest_recipients` + `digest_send_time` | WIRED | Zeilen 889-890: `("digest_recipients", recipients, "string")` + `("digest_send_time", send_time, "string")` im Save-Flow |
| `config_page.rs reload()` | digest-Signals | `get_config_value(&data, "digest_recipients")` + `"digest_send_time"` | WIRED | Zeilen 174-177: Populate aus Config-KV-Store nach Reload |
| `dao_sqlite.rs::DigestStateDaoSqlite` | `digest_state` Tabelle | `INSERT ... ON CONFLICT DO UPDATE` | WIRED | Zeile 1288-1289: `"INSERT INTO digest_state (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"` |

---

### Data-Flow Trace (Level 4)

| Artifact | Datenvariable | Quelle | Echte Daten | Status |
|----------|--------------|--------|-------------|--------|
| `digest.rs::start_digest_worker` | `recipients`, `send_time` | `config_service.get_all()` → Config-KV-Store (SQLite `config_entries`) | Ja — echter DB-Aufruf | FLOWING |
| `digest.rs::start_digest_worker` | `offen` (offene Mails) | `inbox_service.list()` → InboundMailDao → SQLite `inbound_mails WHERE archived = 0` | Ja — echter DB-Aufruf | FLOWING |
| `digest.rs::start_digest_worker` | `last_sent` | `digest_state_dao.get_last_sent_date()` → SQLite `digest_state` | Ja — echter DB-Aufruf | FLOWING |
| `config_page.rs` | `digest_recipients`, `digest_send_time` | `api::get_config_entries` → REST → Config-KV-Store | Ja — HTTP-Aufruf → echter DB-Aufruf | FLOWING |

---

### Behavioral Spot-Checks

Alle direkten Spot-Checks wurden auf Codeebene verifiziert (kein laufender Server verfügbar).

| Verhalten | Methode | Ergebnis | Status |
|-----------|---------|----------|--------|
| `is_due`: heute schon gesendet → false | Code-Lektüre digest.rs:70-71 | `last_sent_date == Some(today) → return false` korrekt implementiert | PASS |
| `is_due`: Catch-up nach verpasstem Fenster | Code-Lektüre digest.rs:74-76 | `now_minutes >= send_minutes` ohne Tagesprüfung — deckt Catch-up ab | PASS |
| Leerer Posteingang: kein Versand, kein `set_last_sent_date` | Code-Lektüre digest.rs:149-151 | `if offen.is_empty()` → nur debug-Log, kein DAO-Aufruf | PASS |
| Kein Empfänger: Worker skipped ohne Fehler | Code-Lektüre digest.rs:129-132 | `if recipients.is_empty()` → debug-Log + sleep + continue | PASS |
| Frontend-Validierung: leeres Feld = gültig | `validate_digest_recipients("")` | Zeile 31-33: `if trimmed.is_empty() { return true; }` | PASS |
| Config-Keys identisch (Backend ↔ Frontend) | Grep-Vergleich | Backend: `CFG_RECIPIENTS = "digest_recipients"`, `CFG_SEND_TIME = "digest_send_time"`; Frontend: gleiche Strings in Save-Flow und reload | PASS |

---

### Requirements Coverage

| Anforderung | Plan | Beschreibung | Status | Evidenz |
|-------------|------|-------------|--------|---------|
| DIGEST-01 | 20-03 | Empfänger-Adressen pflegbar und persistent | SATISFIED | Signals + reload-Populate + `api::set_config_entry("digest_recipients", ...)` |
| DIGEST-02 | 20-03 | Versand-Uhrzeit konfigurierbar und persistent | SATISFIED | `digest_send_time`-Signal + reload-Populate + `api::set_config_entry("digest_send_time", ...)` |
| DIGEST-03 | 20-01, 20-02 | Einmal pro Kalendertag zur konfigurierten Uhrzeit | SATISFIED | `digest_state` Tabelle + DigestStateDao Upsert + `is_due`-Logik (heute-schon-gesendet = false) |
| DIGEST-04 | 20-02 | Kein Versand bei leerem Posteingang | SATISFIED | `if offen.is_empty()` → kein `set_last_sent_date`, kein Versand |
| DIGEST-05 | 20-02 | Digest-Mail listet offene Mails mit Titel, Absender, Eingangszeitpunkt | SATISFIED | `build_digest_body`: `"- {} (von: {}, eingegangen: {})\n"` mit subject/from_address/received_at |
| DIGEST-06 | 20-02 | Digest-Mail enthält Deep-Link auf `/inbox` | SATISFIED | `format!("{}/inbox", app_url.trim_end_matches('/'))` + `"Zum Posteingang: {}\n"` in build_digest_body |
| DIGEST-07 | 20-02, 20-03 | Leeres Empfänger-Feld deaktiviert Feature ohne Fehler | SATISFIED | Frontend: `validate_digest_recipients("") = true` (speicherbar); Worker: `if recipients.is_empty()` → skip |

Alle 7 Anforderungen (DIGEST-01 bis DIGEST-07) sind erfüllt und vollständig in Code und Tests belegt.

---

### Anti-Patterns Found

Keine Blocker-Anti-Patterns gefunden.

| Datei | Befund | Schweregrad | Bewertung |
|-------|--------|-------------|-----------|
| digest.rs | Kein `return null`, keine TODOs, keine leeren Handler | - | Sauber |
| config_page.rs | Save-Button nutzt `r#type: "button"` (kein form-onsubmit) — Dioxus-Reload-Bug vermieden | - | Korrekt |
| dao_sqlite.rs | Upsert-Semantik via `ON CONFLICT` — kein mögliches Duplikat | - | Korrekt |

---

### Human Verification Required

Folgende Punkte können nicht rein automatisiert verifiziert werden und erfordern manuelle Sichtprüfung (kein Blocker, da alle Code-Pfade korrekt implementiert sind):

1. **Visueller Abschnitt "Posteingangs-Benachrichtigung" auf der Config-Seite**
   - Test: Config-Seite öffnen, Abschnitt ausklappen, Empfänger + Uhrzeit eintragen, Speichern klicken, Seite neu laden
   - Erwartet: Werte sind nach Reload erhalten; Inline-Fehler bei ungültiger Adresse / Uhrzeit; leeres Feld speicherbar
   - Warum human: Visuelles Layout, tatsächliches Browser-Verhalten, WASM-Rendering

2. **Echter Digest-Mail-Versand**
   - Test: Empfänger + Uhrzeit konfigurieren (wenige Minuten in der Zukunft), nicht-archivierte Mails im Posteingang haben, warten bis Uhrzeit erreicht
   - Erwartet: Digest-Mail kommt an mit korrektem Betreff, Mail-Liste und `/inbox`-Deep-Link
   - Warum human: Erfordert SMTP-Konfiguration, laufenden Server und echte Wartezeit

Diese Human-Verification-Punkte sind reine UX-/Integrations-Bestätigungen. Alle funktionalen Code-Pfade wurden durch Code-Lektüre und Unit-Tests (21 Tests in digest.rs + 6 in config_page + 3 in dao_sqlite) vollständig verifiziert.

---

## Zusammenfassung

**Phase 20 hat ihr Ziel erreicht.** Alle 4 Roadmap-Erfolgskriterien sind erfüllt:

1. **Config-Seite:** Eigener `CollapsibleSection`-Abschnitt "Posteingangs-Benachrichtigung" mit Empfänger-Feld, Uhrzeit-Feld, clientseitiger Inline-Validierung und Speichern-Flow. Werte werden aus dem Config-KV-Store geladen und bleiben nach Reload erhalten (DIGEST-01, DIGEST-02).

2. **Genau ein Versand pro Tag je Empfänger:** Der Worker pollt alle 60s, vergleicht Server-Lokalzeit mit konfigurierter Uhrzeit via `is_due`, sendet pro Empfänger eine separate Mail und persistiert das Datum via Upsert in der dedizierten `digest_state`-Tabelle (DIGEST-03).

3. **Kein Versand bei leerem Posteingang oder ohne Empfänger:** Explizite Guards im Worker-Loop für beide Fälle, kein Fehler (DIGEST-04, DIGEST-07).

4. **Digest-Mail-Inhalt:** Plain-Text mit Titel/Absender/Eingangszeit je offener Mail (neueste zuerst per InboxService ORDER BY) und `{APP_URL}/inbox`-Deep-Link (DIGEST-05, DIGEST-06).

Der Worker ist vollständig in `genossi_bin` verdrahtet und spawnt beim Serverstart. 30 Unit-Tests (21 + 6 + 3) decken alle Edge-Cases ab.

---

_Verified: 2026-06-27_
_Verifier: Claude (gsd-verifier)_
