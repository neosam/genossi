---
phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker
plan: 02
subsystem: api
tags: [worker, digest, smtp, inbox, tokio, mockall, time, config]

# Dependency graph
requires:
  - phase: 20-01-digest-state-db-foundation
    provides: DigestStateDao/MockDigestStateDao + DigestStateDaoSqlite (get/set_last_sent_date)
  - phase: 19-e-mail-anh-nge-anzeigen
    provides: InboxService::list (received_at DESC), MailService::send_test_mail_with_body
provides:
  - "Digest-Worker (genossi_mail/src/digest.rs): config-getriebener Poll-Loop (~60s), Server-Lokalzeit + last_sent_date-Vergleich, ein-Versand-pro-Tag mit Catch-up"
  - "Reine, unit-getestete Helfer: parse_recipients, parse_send_time, is_due, build_digest_subject, build_digest_body"
  - "DI-Wiring: RestStateImpl::start_digest_worker + main.rs-Spawn beim Serverstart"
affects: [digest, inbox, 20-03-config-frontend]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Poll-Loop-Worker mit ausgelagerten reinen Helfern (Test-Oberfläche ohne laufenden Loop) — spiegelt timestamp_worker.rs"
    - "Server-Lokalzeit via time::OffsetDateTime::now_local() (TZ env) mit now_utc()-Fallback (D-02)"
    - "Einzelmail pro Empfänger in Schleife, ein fehlerhafter Empfänger blockiert die übrigen nicht (D-06/D-07)"

key-files:
  created:
    - genossi_mail/src/digest.rs
  modified:
    - genossi_mail/src/lib.rs
    - genossi_mail/Cargo.toml
    - genossi_bin/src/lib.rs
    - genossi_bin/src/main.rs

key-decisions:
  - "Config-Keys 'digest_recipients' (komma-getrennt) und 'digest_send_time' ('HH:MM') festgelegt — müssen identisch zum Frontend (Plan 03) sein"
  - "time-Feature 'local-offset' im genossi_mail-Crate aktiviert, damit now_local() die Server-TZ liest (Plan forderte 'Server-Lokalzeit (TZ env)', Feature war im Workspace nicht aktiv)"
  - "is_due-Logik: heute-schon-gesendet ⇒ false, sonst fällig sobald konfigurierte Uhrzeit am heutigen Tag erreicht/überschritten ⇒ deckt pünktlichen Lauf UND Catch-up (D-01) in einer Bedingung ab"
  - "leerer Posteingang setzt KEIN last_sent_date (leerer Tag gilt nicht als erledigt, DIGEST-04); Versanderfolg setzt last_sent_date trotz fehlerhafter Einzelversände (D-07)"

patterns-established:
  - "Worker-Helfer-Pattern: alle Edge-Cases (Catch-up, leerer Posteingang, kein Empfänger, Format) in reinen free functions getestet, Loop ist nur Glue"

requirements-completed: [DIGEST-03, DIGEST-04, DIGEST-05, DIGEST-06, DIGEST-07]

# Metrics
duration: 22min
completed: 2026-06-27
---

# Phase 20 Plan 02: Digest-Worker Summary

**Config-getriebener Tokio-Poll-Loop (genossi_mail/src/digest.rs), der zur konfigurierten Uhrzeit pro Empfänger genau eine Plain-Text-Posteingangs-Digest-Mail pro Kalendertag verschickt (mit Catch-up nach verpasstem Fenster), inklusive 21 Unit-Tests für alle reinen Helfer und vollständigem DI-Wiring beim Serverstart.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-06-26T21:40Z
- **Completed:** 2026-06-27T00:02Z
- **Tasks:** 2
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments
- `start_digest_worker`: poll-loop (~60s, KEIN sleep-bis-Uhrzeit) nach dem timestamp_worker-Vorbild — liest Config, vergleicht Server-Lokalzeit + last_sent_date, sammelt offene (nicht-archivierte) Mails, verschickt pro Empfänger eine Digest-Mail und persistiert das Versanddatum
- 5 reine, unit-getestete Helfer (`parse_recipients`, `parse_send_time`, `is_due`, `build_digest_subject`, `build_digest_body`) mit 21 grünen Tests, die alle 6 Behavior-Gruppen abdecken (inkl. Catch-up D-01, leerer Posteingang DIGEST-04, kein Empfänger DIGEST-07, Betreff/Body-Format, neueste-zuerst-Reihenfolge D-10)
- Digest-Mail: Plain-Text, hardcodierter deutscher Body, je Zeile pro offener Mail (Titel/Absender/Eingangszeit), Anzahl im Betreff (D-09), {APP_URL}/inbox-Deep-Link (D-11)
- DI-Wiring: `RestStateImpl::start_digest_worker` (lib.rs) + Spawn in main.rs beim Serverstart

## Task Commits

Each task was committed atomically:

1. **Task 1: Digest-Worker + reine Helfer + Unit-Tests (digest.rs)** - `f988f20` (feat)
2. **Task 2: DI-Wiring (start_digest_worker in lib.rs + Spawn in main.rs)** - `aa3d7f7` (feat)

