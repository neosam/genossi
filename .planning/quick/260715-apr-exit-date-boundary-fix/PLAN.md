---
task_type: quick
slug: exit-date-boundary-fix
created: 2026-07-15
status: in-progress
---

# Fix: Jahres-Export inkludiert Mitglieder mit Austritt zum 31.12.

## Problem

Der automatische WebDAV-Export der Mitgliederliste (`genossi_backup`) verwendet den 31. Dezember als Stichtag für die jährlichen CSV-Dateien (`mitgliederliste-YYYY.csv`) — siehe `genossi_backup/src/worker.rs:132`.

Die SQL-Query in `genossi_dao_impl_sqlite/src/backup.rs:100` filtert Mitglieder aber mit:
```sql
AND (m.exit_date IS NULL OR m.exit_date > ?)
```

Das strikt-größer schließt Mitglieder mit `exit_date = YYYY-12-31` aus dem Export für das entsprechende Jahr aus — obwohl sie das ganze Jahr über Mitglied waren. Bei Genossenschaften ist Austritt zum Jahresende üblich.

## Fix

Vergleich auf `>=` ändern:
```sql
AND (m.exit_date IS NULL OR m.exit_date >= ?)
```

Neue Semantik: „Am Austrittstag noch Mitglied." Konsistent für Jahres-Export UND `mitgliederliste-aktuell.csv`.

## Regressionstests (`genossi_dao_impl_sqlite/src/backup.rs`, `mod tests`)

Analog zum Muster in `attendance.rs`: In-Memory-SQLite + hand-gerolltes Schema für `member` + `member_action`.

- **(a)** Mitglied mit `exit_date == stichtag` → **wird exportiert** (Boundary-Fall, war Bug)
- **(b)** Mitglied mit `exit_date == stichtag - 1 Tag` → **wird nicht exportiert**
- **(c)** Mitglied ohne `exit_date` → **wird exportiert**

## Akzeptanzkriterien

- [ ] `>` → `>=` in `backup.rs:100`
- [ ] Drei Regressionstests grün
- [ ] `cargo test -p genossi_dao_impl_sqlite` grün
- [ ] `cargo test -p genossi_backup` grün
- [ ] `cargo build` grün
- [ ] Ein atomarer jj-Commit
