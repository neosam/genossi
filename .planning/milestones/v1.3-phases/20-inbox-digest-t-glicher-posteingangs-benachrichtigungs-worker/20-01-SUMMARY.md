---
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
plan: 01
subsystem: database
tags: [sqlite, sqlx, dao, migration, digest, upsert, mockall]

# Dependency graph
requires:
  - phase: 19-e-mail-anh-nge-anzeigen
    provides: genossi_mail DAO/SQLite-Layer-Konventionen (InboundMail, MailDaoError, automock)
provides:
  - "digest_state Singleton-KV-Tabelle (Migration) für das letzte Digest-Versanddatum (D-03)"
  - "DigestStateDao Trait (mit #[automock] → MockDigestStateDao) im genossi_mail-Crate"
  - "DigestStateDaoSqlite mit get_last_sent_date / set_last_sent_date (Upsert-Singleton)"
affects: [20-02-digest-worker, digest, inbox]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dedizierte State-Tabelle (digest_state) getrennt vom Config-KV-Store (D-03)"
    - "KV-Upsert-Singleton via INSERT ... ON CONFLICT(key) DO UPDATE (spiegelt ConfigDaoSqlite)"
    - "time::Date <-> TEXT 'YYYY-MM-DD' Konvertierung via runtime time::format_description::parse"

key-files:
  created:
    - migrations/sqlite/20260626000000_create_digest_state_table.sql
  modified:
    - genossi_mail/src/dao.rs
    - genossi_mail/src/dao_sqlite.rs

key-decisions:
  - "Variante A (KV-artige Tabelle digest_state(key, value)) gewählt statt Singleton-Row mit CHECK(id=1) — spiegelt exakt das ConfigDaoSqlite-Upsert-Pattern, einfachste mockbare get/set-Semantik"
  - "time::Date statt String/PrimitiveDateTime als Datums-Typ im Trait — typsicher, Worker (Plan 02) braucht Datums-Vergleich für is_due"
  - "Eigenes Test-Submodul digest_state_tests statt der bestehenden tests-mod, um die setup_db-Namenskollision (mail_jobs-Tabelle vs digest_state) zu vermeiden"

patterns-established:
  - "State-Foundation-Pattern: eigene Tabelle + automock-Trait im genossi_mail-Crate + SQLite-Impl + In-Memory-Tests, unabhängig vom Worker testbar"

requirements-completed: [DIGEST-03]

# Metrics
duration: 6min
completed: 2026-06-26
---

# Phase 20 Plan 01: Digest-State DB-Foundation Summary

**Dedizierte SQLite-Tabelle `digest_state` + `DigestStateDao`/`DigestStateDaoSqlite` mit Upsert-Singleton-Semantik für das letzte Digest-Versanddatum (D-03) — inklusive 3 In-Memory-Unit-Tests.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-26T21:26:39Z
- **Completed:** 2026-06-26T21:33:10Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Migration `20260626000000_create_digest_state_table.sql`: dedizierte Singleton-KV-Tabelle (NICHT der Config-KV-Store), Timestamp > letzter Migration (20260625000000)
- `DigestStateDao` Trait mit `#[automock]` → erzeugt `MockDigestStateDao` für die Plan-02-Worker-Tests
- `DigestStateDaoSqlite` mit `get_last_sent_date` (None bei leerer Tabelle) und `set_last_sent_date` (Upsert via `ON CONFLICT(key) DO UPDATE`)
- 3 grüne Unit-Tests: empty→None, set→get, zweiter set überschreibt Singleton (COUNT == 1)

## Task Commits

Each task was committed atomically (Task 2 als TDD: RED → GREEN):

1. **Task 1: Migration für digest_state Singleton-Tabelle** - `b3f95c3` (feat)
2. **Task 2 (RED): failing tests for DigestStateDao** - `854fdcf` (test)
3. **Task 2 (GREEN): implement DigestStateDaoSqlite** - `64a5d5e` (feat)

_TDD-Gate-Sequenz: test(...) `854fdcf` → feat(...) `64a5d5e`. Kein Refactor nötig (Code folgte direkt den bestehenden Patterns)._

## Files Created/Modified
- `migrations/sqlite/20260626000000_create_digest_state_table.sql` - Singleton-KV-Tabelle `digest_state(key, value)`
- `genossi_mail/src/dao.rs` - `DigestStateDao` Trait (`#[automock] #[async_trait]`) mit get/set last_sent_date
- `genossi_mail/src/dao_sqlite.rs` - `DigestStateDaoSqlite` Impl (Upsert) + `digest_state_tests` Modul (3 Tests)

## Decisions Made
- **Migration-Variante A (KV-artig)** statt Singleton-Row mit `CHECK(id=1)`: spiegelt exakt das ConfigDaoSqlite-Upsert-Pattern und ist am einfachsten mockbar.
- **`time::Date`** als Datums-Typ (statt String/PrimitiveDateTime): typsicher, der Worker braucht in Plan 02 reinen Datums-Vergleich.
- **Eigenes Test-Submodul `digest_state_tests`**: das bestehende `tests`-Modul in dao_sqlite.rs hat bereits ein `setup_db()` (das u.a. `mail_jobs` anlegt). Ein separates Submodul vermeidet die Namenskollision und hält die digest_state-Tabellendefinition isoliert.
- **`time::format_description::parse` (runtime)** statt `time::macros::format_description!` (compile-time): folgt dem bereits in dao_sqlite.rs (Zeilen 21-29) etablierten Pattern und vermeidet eine mögliche Macro-Verfügbarkeitsfrage.

## Deviations from Plan

None - plan executed exactly as written. (Das Plan-Dokument bot `time::macros::format_description!` als Primärweg an, erlaubte aber explizit den `parse`-Fallback "falls nicht verfügbar"; der gewählte runtime-`parse`-Weg liegt innerhalb der Plan-Vorgabe und ist konsistent mit dem bestehenden File-Pattern.)

## Issues Encountered
None.

## User Setup Required
None - keine externe Service-Konfiguration nötig. Die Migration läuft automatisch beim Serverstart (sqlx migrate).

## Next Phase Readiness
- DB-Foundation steht: Plan 02 (Digest-Worker) kann `DigestStateDaoSqlite` via `DigestStateDaoSqlite::new(self.pool.clone())` wiren und `MockDigestStateDao` für Worker-Unit-Tests nutzen.
- Keine Blocker. Worker-Wiring in `genossi_bin/src/lib.rs` (`start_digest_worker`) und main.rs-Spawn sind Plan-02-Scope.

## Self-Check: PASSED

- Files verified present: migration, dao.rs, dao_sqlite.rs, SUMMARY.md
- Commits verified in git log: b3f95c3, 854fdcf, 64a5d5e

---
*Phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker*
*Completed: 2026-06-26*