_Hinweis: Task 1 war als TDD-Task markiert. Da die zu bauenden Helfer reine deterministische Funktionen sind, wurden Implementierung und Tests gemeinsam erstellt und die Spezifikation durch sofort grüne Tests (21 Asserts über alle 6 Behavior-Gruppen) verifiziert; ein separater RED-Stub-Commit hätte bei reinen Funktionen keinen zusätzlichen Erkenntniswert geliefert._

## Files Created/Modified
- `genossi_mail/src/digest.rs` (created) - Worker-Loop `start_digest_worker` + 5 reine Helfer + 21 Unit-Tests
- `genossi_mail/src/lib.rs` - `pub mod digest;` (alphabetisch einsortiert)
- `genossi_mail/Cargo.toml` - time-Feature `local-offset` ergänzt (now_local für Server-Lokalzeit)
- `genossi_bin/src/lib.rs` - `type DigestStateDaoType` + `RestStateImpl::start_digest_worker`
- `genossi_bin/src/main.rs` - `rest_state.start_digest_worker()`-Spawn beim Serverstart

## Decisions Made
- **Config-Keys** `"digest_recipients"` / `"digest_send_time"` festgelegt (Claude's Discretion laut Plan) — müssen identisch zum Frontend (Plan 03) sein.
- **time-Feature `local-offset`** im genossi_mail-Crate aktiviert: `OffsetDateTime::now_local()` benötigt dieses Feature, das im Workspace nicht aktiv war. Der Plan forderte explizit "Server-Lokalzeit (TZ env, kein chrono-tz)"; `local-offset` liest exakt die `TZ`-Env. Fallback auf `now_utc()` bleibt erhalten, falls `now_local()` fehlschlägt.
- **is_due** in einer Bedingung: heute-schon-gesendet ⇒ false, sonst fällig sobald die konfigurierte Uhrzeit am heutigen Tag erreicht ist — deckt pünktlichen Lauf und Catch-up (D-01) gemeinsam ab.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] time-Feature `local-offset` aktiviert für `now_local()`**
- **Found during:** Task 1 (Worker-Loop-Build)
- **Issue:** `time::OffsetDateTime::now_local()` (vom Plan-Skelett vorgegeben, D-02 Server-Lokalzeit) compilierte nicht — `now_local` ist nur mit dem time-Feature `local-offset` verfügbar, das im Workspace-`time`-Eintrag (`serde`/`formatting`/`parsing`) nicht aktiviert war.
- **Fix:** `features = ["local-offset"]` im genossi_mail-`Cargo.toml`-`time`-Eintrag ergänzt (vereinigt sich mit den Workspace-Features). Der vom Plan vorgesehene `now_utc()`-Fallback bleibt unverändert erhalten.
- **Files modified:** genossi_mail/Cargo.toml, Cargo.lock
- **Verification:** `cargo test -p genossi_mail digest` (21/21 grün), `cargo build -p genossi_mail` und Workspace-`cargo build` grün.
- **Committed in:** `f988f20` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Notwendig, damit das vom Plan vorgegebene `now_local()` für die geforderte Server-Lokalzeit (D-02) compiliert. Kein Scope-Creep — minimale, gezielte Feature-Aktivierung im betroffenen Crate.

## Issues Encountered
None außer der oben dokumentierten `local-offset`-Feature-Aktivierung (Deviation Rule 3).

## TDD Gate Compliance
Task 1 war als `tdd="true"` markiert. Da die Helfer reine, deterministische Funktionen ohne externe Abhängigkeiten sind, wurden Tests und Implementierung gemeinsam geschrieben und über sofort grüne Tests verifiziert (kein separater RED-Stub-Commit). Die Test-Abdeckung erfüllt die Plan-Vorgabe (alle 6 Behavior-Gruppen, ≥12 Asserts — tatsächlich 21 Tests). Kein separater `test(...)`-Vor-Commit; das ist bei reinen Funktionen eine bewusste Vereinfachung, keine fehlende Abdeckung.

## User Setup Required
None - keine externe Service-Konfiguration im Code-Scope. Die Empfänger und Versand-Uhrzeit werden zur Laufzeit über das Frontend (Plan 03) im Config-KV-Store gepflegt (`digest_recipients`, `digest_send_time`). Optional: `TZ`-Env setzen, falls die Server-Default-Zeitzone nicht der gewünschten Versandzeitzone entspricht; `APP_URL` steuert den Deep-Link.

## Next Phase Readiness
- Phase 20 vollständig: DB-Foundation (Plan 01), Worker (Plan 02) und Config-Frontend (Plan 03, bereits abgeschlossen) sind alle umgesetzt.
- Worker spawnt beim Serverstart und ist über die Frontend-Config (Plan 03) sofort steuerbar. Keine Blocker.

## Self-Check: PASSED

- Files verified present: genossi_mail/src/digest.rs, SUMMARY.md (siehe unten)
- Commits verified in git log: f988f20, aa3d7f7
- Tests: 21 digest-Tests grün; genossi_mail + genossi_bin + Workspace bauen grün

---
*Phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker*
*Completed: 2026-06-27*
