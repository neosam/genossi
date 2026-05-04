---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
last_updated: "2026-05-04T08:35:21.359Z"
progress:
  total_phases: 5
  completed_phases: 2
  total_plans: 19
  completed_plans: 18
  percent: 95
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
Plan: 6 of 6 (Plans 01 + 02 + 03 + 04 + 05 complete)
**Phase:** 3
**Plan:** 05 — AttendanceServiceImpl + close_assembly Cascade (DONE 2026-05-04)
**Status:** Executing Phase 03 (Wave 1 + Wave 2 + Wave 3 done — Plans 01+02+03+04+05 done; 06 pending)
**Progress:** [█████████▓] 95%

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
| Phase 03 P02 | ~10 min | 1 tasks | 3 files |
| Phase 03 P03 | ~7 min | 1 tasks | 1 files |
| Phase 03 P04 | 8min | 2 tasks | 3 files |
| Phase 03 P05 | ~13 min | 2 tasks | 4 files |

## Accumulated Context

### Key Decisions (carry-over aus PROJECT.md)

| Decision | Rationale | Phase |
|----------|-----------|-------|
| GV als eigene Entität (`Assembly`) statt globalem Zustand | Mehrere GVs pro Jahr, Historie für Protokoll, klare Lifecycle-Grenzen | Phase 1 |
| Anwesenheit als Join-Tabelle, nicht Member-Flag | Saubere Mehrfach-GV-Historie, kein Datenverlust | Phase 3 |
| Plan 03-01: `search: Option<String>` statt `Option<&str>` in DAO-Trait | `async_trait` + `automock` verlangen named lifetime auf borrowed Option-Parametern; owned String ist projekt-konsistent (DAO allokiert intern eh `format!("%{}%", ...)` neu) | Phase 3, Plan 01 |
| Plan 03-02: hand-rolled `TestHelperTokenDao`-Mock liegt in `genossi_service_impl/src/helper_token.rs::tests` (NICHT in `assembly.rs::tests` wie das Plan-Dokument behauptete) | DAO-Trait-Erweiterungen müssen den existierenden hand-rolled Mock synchron pflegen, sonst E0046 in `cargo test -p genossi_service_impl`. Plan 05 wird einen **neuen** Mock in `assembly.rs::tests` anlegen, der ebenfalls die neue Method listen muss. | Phase 3, Plan 02 |
| Plan 03-03: `ClaimContext::as_helper`-Signatur ist `Option<Uuid>`, NICHT `Option<(Arc<str>, Uuid)>` — `session_id` ist NICHT Teil des Phase-2-Helper-Claims-JSON (verifiziert in `genossi_service_impl/src/session.rs:17-30`); sie liegt in `AuthenticatedContext.user_id` (Format `helper:<token_id>`) bzw. wird via `HelperTokenDao::list_session_ids_for_assembly` im Cascade-Pfad gelesen. Plan 05 destrukturiert `Some(helper_aid)` als reine `Uuid`. | Phase 3, Plan 03 |
| Plan 03-03: `permission::MockContext` (`genossi_service/src/permission.rs:150`) ist Unit-Struct ohne `Default`-Impl — distinkt von `auth_types::MockContext`. Plan 05+ Test-Fixtures müssen Unit-Construct-Syntax `MockContext` verwenden, kein `::default()`. | Phase 3, Plan 03 |
| Plan 03-04: AttendanceService-Trait-Test 3 nutzt `#[test]` (sync) statt `#[tokio::test]` (async) — `genossi_service` hat keine tokio dev-dependency. Test verifiziert nur die `#[automock]`-Builder-API (`expect_*` für alle 4 Methods); der reale `await`-Pfad wird in Plan 06's REST-Tests gegen `AttendanceServiceImpl` exerciert. Symmetrisches Pattern für zukünftige genossi_service-Trait-Tests. | Phase 3, Plan 04 |
| Plan 03-04: AttendanceMemberTO-PII-Guard hat 3 Verteidigungslinien: (1) strikte 7-Feld-Whitelist auf Struct-Ebene, (2) Doc-Comment-Verbot für `From<&MemberTO>`, (3) Konversion exklusiv aus `AttendanceMemberRow` (DAO-7-Spalten-SELECT-Whitelist). Plan 06's E2E-Test verifiziert das gleiche Pattern auf HTTP-Response-JSON. | Phase 3, Plan 04 |
| Plan 03-05: `check_assembly_access` Permission-Funnel ist EXKLUSIV im AttendanceServiceImpl (NICHT im PermissionService) und wird von ALLEN 4 Endpoint-Methods als erster DAO-touchender Schritt aufgerufen. Helper-Branch terminiert mit `return Ok(assembly)` nach Status-Check — fällt NIEMALS in den admin-Branch durch (failure-closed). Admin-Branch hat KEINEN Status-Check (D-20, ASSY-06). | Phase 3, Plan 05 |
| Plan 03-05: `close_assembly` Cascade-Reihenfolge: `audited_update!` → `list_session_ids_for_assembly` (in tx) → `tx.commit()` → pool-loop `delete_session` mit `tracing::warn!` bei Fehlern. Conflict-2-Resolution: pool-basierte `delete_session` deadlockt gegen offenen BEGIN, daher MUSS commit VOR der Loop. Continue-on-Error mit Defense-in-Depth via Phase-2 D-18 verify_user_session-Status-Check. | Phase 3, Plan 05 |
| Plan 03-05: `genossi_bin` `helper_token_dao` wird BEVOR `AssemblyServiceImpl`-Construction angelegt und dann mit `HelperTokenServiceImpl` per `Arc::clone` geteilt — exakt EIN HelperTokenDaoImpl pro Prozess. Pattern-Anker für künftige Service-DAO-Sharing-Setups. | Phase 3, Plan 05 |
| Plan 03-05: Hand-rolled `TestHelperTokenDao` + `TestPermissionDao` Mocks duplizieren bewusst den existierenden Mock in `helper_token.rs::tests` (Pitfall 4). Beide Test-Module müssen synchron mitgepflegt werden, weil mockall-`automock` den `Transaction`-Type hartkodiert; cross-Module-Sharing erfordert `pub(crate)`-Refactor (out-of-scope). | Phase 3, Plan 05 |
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

