## Context

Die Mail-Seite (`mail_page.rs`) lädt alle Mitglieder über den globalen `MEMBERS`-State und bietet einen "Alle"-Button, der sämtliche Mitglieder-IDs auswählt. Die Mitglieder-Seite (`members.rs`) hat bereits eine `is_active()`-Funktion (Zeile 10-18), die prüft, ob ein Mitglied basierend auf `join_date` und `exit_date` aktiv ist, sowie eine `today()`-Funktion (Zeile 20-29) für das aktuelle Datum via `js_sys::Date`.

## Goals / Non-Goals

**Goals:**
- "Alle"-Button auf der Mail-Seite wählt nur aktive Mitglieder (nicht ausgetreten, nicht gelöscht) aus
- Zählung im "Alle"-Button zeigt nur aktive Mitglieder mit E-Mail an
- `is_active()` und `today()` als gemeinsam nutzbare Funktionen verfügbar machen

**Non-Goals:**
- Kein separater API-Endpunkt für aktive Mitglieder (Filterung bleibt im Frontend)
- Keine Änderung am Such-Dropdown (dort können weiterhin alle Mitglieder einzeln ausgewählt werden)

## Decisions

### 1. `is_active()` und `today()` in ein gemeinsames Modul verschieben

Die Funktionen `is_active()` und `today()` werden aus `members.rs` in ein neues Modul (z.B. `src/util.rs` oder `src/member_utils.rs`) extrahiert, damit sowohl `members.rs` als auch `mail_page.rs` sie verwenden können.

**Warum**: Code-Duplizierung vermeiden. Beide Seiten brauchen dieselbe Aktiv-Logik. Alternative wäre die Funktionen einfach in `mail_page.rs` zu duplizieren — aber das widerspricht DRY und birgt die Gefahr inkonsistenter Logik.

### 2. Filterung im Frontend, nicht im Backend

Die Filterung erfolgt im Frontend bei der "Alle"-Auswahl. Die MEMBERS-Liste enthält weiterhin alle Mitglieder.

**Warum**: Die Mitglieder werden bereits vollständig geladen. Eine Backend-Änderung (neuer Endpunkt oder Query-Parameter) wäre Overhead für eine reine UI-Logik. Außerdem muss das Dropdown weiterhin alle Mitglieder zeigen können.

### 3. Doppelter Filter: `deleted` und `exit_date`

Beim "Alle"-Button werden Mitglieder ausgeschlossen, die:
- `deleted.is_some()` haben (soft-deleted)
- Nicht aktiv sind laut `is_active()` (basierend auf `exit_date` und `join_date`)

**Warum**: Konsistenz mit der Mitglieder-Seite, die beide Filter anwendet (Zeile 78 und 91-95 in `members.rs`).

## Risks / Trade-offs

- **Einzelauswahl nicht gefiltert**: Im Such-Dropdown können weiterhin ausgetretene Mitglieder einzeln ausgewählt werden. Das ist beabsichtigt — in Ausnahmefällen möchte man einem ehemaligen Mitglied vielleicht doch eine E-Mail senden.

## Open Questions

Keine.
