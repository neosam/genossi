---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
last_updated: "2026-05-04T07:36:33.355Z"
progress:
  total_phases: 5
  completed_phases: 2
  total_plans: 19
  completed_plans: 14
  percent: 74
---

# State: Genossi — GV-Anwesenheits-Erfassung

**Initialized:** 2026-05-02
**Last Updated:** 2026-05-02 (roadmap creation)

## Project Reference

**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit. Dieser Milestone bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

**Current Focus:** Phase 03 — attendance-aggregat-cascade-invalidation

**Granularity:** coarse (5 Phasen, 1–3 Plans pro Phase)

## Current Position

Phase: 03 (attendance-aggregat-cascade-invalidation) — EXECUTING
Plan: 2 of 6 (Plan 01 complete)
**Phase:** 3
**Plan:** 01 — DAO Foundation (DONE 2026-05-04)
**Status:** Executing Phase 03 (Wave 1 partial — Plan 01 done; 02/03/04 still pending)
**Progress:** [███████░░░] 74%

```
[ ] Phase 1: Assembly-Aggregat + Audit-Hardening                     0/0 plans
[ ] Phase 2: Helfer-Token + Session + AuthContext::Helper            0/0 plans
[ ] Phase 3: Attendance-Aggregat + Cascade-Invalidation              0/0 plans
[ ] Phase 4: Frontend (Component-First)                              0/0 plans
[ ] Phase 5: Pre-GV-Generalprobe und Operations-Plan                 0/0 plans

Overall: 0% complete
```

## Performance Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| v1 Requirements | 22 | Aus 4 Kategorien (ASSY/HLPR/ATTN/SYNC) |
| Coverage | 22/22 (100%) | Keine Orphans |
| Phases | 5 | coarse Granularity |
| Build Order | Backend-First → Frontend → Operations | Genossi-Konvention |
| Audit Scope | ASSY-* + HLPR-* lifecycle | ATTN-* explizit OFF (User-Decision) |
| Phase 03 P01 | 12min | 2 tasks | 5 files |

## Accumulated Context

### Key Decisions (carry-over aus PROJECT.md)

| Decision | Rationale | Phase |
|----------|-----------|-------|
| GV als eigene Entität (`Assembly`) statt globalem Zustand | Mehrere GVs pro Jahr, Historie für Protokoll, klare Lifecycle-Grenzen | Phase 1 |
| Anwesenheit als Join-Tabelle, nicht Member-Flag | Saubere Mehrfach-GV-Historie, kein Datenverlust | Phase 3 |
| Plan 03-01: `search: Option<String>` statt `Option<&str>` in DAO-Trait | `async_trait` + `automock` verlangen named lifetime auf borrowed Option-Parametern; owned String ist projekt-konsistent (DAO allokiert intern eh `format!("%{}%", ...)` neu) | Phase 3, Plan 01 |
| One-Time-Use-QR pro Helfer | Verhindert Token-Weitergabe an Unbefugte | Phase 2 |
| Helfer-Memo-Name = Freitext, kein Identitäts-Anker | Reine UX-Hilfe für Vorstand beim Drucken | Phase 2 |
| GV-Status final nach Schluss; Vorstand-Korrekturen ohne Re-Open | Vermeidet Status-Pingpong, hält Audit-Story einfach | Phase 1 |
| Manual-Code-Fallback (8–12 alphanumerisch) zusätzlich zu QR | iOS-Safari-Kamera-Quirks bekannt, Worst-Case auf echter GV vermeiden | Phase 4 |
| Atomarer Redeem via `UPDATE ... WHERE used_at IS NULL RETURNING ...` | Verhindert Race-Condition zwischen parallelen Scans | Phase 2 |
| Member-Universe-Snapshot beim GV-Öffnen | Stabiles Y im Counter, Late-Joins/Austritte verfälschen Protokoll nicht | Phase 1 |
| Sync nur bei Refresh, kein Live-Push | Doppel-Abhaken durch Idempotenz abgefangen, kein SSE/WebSocket nötig | Phase 3+4 |
| Anwesenheit ohne Audit-Hashchain | Verband fordert nur Anzahl, nicht den Anhakel-Vorgang | Phase 3 |
| Helfer-View auch für Vorstand zugänglich (ohne QR) | Vorstand will im Notfall mithelfen, kein UI-Duplikat | Phase 3+4 |

### Open TODOs

- Vor Phase 2 entscheiden: `tower-sessions` 0.14 → 0.15 Upgrade jetzt oder später? (HIGH-Risiko-Bewertung im Plan-Phase-Schritt)
- In Phase 5 dokumentieren: Server-Hosting-Entscheidung für GV-Tag (lokal im Vereinsheim vs. Cloud) — beeinflusst Backup-Plan

### Blockers

Keine.

### Skills / Conventions to Apply

- **Audit-Macros**: `audited_create!`, `audited_update!`, `audited_delete!` für Member/MemberAction/MemberDocument/Application-Operationen, die in den GV-Phasen mitlaufen, sowie für Assembly-Lifecycle und HelperPreToken-Erzeugung. NICHT für Anwesenheits-Markierungen.
- **Component-First**: `genossi-frontend/src/component/` (siehe `genossi-frontend/CLAUDE.md`); identische UI auf zwei Pages → eigene Component-Datei
- **Layered Architecture**: DAO trait → SQLite impl → Service trait → Service impl → REST handler — Reihenfolge je Phase
- **Optimistic Locking**: `version: Uuid` für Member/Application — bei Assembly identisch, beim Redeem-Token EXPLIZIT NICHT (dort `used_at IS NULL` als Constraint)
- **Soft-Delete**: `deleted: Option<PrimitiveDateTime>` — Standard für alle neuen Aggregate
- **ISO8601-Datetime**: `genossi_rest_types::iso8601_datetime` serde-Modul für alle TO-Datetime-Felder

## Session Continuity

**Last action:** Plan 03-01 (Attendance-Aggregat DAO) komplett — Migration + Trait + SQLite-Impl + 9 grüne Tests, 2 atomare Task-Commits (`56ae4fc`, `63a7371`), SUMMARY.md geschrieben.

**Next action:** Plan 03-02 (HelperTokenDao::list_session_ids_for_assembly), 03-03 (Cascade-Invalidation in close_assembly), oder 03-04 (AttendanceService Trait) — alle in Wave 1 parallelisierbar gegen Plan 01-Output.

**Files written this session:**

- `migrations/sqlite/20260504000000_create_attendance_table.sql` (NEW — Schema-DDL)
- `genossi_dao/src/attendance.rs` (NEW — Trait + Entities + 3 Tests)
- `genossi_dao_impl_sqlite/src/attendance.rs` (NEW — Impl + 6 Tests)
- `genossi_dao/src/lib.rs` (MOD — pub mod attendance)
- `genossi_dao_impl_sqlite/src/lib.rs` (MOD — pub mod attendance)
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-01-SUMMARY.md` (NEW)
- `.planning/STATE.md` (MOD — diese Aktualisierung)
- `.planning/ROADMAP.md` (MOD — Plan 01 Progress via SDK)
- `.planning/REQUIREMENTS.md` (MOD — ATTN-03/ATTN-04/SYNC-02 als complete markiert)

---
*State initialized: 2026-05-02*
*Phase 03 Plan 01 completed: 2026-05-04*