**Last action:** Plan 03-05 (AttendanceServiceImpl + close_assembly Cascade-Invalidation) komplett — AttendanceServiceImpl mit zentralem `check_assembly_access` Permission-Funnel + 4 Endpoint-Methods (list_members/mark_present/mark_absent/stats), AssemblyServiceImpl::close_assembly um Cascade-Loop erweitert (Conflict-2: commit BEFORE pool-loop, Continue-on-Error mit tracing::warn!), 2 neue Deps (HelperTokenDao + PermissionDao) auch in genossi_bin gewired, 19 grüne Service-Layer-Tests (14 attendance + 4 cascade + 1 Phase-1-Regression), 2 Task-Commits (`8624b1c`, `4a18d62`), SUMMARY.md geschrieben.

**Next action:** Plan 03-06 (REST + E2E — Wave 4, letzter Plan dieser Phase). REST-Handler binden gegen AttendanceService, RestStateImpl bekommt attendance_service-Field, E2E-Tests für SYNC-02-Race + Cascade-DB + PII-Leak + Hash-Chain-Stability + Post-Close-Edit + Helfer-vs-Vorstand.

**Files written this session (Plan 05):**

- `genossi_service_impl/src/attendance.rs` (NEW — AttendanceServiceImpl + check_assembly_access + 4 Methods + 14 Modul-Tests + TestContext + handgeschriebene Mocks)
- `genossi_service_impl/src/lib.rs` (MOD — `pub mod attendance;`)
- `genossi_service_impl/src/assembly.rs` (MOD — gen_service_impl-Erweiterung um HelperTokenDao + PermissionDao + close_assembly-Cascade-Body + 4 Cascade-Tests + handgeschriebene TestHelperTokenDao/TestPermissionDao Mocks)
- `genossi_bin/src/lib.rs` (MOD — AssemblyServiceDeps wiring + AssemblyServiceImpl-Construction um 2 neue Felder erweitert + helper_token_dao Construction-Order vorgezogen)
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-05-SUMMARY.md` (NEW)
- `.planning/STATE.md` (MOD — diese Aktualisierung)
- `.planning/ROADMAP.md` (MOD — Plan 05 Progress)
- `.planning/REQUIREMENTS.md` (MOD — ASSY-04 + ASSY-06 + ATTN-01..06 + SYNC-02 als Service-Vertrag complete)

---
*State initialized: 2026-05-02*
*Phase 03 Plan 01 completed: 2026-05-04*
*Phase 03 Plan 02 completed: 2026-05-04*
*Phase 03 Plan 03 completed: 2026-05-04*
*Phase 03 Plan 04 completed: 2026-05-04*
*Phase 03 Plan 05 completed: 2026-05-04*
