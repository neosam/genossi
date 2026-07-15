---
task_type: quick
slug: exit-date-boundary-fix
created: 2026-07-15
completed: 2026-07-15
status: complete
commit: 2dc736be
---

# Summary: exit-date-boundary-fix

## Was gefixt wurde

**Bug:** `genossi_dao_impl_sqlite/src/backup.rs:100` filterte Mitglieder mit
`exit_date > ?`. In Kombination mit dem 31.12.-Stichtag (`genossi_backup/src/worker.rs:132`) fielen Mitglieder mit Austritt zum Jahresende aus `mitgliederliste-YYYY.csv` — obwohl sie das ganze Jahr Mitglied waren.

**Fix:** Vergleich auf `>=`. Neue Semantik: „am Austrittstag noch Mitglied". Konsistent für Jahres- und aktuelle Liste.

## Changed Files

| Datei | Änderung |
|---|---|
| `genossi_dao_impl_sqlite/src/backup.rs` | 1-Zeichen-Fix in `members_at_date` + 3 Regressionstests (in-memory SQLite) |

## Verification

| Check | Result |
|---|---|
| `cargo test -p genossi_dao_impl_sqlite` | 74 passed, 0 failed |
| `cargo test -p genossi_backup` | 40 passed, 0 failed |
| `cargo clippy -p genossi_dao_impl_sqlite --lib --tests` | clean |
| Neue Tests | 3/3 grün |

## Commit

`2dc736be — fix(backup): jahres-export inkludiert mitglieder mit austritt am stichtag`

## Follow-ups (nicht in diesem Quick-Task)

Aus der ursprünglichen Prüfung sind zwei weitere Inkonsistenzen offen:

- `all_actions()` und `all_communications()` filtern **nicht** auf `m.deleted IS NULL` und **nicht** auf `status != 'FehlerhaftErfasst'` → Aktionen/Mails zu gelöschten oder als fehlerhaft markierten Mitgliedern erscheinen in den Export-CSVs, obwohl die Mitglieder-Liste sie ausblendet.
- `all_documents()` filtert `m.deleted`, aber ebenfalls nicht auf `FehlerhaftErfasst`.

Empfehlung: Separater Task, sobald die datenschutzliche Soll-Semantik geklärt ist.
