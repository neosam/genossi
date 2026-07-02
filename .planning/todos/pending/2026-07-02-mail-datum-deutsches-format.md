---
created: 2026-07-02T00:00:00.000Z
title: Datums-Template-Variablen in Mails im deutschen Format (DD.MM.YYYY)
area: general
resolves_phase: 23
files:
  - genossi_mail/src/template.rs:17-18
---

## Problem

Beim Versenden einer Mail mit der Template-Variable `exit_date` (und ebenso
`join_date`) ist das Datum unschön formatiert. Der Wert kommt im Default-/
ISO-Format heraus (z. B. `2026-07-02`), erwartet wird aber das deutsche
Format `DD.MM.YYYY`, z. B. **02.07.2026**.

Ursache: In `genossi_mail/src/template.rs:17-18` werden die Datumsfelder per
`.to_string()` in die minijinja-Template-Variablen übernommen:

```rust
let join_date_str = entity.join_date.to_string();
let exit_date_str = entity.exit_date.map(|d| d.to_string());
```

`.to_string()` liefert die technische Default-Repräsentation, kein
lokalisiertes Datum.

## Solution

TBD — grobe Richtung:

- Datumsfelder für Template-Variablen mit einer expliziten
  `time::format_description`-Vorlage `"[day].[month].[year]"` formatieren
  (statt `.to_string()`), sodass `exit_date`/`join_date` als `02.07.2026`
  erscheinen. Betrifft `join_date` und `exit_date` in `template.rs` (ggf.
  weitere Datumsvariablen prüfen).
- Kleiner Helfer `fn format_de(date) -> String` in `genossi_mail`, damit
  alle Datums-Template-Variablen konsistent formatiert sind.
- Unit-Test analog zu `test_exit_date_null` (template.rs:481) ergänzen, der
  prüft, dass ein gesetztes `exit_date` als `DD.MM.YYYY` gerendert wird.
- Passt thematisch in v1.4 (Mail-Formatierung) — beim Planen von Phase 22/23
  mitnehmen oder als eigenständiger Quick.
