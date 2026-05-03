# Phase 3: Attendance-Aggregat + Cascade-Invalidation – Research

**Researched:** 2026-05-03
**Domain:** Backend-Aggregat (DAO/Service/REST) + Cascade-Invalidation across Phase-2-Sessions
**Confidence:** HIGH (codebase verified end-to-end; alle relevanten Patterns existieren in Phase 1+2)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Attendance-Tabellen-Schema
- **D-01:** Tabelle `attendance`. Felder: `assembly_id` (BLOB UUID, FK auf `assembly.id`, RESTRICT), `member_id` (BLOB UUID, FK auf `member.id`, RESTRICT), `marked_at` (PrimitiveDateTime), `marked_by_user_id` (TEXT — synthetic `helper:<token_id>` ODER OIDC-Vorstands-User-ID), `deleted` (Option<PrimitiveDateTime>, Soft-Delete für Toggle-Off). KEIN `id`/`version`-Feld.
- **D-02:** Schema-Vorbild: `assembly_member_snapshot` (`genossi_dao/src/assembly_member_snapshot.rs`).
- **D-03:** Toggle-Off-Modell: Soft-Delete-Flip (`UPDATE attendance SET deleted = ? WHERE assembly_id = ? AND member_id = ?`).
- **D-04:** UNIQUE-Index `(assembly_id, member_id)` als regulärer UNIQUE (nicht partial) — durch UPSERT-Reuse-Pattern existiert immer genau eine Row pro Pair.
- **D-05:** Atomarer Toggle-On via SQLite UPSERT: `INSERT INTO attendance (...) VALUES (...) ON CONFLICT(assembly_id, member_id) DO UPDATE SET marked_at = excluded.marked_at, marked_by_user_id = excluded.marked_by_user_id, deleted = NULL`.
- **D-06:** Atomarer Toggle-Off via UPDATE-Soft-Delete; Service gibt 200 OK auch bei No-Op zurück (idempotent).
- **D-07:** Kein `unmarked_by`-Feld.
- **D-08:** Kein Audit — keine `Auditable`-Impl, keine `audited_*!`-Macros.
- **D-09:** Soft-Delete-Slot wird produktiv genutzt; Toggle-Off ist der einzige Schreib-Pfad, der ihn setzt.
- **D-10:** Migration-Filename: `YYYYMMDDHHMMSS_create_attendance_table.sql`.

#### Cascade-Invalidation in close_assembly
- **D-11:** `close_assembly` (in `genossi_service_impl/src/assembly.rs`) wird um aktive Cascade ergänzt: nach `audited_update!`, im selben Transaction-Scope.
- **D-12:** Cascade-Discovery via `helper_token.session_id`: Neue DAO-Method `HelperTokenDao::list_session_ids_for_assembly(assembly_id, tx) -> Vec<Arc<str>>`.
- **D-13:** Kein neuer `HelperSessionService`-Wrapper — `AssemblyServiceImpl::close_assembly` orchestriert direkt; O(N) DELETE-Aufrufe statt einem SQL.
- **D-14:** Phase-2-D-18-Status-Check (im `verify_user_session`-Pfad) bleibt als Defense-in-Depth bestehen.
- **D-15:** Reihenfolge im close_assembly-tx: (1) Status-Check, (2) Status-Update + audited_update!, (3) `list_session_ids_for_assembly` + Loop `delete_session`, (4) Commit. Bei DELETE-Fehler: Transaction-Rollback.
- **D-16:** AssemblyServiceImpl bekommt eine neue Dependency `HelperTokenDao`.

#### Permission-Branch für AuthContext::Helper
- **D-17:** Method `check_assembly_access(assembly_id, ctx, tx)` lebt im **AttendanceServiceImpl**, NICHT im PermissionService.
- **D-18:** Implementierung:
  ```text
  Authentication::Full → Ok(())
  Authentication::Context(AuthContext::Helper{assembly_id: helper_aid, ..}) →
      assembly = load_assembly(assembly_id)
      if helper_aid != assembly_id → PermissionDenied
      if assembly.status != Open → PermissionDenied
      Ok(())
  Authentication::Context(_other_) →
      permission_service.check_permission("admin", ctx)
  ```
- **D-19:** Vorstand reicht `admin`-Privilege. Keine neue `attendance.access`-Konstante.
- **D-20:** Vorstand-Post-Close-Edit: `check_assembly_access` macht für admin-Branch KEINEN Status-Check. Kein status-write-path.

#### Toggle-Endpoint-Design + Service-Layout
- **D-21:** Endpoints: `GET /api/attendance/{assembly_id}/members?q={substring}`, `PUT /api/attendance/{assembly_id}/{member_id}`, `DELETE /api/attendance/{assembly_id}/{member_id}`, `GET /api/assembly/{assembly_id}/stats`.
- **D-22:** Neuer Service `AttendanceService` mit Methods: `list_members`, `mark_present`, `mark_absent`, `stats`. Liegt in `genossi_service/src/attendance.rs` (Trait) + `genossi_service_impl/src/attendance.rs` (Impl).
- **D-23:** Deps: `AttendanceDao`, `AssemblyDao`, `MemberDao`, `AssemblyMemberSnapshotDao`, `PermissionService`, `TransactionDao`. Kein UuidService, kein AuditLogDao.
- **D-24:** Reduzierter Member-View — eigenes TO `AttendanceMemberTO` mit nur `member_number`, `first_name`, `last_name`, `salutation`, `title` plus `is_present: bool`. NICHT `MemberTO` mit serde-skip.
- **D-25:** Substring-Search wird im DAO ausgeführt (SQL `LIKE`).
- **D-26:** Permission-Status-Codes: 403 Forbidden (Helfer falsche assembly_id / GV nicht Open), 404 Not Found (assembly_id/member_id unbekannt), 200 OK (Erfolg).
- **D-27:** `mark_present`/`mark_absent` müssen prüfen, dass `member_id` im `assembly_member_snapshot` der GV ist (sonst 404).

#### Naming
- **D-28:** Code-Identifier englisch: `Attendance`, `AttendanceEntity`, `AttendanceDao`, `AttendanceService`, `AttendanceServiceImpl`, `AttendanceMemberTO`, `AttendanceStatsTO`. Tabelle `attendance`.

### Claude's Discretion
- **UNIQUE-Index-WHERE-Clause** (D-04): plain `UNIQUE(assembly_id, member_id)` vs. partial mit `WHERE deleted IS NULL` — finalisieren für UPSERT-Kompatibilität.
- **FK-ON-DELETE-Verhalten** für `attendance.assembly_id` und `attendance.member_id` — RESTRICT vs CASCADE.
- **Search-Min-Length / Max-Results** — kein Minimum, kein Pagination.
- **Stats-Polling-Rate-Limit** — Plan-Discretion.
- **Reihenfolge der Filter im Substring-LIKE-Query**.
- **Test-Strategie für UPSERT-Race** mit `tokio::join!`.
- **`stats`-Permission-Branch für Helper** — dürfen Live-Counter sehen.
- **Error-Strategie bei `delete_session`-Fehler in Cascade** — Rollback vs. Continue-on-Error.

### Deferred Ideas (OUT OF SCOPE)
- **Phase 4:** Anwesenheits-UI-Components, Live-Counter-Polling-Frontend, Connection-Banner, Manual-Code-Eingabe-UI.
- **Phase 5:** Stats-Polling unter realer GV-Last (Generalprobe).
- **Out of Scope:** Bulk-Mark-Endpoint, Pagination, eigene `attendance.access`-Privilege, Audit-Log für Vorstand-Post-Close-Edit, `unmarked_by`-Feld, Stats-View für geschlossene GVs (UI-seitig — Backend funktioniert), Pro-IP-Rate-Limit speziell für Stats-Polling.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **ASSY-04** | Vorstand sieht während offener GV einen Live-Counter „X von Y anwesend" | `GET /api/assembly/{id}/stats` (D-21) → `AttendanceServiceImpl::stats` lädt `attendance.count_present_by_assembly` + `assembly_member_snapshot_dao.count_by_assembly_id` (existiert seit Phase 1). Doku-Anker: §Architecture, §AttendanceStatsTO. |
| **ASSY-06** | Vorstand kann nach GV-Schluss Anwesenheits-Einträge ergänzen/entfernen; Status bleibt `Closed` | `check_assembly_access`-admin-Branch (D-18, D-20) ohne Status-Check. Doku-Anker: §Permission-Funnel, §Vorstand-Post-Close-Edit-Test. |
| **ATTN-01** | Helfer-API liefert nur Mitgliedsnummer/Name/Titel/Anrede — keine PII | `AttendanceMemberTO` mit 5 Feldern + `is_present: bool` (D-24); `From<&MemberEntity>` direkt (NICHT `From<MemberTO>`). Doku-Anker: §AttendanceMemberTO, §Pitfall „PII-Leak Guard". |
| **ATTN-02** | Substring-Suche auf Name oder Mitgliedsnummer | `AttendanceDao::list_members_for_assembly(aid, search: Option<&str>, tx)` mit SQL LIKE (D-25). Doku-Anker: §Substring-Search-Strategie. |
| **ATTN-03** | Idempotenter PUT — fünfmal 200 OK, ein Eintrag | SQLite UPSERT (D-05). Doku-Anker: §UPSERT-Pattern, §SYNC-02-Race-Test. |
| **ATTN-04** | Idempotentes Austragen — fünfmal DELETE liefert 5×200 | UPDATE-Soft-Delete (D-06). Doku-Anker: §Toggle-Off-Pattern. |
| **ATTN-05** | Anwesenheits-Markierungen werden NICHT in Audit-Hashchain | Kein `Auditable`-Impl, keine `audited_*!`-Aufrufe (D-08). Doku-Anker: §Hash-Chain-Stabilitätstest. |
| **ATTN-06** | Helfer-View ist auch für eingeloggte Vorstands-User aufrufbar | `check_assembly_access` admin-Branch (D-18). Doku-Anker: §Permission-Funnel. |
| **SYNC-02** | Doppel-Markierung erzeugt keinen Fehler/Duplikat | UPSERT-Atomik + Race-Test mit `tokio::join!` (Pattern bereits in HLPR-04 etabliert). Doku-Anker: §SYNC-02-Race-Test. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

| Direktive | Anwendung in Phase 3 |
|-----------|----------------------|
| **Layered Architecture (DAO/Service/REST/Bin)** | `AttendanceDao` (trait + sqlite-impl), `AttendanceService` (trait + impl), Axum-Handler, DI-Wiring in `genossi_bin/src/lib.rs`. |
| **Soft-Delete-Konvention** (`deleted: Option<PrimitiveDateTime>`) | Toggle-Off setzt `deleted` (D-03/D-09); Toggle-On setzt es zurück auf NULL via UPSERT-`SET deleted = NULL` (D-05). |
| **Optimistic Locking via `version: Uuid`** | **Nicht anwendbar** — `attendance` hat kein id/version (D-01). Idempotenz statt Locking. |
| **Audit-Macros NUR für Member/MemberAction/MemberDocument/Application** | Phase 3 fügt **keine** Audit-Macros hinzu (D-08, ATTN-05 explizit ausgeschlossen). |
| **ISO8601 Datetime via `iso8601_datetime` serde-Modul** | `AttendanceMemberTO` enthält **kein** Datetime-Feld (4-Felder-Reduktion). `AttendanceStatsTO` enthält nur u64-Counter. → kein serde-Datetime-Aufwand. |
| **Trait-basierte DI mit `gen_service_impl!`-Macro** | `AttendanceServiceImpl` wird via `gen_service_impl!` mit 6 Deps (D-23) generiert. |
| **Component-First Frontend** | **Phase 4** (out of scope). |
| **Endpoint-Naming englisch** | `/api/attendance/...`, `/api/assembly/{id}/stats` (D-21, D-28). |

---

## Summary

Phase 3 fügt das **Attendance-Aggregat** als leichtgewichtige Join-Tabelle ein (kein id/version, kein Audit), dessen einzige zwei Schreib-Pfade ein **idempotenter UPSERT** (Toggle-On) und ein **idempotenter UPDATE-Soft-Delete** (Toggle-Off) sind. Die Permission-Funnel `check_assembly_access` ersetzt den Phase-2-D-20-Stub durch eine positive Helfer-Branch (assembly_id-Match + Status=Open) und einen admin-Pass-Through ohne Status-Check (Vorstand-Post-Close-Edit ASSY-06). `close_assembly` wird um eine **aktive Cascade** ergänzt: alle `helper_token.session_id`-Einträge dieser GV werden über `permission_dao.delete_session` invalidiert, im selben Transaction-Scope.

Alle in CONTEXT.md gelockten Entscheidungen sind technisch sauber umsetzbar — die SQLite-Version (3.24+) ist projektweit etabliert, der UPSERT-Pattern ist bereits bei `permission.ensure_user_exists` im Code, und der Cascade-Loop folgt dem bestehenden Phase-1-`open_assembly`-Single-Transaction-Vorbild. **Eine kritische Inkompatibilität** wurde identifiziert: D-26 fordert 403 Forbidden für `PermissionDenied`-Fälle, aber `genossi_rest/src/lib.rs:106` mappt `ServiceError::PermissionDenied → RestError::Unauthorized → 401`. Die Phase-3-Plan muss hier entweder (a) den globalen `From<ServiceError>`-Mapping anpassen, (b) eine neue Error-Variante einführen, oder (c) im Handler explizit `RestError::Forbidden` zurückgeben (Pattern existiert bereits bei `redeem_helper_token` in `helper_token.rs:303-311`). Empfehlung: (c) — minimalinvasiv, identisch zum bewährten Helper-Token-Redeem-Pfad.

**Primary recommendation:** Implementiere die Phase in dieser Reihenfolge: (1) Migration + AttendanceDao-Trait + sqlite-Impl, (2) HelperTokenDao-Erweiterung um `list_session_ids_for_assembly`, (3) `AssemblyServiceImpl::close_assembly` Cascade-Erweiterung + AssemblyServiceDeps-Update, (4) AttendanceService-Trait + Domain-Types + AttendanceMemberTO/AttendanceStatsTO, (5) AttendanceServiceImpl mit `check_assembly_access`, `mark_present`/`mark_absent`/`list_members`/`stats`, (6) REST-Handler + Router-Nest + DI-Wiring, (7) E2E-Tests (Race, Cascade, PII-Leak, Hash-Chain-Stability, Post-Close-Edit). Wichtige Tests pro Plan, damit jede Schicht abgedeckt ist (CLAUDE.md global rule "Always make sure you have tests for the changes" + project rule "Layered Architecture").

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Attendance-Persistierung (UPSERT/Soft-Delete) | DAO (`AttendanceDaoImpl`) | — | Single-Statement-SQL, keine Business-Logik |
| Idempotenz-Garantie (ATTN-03/04, SYNC-02) | DAO (UPSERT-SQL) | Service (200-OK-auch-bei-No-Op-für-DELETE) | Race-Sicherheit ist DB-Eigenschaft, HTTP-Semantik ist Service-Vertrag |
| Permission-Funnel `check_assembly_access` | Service (`AttendanceServiceImpl`) | — | Domain-Aggregat-Wissen (Assembly-Status); D-17 explizit |
| Snapshot-Membership-Check (D-27) | Service | DAO (`AssemblyMemberSnapshotDao::find_by_assembly_id` o.ä.) | Validierung gehört in Service; DAO liefert die Daten |
| Stats `{present, total}` | Service | DAO (`AttendanceDao::count_present_by_assembly` + `AssemblyMemberSnapshotDao::count_by_assembly_id`) | Aggregat über zwei DAOs |
| Cascade-Discovery `list_session_ids_for_assembly` | DAO (`HelperTokenDao` neu) | — | Reine SELECT-Query auf bestehender Tabelle |
| Cascade-Orchestrierung in `close_assembly` | Service (`AssemblyServiceImpl`) | DAO (`HelperTokenDao` + `PermissionDao`) | D-13: kein neuer Wrapper-Service |
| Cookie-Header / HTTP-Status-Code-Mapping | REST (`genossi_rest/src/attendance.rs`) | — | Stat-Codes folgen `error_handler()`-Wrapper |
| Auth-Middleware (Cookie → AuthContext::Helper) | bereits Phase 2 (`SessionServiceImpl::extract_auth_context`) | — | Bleibt unverändert; Phase 3 konsumiert nur |

---

## Standard Stack

### Verifizierte Versionen (am 2026-05-03 gegen Cargo.lock + workspace-Cargo.toml)

| Crate | Version (in Genossi) | Phase-3-Verwendung | Verifikation |
|-------|----------------------|--------------------|--------------|
| `sqlx` | 0.8 | UPSERT mit `ON CONFLICT(...) DO UPDATE`, query_scalar für COUNT | [VERIFIED: STACK.md, Cargo.toml] |
| `tokio` | 1.35 (full) | `tokio::join!` für Race-Tests | [VERIFIED: STACK.md] |
| `axum` | 0.8.3 | Handler-Pattern, `Path((aid, mid))`, `Query<...>` für `?q=` | [VERIFIED: STACK.md] |
| `time` | 0.3 | `PrimitiveDateTime::now()` für `marked_at` | [VERIFIED: STACK.md] |
| `uuid` | 1.6 | `Uuid` für assembly_id/member_id-FKs | [VERIFIED: STACK.md] |
| `mockall` | 0.13 | `MockHelperTokenDao`-Erweiterung um `expect_list_session_ids_for_assembly` | [VERIFIED: STACK.md] |
| `tower_governor` | 0.6 | optionaler stats-rate-layer (Discretion) | [VERIFIED: lib.rs:468] |
| `utoipa` | 5.0 | OpenAPI-Schemas für `AttendanceMemberTO`, `AttendanceStatsTO`, `?q=`-Query-Param | [VERIFIED: STACK.md] |
| `async-trait` | (workspace) | DAO + Service-Traits | [VERIFIED: bestehende DAO-Files] |

### SQLite-Version
- **SQLite ≥ 3.24** ist Voraussetzung für UPSERT-Syntax (`ON CONFLICT ... DO UPDATE`) — bereits etabliert im Projekt durch `genossi_dao/src/permission.rs:23-40` (`ensure_user_exists` nutzt ON CONFLICT-Idiom indirekt) und durch `helper_token.rs::atomic_redeem` (UPDATE … RETURNING braucht ≥ 3.35). Phase 2 läuft, also ist die DB-Version für Phase 3 garantiert ausreichend. [CITED: SQLite docs.sqlite.org/lang_upsert.html — UPSERT seit 3.24.0 (2018-06-04)]

### Keine neuen Dependencies erforderlich
Phase 3 fügt **null** neue Crates zum Workspace hinzu. Alle benötigten Bausteine sind über Phase 1+2 bereits verfügbar.

---

## System Architecture Diagram

```text
┌──────────────────────────────────────────────────────────────────────────┐
│  Helfer (Browser)                                                         │
│  Cookie: app_session=helper:<assembly_uuid>:<token_id>                    │
│  oder Vorstand: Cookie: app_session=<oidc-session-id>                     │
└─────────────────────────────────┬──────────────────────────────────────┘
                                  │ HTTPS
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  Axum Router (`genossi_rest/src/lib.rs`)                                  │
│   .nest("/api/attendance/{aid}/...", attendance::generate_route(..))      │
│   .nest("/api/assembly/{aid}/stats", ...)  // ggf. inline statt nest      │
└─────────────────────────────────┬──────────────────────────────────────┘
                                  │
                                  │ Auth-Middleware kettet:
                                  │   1) extract_session_from_cookie
                                  │   2) verify_user_session  (D-18 Status-Check)
                                  │   3) extract_auth_context  (Phase 2 D-15/D-16)
                                  │      → AuthContext::Helper{session_id, assembly_id}
                                  │      ODER AuthContext::Mock(...)/Oidc(...)
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  REST Handler (`genossi_rest/src/attendance.rs`)                          │
│   - list_attendance_members(...)  GET  /api/attendance/{aid}/members?q=   │
│   - mark_attendance_present(...)  PUT  /api/attendance/{aid}/{mid}        │
│   - mark_attendance_absent(...)   DELETE /api/attendance/{aid}/{mid}      │
│   - get_assembly_stats(...)       GET  /api/assembly/{aid}/stats          │
│  Wrapped in error_handler(); ServiceError → RestError mapping.            │
└─────────────────────────────────┬──────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  Service Layer                                                            │
│   AttendanceServiceImpl<Deps>                                             │
│     ├─ check_assembly_access(aid, ctx, tx)  ⟵ ZENTRALER FUNNEL (D-17)     │
│     │    Authentication::Full      → Ok(())                                │
│     │    AuthContext::Helper{aid'} → match + status==Open (D-18)          │
│     │    AuthContext::Mock/Oidc    → permission_service.check("admin")    │
│     ├─ list_members(aid, q, ctx)                                           │
│     ├─ mark_present(aid, mid, ctx) ⟵ snapshot-Check + UPSERT             │
│     ├─ mark_absent (aid, mid, ctx) ⟵ snapshot-Check + UPDATE-soft-del    │
│     └─ stats(aid, ctx)             ⟵ count_present + count_total          │
│                                                                            │
│   AssemblyServiceImpl<Deps>  (ERWEITERT in Phase 3)                       │
│     close_assembly(aid):                                                   │
│       1. permission + status-check                                        │
│       2. audited_update! (status=Closed, closed_at=now())                  │
│       3. NEU: helper_token_dao.list_session_ids_for_assembly(aid, tx)     │
│       4. NEU: for sid in sids: permission_dao.delete_session(sid).await   │
│       5. transaction_dao.commit(tx)                                       │
└─────────────────────────────────┬──────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  DAO Layer                                                                │
│   AttendanceDao   (NEU)                                                   │
│     ├─ upsert_present(aid, mid, marked_at, marked_by_user_id, tx)         │
│     ├─ soft_delete (aid, mid, deleted_at, tx)                             │
│     ├─ list_members_for_assembly(aid, search: Option<&str>, tx)           │
│     │    JOIN assembly_member_snapshot ⨝ member ⟵ filter by snapshot     │
│     │    LEFT JOIN attendance ⟵ for is_present                            │
│     │    + LIKE-WHERE bei Some(q)                                         │
│     ├─ count_present_by_assembly(aid, tx)                                 │
│     └─ is_in_snapshot(aid, mid, tx)  ⟵ optional, kann auch im Service    │
│                                                                            │
│   HelperTokenDao  (ERWEITERT)                                             │
│     + list_session_ids_for_assembly(aid, tx) -> Vec<Arc<str>>             │
│                                                                            │
│   PermissionDao   (UNVERÄNDERT)                                           │
│     - delete_session(sid) ⟵ POOL-basiert, KEIN tx-Argument! (Pitfall)    │
└─────────────────────────────────┬──────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  SQLite (in-memory bei E2E, file-based in Prod)                           │
│   attendance(assembly_id, member_id, marked_at, marked_by_user_id, deleted)│
│     UNIQUE(assembly_id, member_id) — plain, kein WHERE                    │
│   assembly_member_snapshot(...)  ⟵ Phase 1                                │
│   helper_token(... session_id ...) ⟵ Phase 2                              │
│   session(id, ...)              ⟵ tower_sessions/PermissionDao            │
└──────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

Bestehende Struktur — Phase 3 fügt 5 neue Files und modifiziert 4 bestehende.

```text
genossi_dao/src/
├── attendance.rs             # NEU — AttendanceEntity + AttendanceDao trait
└── helper_token.rs           # MODIFY — list_session_ids_for_assembly hinzufügen

genossi_dao_impl_sqlite/src/
├── attendance.rs             # NEU — AttendanceDaoImpl (UPSERT, soft-delete, list, count)
└── helper_token.rs           # MODIFY — list_session_ids_for_assembly impl

genossi_service/src/
└── attendance.rs             # NEU — AttendanceService trait + Domain-Types

genossi_service_impl/src/
├── attendance.rs             # NEU — AttendanceServiceImpl + check_assembly_access
└── assembly.rs               # MODIFY — close_assembly Cascade-Loop + neue HelperTokenDao-Dep

genossi_rest/src/
└── attendance.rs             # NEU — 4 Handler + AttendanceRestState trait + Router

genossi_rest_types/src/
└── lib.rs                    # MODIFY — AttendanceMemberTO + AttendanceStatsTO + From<&MemberEntity>

genossi_rest/src/
└── lib.rs                    # MODIFY — generate_route registrieren, schemas in OpenApi-Doc

genossi_bin/src/
└── lib.rs                    # MODIFY — AttendanceServiceImpl-Wiring; HelperTokenDao zu AssemblyServiceDeps

migrations/sqlite/
└── 20260504000000_create_attendance_table.sql  # NEU
```

---

## Architecture Patterns

### Pattern 1: SQLite-UPSERT für idempotente Toggle-On (D-05)

**What:** Single-Statement Atomic-Insert-Or-Update. Race-frei, idempotent, kein separater INSERT/UPDATE-Branch.

**When to use:** Toggle-On-Operationen auf Tabellen mit UNIQUE-Constraint, bei denen Re-Toggle (Schreib-Pfad nochmal aufrufen) keinen Fehler erzeugen soll.

**SQL-Template (verbatim für `attendance.upsert_present`):**

```sql
-- Source: SQLite UPSERT spec, docs.sqlite.org/lang_upsert.html (≥ 3.24)
-- Verifiziert gegen Genossi-Pattern in genossi_dao_impl_sqlite/permission.rs (UPSERT idiom).
INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted)
VALUES (?, ?, ?, ?, NULL)
ON CONFLICT(assembly_id, member_id) DO UPDATE SET
    marked_at = excluded.marked_at,
    marked_by_user_id = excluded.marked_by_user_id,
    deleted = NULL
```

**Bind-Reihenfolge (Rust):** `assembly_id (Vec<u8>)`, `member_id (Vec<u8>)`, `marked_at (String ISO8601)`, `marked_by_user_id (String)`. Folgt exakt dem Pattern aus `helper_token.rs::create` (Zeile 130–139 in `genossi_dao_impl_sqlite/src/helper_token.rs`).

**Wichtig:** `ON CONFLICT(assembly_id, member_id)` braucht **plain UNIQUE** (kein partial WHERE) — siehe §Discretion-Auflösung „UNIQUE-Index-WHERE-Clause".

### Pattern 2: UPDATE-Soft-Delete für idempotente Toggle-Off (D-06)

```sql
-- Idempotent: 0-affected rows ist OK (No-Op = bereits absent oder noch nie markiert).
-- Service gibt 200 OK zurück, egal ob 0 oder 1 row betroffen.
UPDATE attendance
   SET deleted = ?
 WHERE assembly_id = ? AND member_id = ?
```

Bind: `deleted (String ISO8601)`, `assembly_id (Vec<u8>)`, `member_id (Vec<u8>)`. Service ignoriert `rows_affected()` (kein 404, anders als helper_token.set_session_id).

### Pattern 3: Substring-LIKE-Query mit JOIN auf snapshot (D-25, ATTN-02)

**What:** Mitgliederliste = Snapshot der Assembly ⨝ Member, optional gefiltert nach Substring auf Name oder Mitgliedsnummer, mit `is_present`-Flag aus LEFT JOIN auf attendance.

**SQL-Template (verbatim für `AttendanceDaoImpl::list_members_for_assembly`):**

```sql
-- ATTN-01: nur Snapshot-Mitglieder; ATTN-02: optionales Substring-Filter.
-- LEFT JOIN auf attendance liefert is_present in einer Query (vermeidet N+1).
-- COLLATE NOCASE auf den Suchspalten — gibt case-insensitive matching ohne ILIKE
-- (SQLite kennt kein ILIKE; siehe §Discretion-Auflösung „Substring-Search").
SELECT
    m.id,
    m.member_number,
    m.first_name,
    m.last_name,
    m.salutation,
    m.title,
    CASE WHEN a.assembly_id IS NOT NULL AND a.deleted IS NULL THEN 1 ELSE 0 END AS is_present
FROM assembly_member_snapshot s
JOIN member m ON m.id = s.member_id AND m.deleted IS NULL
LEFT JOIN attendance a
    ON a.assembly_id = s.assembly_id AND a.member_id = m.id
WHERE s.assembly_id = ?
  AND ( ? IS NULL                                                                 -- kein Filter
        OR (m.last_name  || ' ' || m.first_name)  LIKE ? COLLATE NOCASE
        OR CAST(m.member_number AS TEXT)          LIKE ?
      )
ORDER BY m.last_name COLLATE NOCASE, m.first_name COLLATE NOCASE
```

**Bind-Reihenfolge:** `assembly_id (Vec<u8>)`, `search.is_some_then_pass_dummy_else_NULL (Option<String>)`, `pattern_with_percent (String)`, `pattern_with_percent (String)`.

Genauer: Im Rust-Code sollte `Option<&str>` so gehandhabt werden:
```rust
let pattern: Option<String> = search.map(|s| format!("%{}%", s.trim()));
let q_marker: Option<&str> = pattern.as_deref();   // None → SQL ? IS NULL trifft → kein Filter
```

Für `search = None` setzt sqlx alle drei Marker auf NULL — die `OR ? IS NULL`-Klausel nimmt den ersten Branch und der Filter ist effektiv aus.

### Pattern 4: COUNT für stats-Endpoint (ASSY-04)

```sql
-- present-count: alle attendance-Rows mit deleted IS NULL für die Assembly
SELECT COUNT(*) FROM attendance
WHERE assembly_id = ? AND deleted IS NULL
```

`total` kommt aus `assembly_member_snapshot_dao.count_by_assembly_id(aid, tx)` — bereits in Phase 1 implementiert (`assembly_member_snapshot.rs:133-148`).

### Pattern 5: Cascade-Loop im close_assembly (D-11..D-15)

**Critical:** `PermissionDao::delete_session` nimmt **kein Transaction-Argument** an — siehe `genossi_dao/src/permission.rs:90`. Das ist eine Pool-basierte Operation, nicht Transaction-Scope-basiert. Für die Cascade-Reihenfolge bedeutet das: die Session-DELETEs sind nicht in derselben SQLite-Transaktion wie das Status-Update. Dieselbe Architektur-Eigenschaft hat HelperTokenServiceImpl::redeem_helper_token (siehe Kommentar in `genossi_service_impl/src/helper_token.rs:316-325` — die Service-Impl committet die Token-TX, dann ruft sie `permission_dao.create_session` außerhalb dieser TX auf, weil sonst sqlx-sqlite-pool-acquire deadlockt).

**Auswirkung auf D-15:** Die Reihenfolge `Status-Update → Cascade → Commit` muss adaptiert werden:

```text
1. Status-Check (Open zwingend)
2. audited_update! (status=Closed)  -- innerhalb tx
3. session_ids = helper_token_dao.list_session_ids_for_assembly(aid, tx)  -- innerhalb tx
4. transaction_dao.commit(tx)        -- TX SCHLIESSEN, BEVOR delete_session läuft
5. for sid in session_ids:
       permission_dao.delete_session(sid)  -- pool-basiert, außerhalb tx
       (Error-Handling siehe §Discretion-Auflösung)
```

Das bricht **D-15-Wortlaut** („Im selben Transaction-Scope, … (4) Commit"), aber respektiert das Architektur-Constraint, das schon Phase-2-D-25-Pattern erzwungen hat. Siehe §⚠ DECISION CONFLICT für Begründung.

### Pattern 6: `check_assembly_access`-Permission-Funnel

**Body-Template (verbatim für `AttendanceServiceImpl::check_assembly_access`):**

```rust
// Source: D-18 (CONTEXT.md), kombiniert mit bestehendem
//   permission_service.check_permission-Pattern (assembly.rs:80-81).
// Returns: Ok((assembly_entity)) damit Caller den Status nicht erneut laden muss.
//
// Pitfall: AuthContext::Mock(_) und Oidc(_) haben KEIN match-arm in der
// Helper-Branch. Der Catch-all `_` muss alle nicht-Helper-Cases auf den
// admin-Path routen — sonst PermissionDenied wenn AuthContext::Mock anliegt.
async fn check_assembly_access(
    &self,
    assembly_id: Uuid,
    context: Authentication<Self::Context>,
    tx: Self::Transaction,
) -> Result<AssemblyEntity, ServiceError> {
    let assembly = self
        .assembly_dao
        .find_by_id(assembly_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;

    match &context {
        Authentication::Full => return Ok(assembly),
        Authentication::Context(ctx) => {
            // Discriminate Helper from Mock/Oidc.
            // ClaimContext-Trait reicht hier nicht — wir brauchen die typisierte
            // Helper-Variante. Da Context generic ist (`Self::Context`), kann
            // dieser match nicht direkt auf AuthContext-Varianten greifen.
            // Lösung: AttendanceService bindet Context = AuthContext im Bin-DI
            // (oder die Discriminator-Logik wandert in eine
            // ClaimContext::is_helper_for_assembly(...)-Method).
            //
            // Praktischer Pfad: Bin verwendet `Context = AuthenticatedContext`
            // (oidc) bzw. `Context = MockContext` (mock_auth). Die Helper-
            // Branch ist NICHT über Self::Context erreichbar.
            //
            // ALTERNATIV (empfohlen): die check_assembly_access nimmt
            // AuthContext als zusätzlichen Parameter, der auf REST-Layer
            // direkt aus der Cookie-Extraction stammt und nicht durch den
            // generic-Context-Wrapper geht. So bleibt der bestehende
            // PermissionService-Vertrag intakt.
            //
            // Konkrete Empfehlung:
            //   AttendanceServiceImpl::list_members(aid, q, auth_context: AuthContext, ctx: Auth<Ctx>, tx)
            // wobei `auth_context` direkt aus der Cookie kommt (REST-Handler
            // ruft `extract_auth_context_typed(...)` auf, der die volle
            // AuthContext-Variante liefert) und `ctx` nur für den admin-Path
            // den PermissionService.check_permission braucht.
            //
            // → Siehe Open Question 1.
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context.clone())
                .await?;
            Ok(assembly)
        }
    }
}
```

**⚠ Wichtige architektonische Konsequenz:** `Self::Context` (das Generic-Argument im `gen_service_impl!`-Macro) ist im Mock-Build `MockContext` und im OIDC-Build `AuthenticatedContext`. **Es ist NICHT `AuthContext`** — die Discriminator-Logik für Helper vs. Vorstand sitzt in `extract_auth_context` (`genossi_service_impl/src/session.rs:161-232`), nicht im generic-Context-System. Damit `check_assembly_access` die Helper-Branch erkennen kann, muss der REST-Handler **zwei Arten von Auth-Information** weitergeben:

1. **Die volle `AuthContext`-Variante** (für Helper-Discrimination via `match`)
2. **`Authentication<Self::Context>`** (für die admin-PermissionService-Aufrufe)

Praktische Umsetzung — siehe Open Question 1.

### Pattern 7: REST-Handler mit `error_handler()`-Wrapper (Phase-2-helper-Vorbild)

```rust
// Source: helper_token.rs:91-140 (verbatim Pattern, angepasst auf attendance).
#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Attendance",
    path = "/{assembly_id}/{member_id}",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID"),
        ("member_id"   = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Marked present (idempotent)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (helper wrong assembly or assembly not Open)"),
        (status = 404, description = "Member not in snapshot or assembly not found"),
    ),
)]
pub async fn mark_attendance_present<RestState: RestStateDef + AttendanceRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, member_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context.clone()))?;
            // For Helper-discrimination: the REST handler resolves the
            // typed AuthContext from the same cookie/middleware chain that
            // populated `context`. See Open Question 1.
            rest_state
                .attendance_service()
                .mark_present(assembly_id, member_id, auth)
                .await
                .map_err(map_attendance_error)?;
            Ok(Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

// Differentielles ServiceError → RestError-Mapping für 403-Korrektheit (D-26).
fn map_attendance_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        // alle anderen → Standard-Mapping aus genossi_rest/src/lib.rs:95-111
        other => other.into(),
    }
}
```

### Anti-Patterns to Avoid

- **Kein `From<&MemberTO> for AttendanceMemberTO` einführen.** Die ROADMAP-Hard-Constraint Phase 3 fordert: "eigenes `AttendanceMemberTO` mit nur 4 Feldern (NICHT `MemberTO` mit serde-skip)". Die Konvertierung MUSS direkt von `&MemberEntity` (oder von `&Member` aus `genossi_service`) kommen — siehe §Pitfall „PII-Leak Guard".
- **Kein `BulkPresent`-Endpoint** (Out of Scope, CONTEXT §Deferred).
- **Kein Audit für post-close-edit** — derselbe Service-Pfad ohne `audited_*!`-Macros.
- **Keine eigene `attendance.access`-Privilege** — bleibt bei `admin` (D-19).
- **Kein `unmarked_by`-Feld** in der attendance-Tabelle (D-07).
- **Kein neuer `HelperSessionService`-Wrapper** für Cascade — direct DAO-Calls (D-13).

---

## ⚠ DECISION CONFLICTS

### Conflict 1: D-26 fordert „403 Forbidden" für PermissionDenied — bestehender Code mappt auf 401

**Befund:** D-26 in CONTEXT spezifiziert:
> 403 Forbidden — Helfer mit falscher `assembly_id` ODER Helfer auf geschlossener GV (D-18-Branch); nicht-admin User auf admin-only Endpoints

Aber `genossi_rest/src/lib.rs:106` definiert:
```rust
genossi_service::ServiceError::PermissionDenied => RestError::Unauthorized,  // → 401
```

Phase-1- und Phase-2-Endpoints liefern damit aktuell **401** für PermissionDenied. D-26 erwartet **403** für die Phase-3-Attendance-Endpoints.

**Resolution-Empfehlung:** Pattern aus `helper_token.rs:303-311` adoptieren — der Handler macht ein lokales differentielles Mapping:

```rust
fn map_attendance_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}
```

`RestError::Forbidden` existiert bereits (`lib.rs:84`, mappt auf HTTP 403). **Nicht** den globalen From-Mapping ändern — das würde Phase 1/2-Endpoints stillschweigend von 401 auf 403 kippen.

### Conflict 2: D-15 fordert Cascade „im selben Transaction-Scope" — `delete_session` ist Pool-basiert

**Befund:** D-15:
> Reihenfolge im close_assembly-tx: (1) Status-Check, (2) Status-Update + audited_update!, (3) `list_session_ids_for_assembly` + Loop `delete_session`, (4) Commit. Falls eine Session-DELETE fehlschlägt: Transaction-Rollback.

Aber `PermissionDao::delete_session(session_id: &str)` (`genossi_dao/src/permission.rs:90`) nimmt **kein Transaction-Argument** entgegen. Es ist eine Pool-based-Method. Phase 2 Helper-Token-Service hat genau dieses Problem dokumentiert (`genossi_service_impl/src/helper_token.rs:316-325`):

> The TX is committed BEFORE we touch any DAO that uses its own pool connection (permission_dao.create_session, permission_dao.ensure_user_exists). If the redeem-TX is still open while a parallel pool-acquire is requested in the same async task, sqlx-sqlite serialises pool acquires and the task deadlocks (an open BEGIN holds its connection; the next acquire waits indefinitely).

**Resolution-Empfehlung:** Die D-15-Reihenfolge muss adaptiert werden:

```text
Empfohlene neue Reihenfolge im close_assembly:
1. Status-Check (Open zwingend)
2. audited_update! (status=Closed, closed_at=now())  ← in tx
3. session_ids = helper_token_dao.list_session_ids_for_assembly(aid, tx.clone())  ← in tx
4. transaction_dao.commit(tx)  ← TX SCHLIESSEN ZUERST
5. for sid in session_ids:
       match permission_dao.delete_session(sid).await {
           Ok(()) => continue,
           Err(e) => tracing::warn!(error=?e, session_id=%sid, "cascade delete failed"),
       }
   // Continue-on-Error: Status-Close ist bereits committed, Defense-in-Depth
   // (D-14 Phase-2-D-18-Status-Check) deckt Race ab.
```

Das ist nicht die in D-15 vorgesehene **all-or-nothing-Tx**, sondern **Phase-1-Fail-Forward**: Der wichtige Schritt (Status=Closed) ist persistent, die Cascade ist Best-Effort. **Begründung:** der Phase-2-D-18-Status-Check ist explizit als Defense-in-Depth dokumentiert (D-14 in 03-CONTEXT) und deckt jeden Edge-Case ab, in dem eine Session-DELETE fehlschlägt — der nächste Helfer-Request würde sowieso über `extract_auth_context` und `verify_user_session` laufen, dort den Assembly-Status `Closed` erkennen und die Session invalidieren.

**Wenn der User strikt auf D-15-Wortlaut besteht** (Rollback), gibt es eine zweite Option:
- **Workaround:** Eine neue `PermissionDao::delete_sessions_in_tx(session_ids: &[String], tx: Self::Transaction)` einführen, die alle DELETEs in der bestehenden TX ausführt. Das bricht die Phase-2-Konvention auf, hält aber D-15 ein. Architektur-Implikation: PermissionDao kriegt erstmals einen `type Transaction = ...` (siehe Verlauf der `PermissionDao`-Trait — zur Zeit ohne Transaction-Type).

Die **erste Resolution-Empfehlung** (Continue-on-Error nach Commit) ist die minimalinvasive und projektkonsistente Variante.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Race-frei „mark present" trotz Doppel-Klick | `if not exists then INSERT else UPDATE`-Branching | SQLite UPSERT (Pattern 1) | Atomar, ein Statement, race-frei; spart einen Roundtrip |
| Mitgliederliste mit `is_present` aus zwei Queries | Service lädt members + lädt attendance + merged in Rust | Single SQL mit `LEFT JOIN attendance` (Pattern 3) | N+1-Vermeidung, eine DB-Roundtrip |
| Substring-Match in Rust-Memory | `members.iter().filter(|m| m.last_name.contains(q))` | SQL `LIKE` mit `COLLATE NOCASE` (D-25) | Index-fähig, skaliert über Genossenschafts-Größenordnungen hinaus |
| Cookie-Parsing für Helfer-Discrimination | Eigenes Cookie-Parsing in attendance-Handler | Bestehender `extract_auth_context` (Phase 2 D-15/D-16) liefert `AuthContext::Helper` | Phase 2 hat die Discriminator-Logik bereits zentralisiert |
| Session-Cascade-Discovery | Walk through alle session-Rows und parse claims-JSON | `helper_token.session_id`-Spalte (Phase 2 D-01) — Phase 3 fügt nur `list_session_ids_for_assembly`-DAO-Method hinzu (D-12) | session_id-FK ist genau für diesen Zweck eingeführt |
| Hash-Chain-Stabilität testen | Manuelles SHA256 nachrechnen | `GET /api/audit/verify` aufrufen — liefert `{valid: bool, broken_links: []}` (Pattern bereits in Phase 1 D-12, Phase 2 HLPR-07-Test) | Endpoint ist bereits etabliert; Stabilitätstest = `entries-count vor toggle == entries-count nach 100 toggles + valid==true` |

**Key insight:** Phase 3 ist überwiegend **Komposition bestehender Patterns**, kein neues Architektur-Element. Die einzigen wirklich neuen Bausteine sind die `attendance`-Tabelle, der UPSERT-SQL, und die `check_assembly_access`-Funnel-Method. Alles andere ist 1:1-Wiederverwendung.

---

## Discretion-Area-Auflösungen

### 1. UNIQUE-Index — plain UNIQUE, kein partial WHERE [VERIFIED: SQLite docs]

**Frage (D-04 / Hard-Constraint Phase 3):** Plain `UNIQUE(assembly_id, member_id)` vs. partial `UNIQUE(...) WHERE deleted IS NULL`.

**Resolution:** **Plain UNIQUE.** Begründung:

1. **SQLite-UPSERT-Anforderung:** `ON CONFLICT(assembly_id, member_id)` braucht einen Conflict-Target, der ein vollwertiges UNIQUE-Constraint oder ein vollwertiger UNIQUE-Index ist. Ein partieller Index (`CREATE UNIQUE INDEX ... WHERE ...`) ist als Conflict-Target nur dann nutzbar, wenn die WHERE-Klausel im SQL-Statement **identisch** repliziert wird via `WHERE`-Clause auf der INSERT — das ist beim ON-CONFLICT-Idiom nicht möglich. [CITED: SQLite docs.sqlite.org/lang_upsert.html "The conflict-target must specify a uniqueness constraint that already exists … not just any constraint."]
2. **Reuse-Pattern (D-05):** Durch Soft-Delete-Flip + UPSERT-Reuse existiert immer **genau eine Row** pro `(assembly_id, member_id)`-Pair — mit oder ohne `deleted`. Ein partial UNIQUE wäre semantisch redundant.
3. **ROADMAP-Hard-Constraint** „UNIQUE(assembly_id, member_id) WHERE deleted IS NULL" ist **funktional** erfüllt (es gibt nie zwei aktive Rows pro Pair), auch ohne dass die WHERE-Klausel in der DDL steht.

**Migration-Snippet:**
```sql
CREATE TABLE IF NOT EXISTS attendance (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    marked_at TEXT NOT NULL,
    marked_by_user_id TEXT NOT NULL,
    deleted TEXT,
    PRIMARY KEY (assembly_id, member_id),
    FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
    FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT
);
-- Composite-PK is automatically UNIQUE → ON CONFLICT(assembly_id, member_id) works.
-- KEIN separater UNIQUE-Index notwendig.

CREATE INDEX IF NOT EXISTS idx_attendance_assembly_present
    ON attendance(assembly_id) WHERE deleted IS NULL;
-- Optional: beschleunigt count_present_by_assembly + list_members_for_assembly LEFT JOIN.
```

### 2. FK-ON-DELETE-Verhalten — RESTRICT für beide [ASSUMED, but CONSISTENT with codebase pattern]

**Frage:** RESTRICT vs. CASCADE für `attendance.assembly_id` und `attendance.member_id`.

**Resolution:** **RESTRICT** für beide. Begründung:

1. **Konsistent mit Phase-2-Konvention** — `helper_token.assembly_id REFERENCES assembly(id) ON DELETE RESTRICT` (siehe `migrations/sqlite/20260503000000_create_helper_token_table.sql:22`).
2. **Soft-Delete ist Standard** — Hard-Delete von Assembly oder Member ist im Genossi-Codebase nicht vorgesehen (kein DELETE-Endpoint). RESTRICT macht eine versehentliche Hard-Delete sichtbar (Constraint-Violation), CASCADE würde sie still durchwinken und Anwesenheits-Historie für das GV-Protokoll löschen.
3. **Cleanup-Job-Implikation:** Es gibt **keinen** Cleanup-Job für historische Anwesenheits-Daten in v1 (out-of-scope). Falls in v2 ein „GV-Daten archivieren"-Pfad kommt, wäre das ein expliziter Service-Pfad mit Audit, nicht ein Cascade-DELETE.

**⚠ FK-Enforcement-Caveat:** Die FK-Klauseln werden **nur dann** erzwungen, wenn `PRAGMA foreign_keys = ON` gesetzt ist. Im aktuellen Codebase ist das **off by default** (siehe Kommentar in `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs:34-44` — WR-03-Note). Phase 3 erbt diese Eigenschaft. Die FK-Klauseln dokumentieren die Intent, aber Service-Layer-Validation (snapshot-membership-check, D-27) ist die operative Schutzschicht.

[ASSUMED] Ob Phase 3 das `PRAGMA foreign_keys = ON` global aktiviert, ist eine separate Architektur-Entscheidung, die hier nicht getroffen wird — Plan-Detail.

### 3. Substring-LIKE-Strategie — `last_name || ' ' || first_name` + Mitgliedsnummer-CAST mit `COLLATE NOCASE` [VERIFIED: SQLite docs]

**Frage:** Wie genau sollte der LIKE-Filter aussehen?

**Resolution:** Siehe Pattern 3. Konkret:

- **Concat statt OR-Chain auf einzelnen Spalten:** `(m.last_name || ' ' || m.first_name) LIKE ?` ist effizienter als zwei OR-Bedingungen, weil SQLite eine einzige String-Vergleichsoperation macht. Verbund mit Space (`' '`) statt direktem Append, damit „Maxi Müller" auch via Suchstring „Maxi M" gefunden wird (sonst würde es als „MaxiM" gelesen).
- **Mitgliedsnummer-CAST:** `CAST(m.member_number AS TEXT) LIKE ?` — nicht `member_number = ?`, weil eine echte Substring-Suche gefordert ist (z.B. „123" soll Mitglieds-Nr. 123, 1234, 1230 alle finden).
- **`COLLATE NOCASE`:** SQLite kennt **kein ILIKE**. `LIKE` ist standardmäßig case-insensitive für ASCII (das ist eine SQLite-Eigenheit), aber **nicht** für Unicode (Umlaute). `COLLATE NOCASE` macht den Vergleich für ASCII-Buchstaben case-insensitive; für deutsche Umlaute bleibt es case-sensitive (SQLite-Limitation, kein Bug). [CITED: SQLite docs.sqlite.org/lang_expr.html#like; SQLite-Limitation Beim Unicode-Casefolding.]
  - Das ist im Genossi-Kontext **akzeptabel**: Mitgliedsnamen werden in der Regel exakt geschrieben; Helfer tippen den Namen wie er auf der Mitgliederkarte steht.
- **Kein Min-Length / kein Pagination** (laut CONTEXT-Discretion). Genossenschafts-Größenordnung 10–500 Mitglieder, full-table-scan auf SQLite-in-Memory ist sub-millisecond.

### 4. Test-Strategie für UPSERT-Race (SYNC-02) — `tokio::join!` mit zwei reqwest-Clients [VERIFIED: Pattern in HLPR-04]

**Frage:** Wie genau sieht der SYNC-02-Race-Test aus?

**Resolution:** Adoptiere das Pattern aus `genossi_bin/tests/e2e_tests.rs:8784-8819` (HLPR-04-Race-Test):

```rust
#[tokio::test]
async fn test_attendance_upsert_race_one_row_two_200ok() {
    // Setup: GV anlegen + öffnen + 1 Member im Snapshot + 2 Helfer-Cookies
    let server = setup_with_member().await;
    let client_a = reqwest::Client::new();
    let client_b = reqwest::Client::new();
    let assembly_id = create_open_assembly_with_member(&client_a, &server, &member_id).await;

    let url = server.url(&format!("/api/attendance/{}/{}", assembly_id, member_id));

    // Two parallel PUTs from two helpers on the SAME (aid, mid).
    let (resp_a, resp_b) = tokio::join!(
        client_a.put(&url).send(),
        client_b.put(&url).send(),
    );
    let status_a = resp_a.unwrap().status();
    let status_b = resp_b.unwrap().status();

    // Both must be 200 OK (idempotent — ATTN-03 + SYNC-02).
    assert_eq!(status_a, StatusCode::OK);
    assert_eq!(status_b, StatusCode::OK);

    // Verify exactly one row in DB via GET .../members?q= and check is_present count == 1
    // (oder: separater diagnostic endpoint /api/attendance/{aid}/_count_for_member/{mid},
    //  aber das wäre out-of-scope. Praktischer: count via stats-endpoint).
    let stats_resp = client_a
        .get(server.url(&format!("/api/assembly/{}/stats", assembly_id)))
        .send()
        .await
        .unwrap();
    let stats: serde_json::Value = stats_resp.json().await.unwrap();
    assert_eq!(stats["present"], 1, "Race must produce exactly ONE present-row, not two");
}
```

**Sub-Test:** Race auf Toggle-Off (DELETE) — beide DELETEs müssen 200 OK liefern, danach `present == 0`. Plus Toggle-On-Off-Race (PUT + DELETE parallel) — Endzustand ist nicht-deterministisch (entweder 0 oder 1 present), aber die DB darf nicht in Fehler-State landen.

**Test-Lokation:** `genossi_bin/tests/e2e_tests.rs` — bestehende Datei erweitern, **keine** neue Datei erstellen (Konvention aus Phase 1+2).

### 5. Error-Strategie bei Cascade-`delete_session`-Fehler — Continue-on-Error nach Commit

Siehe **§DECISION CONFLICT 2** für Begründung. Empfehlung:

```rust
// In AssemblyServiceImpl::close_assembly, nach transaction_dao.commit(tx):
let mut failed = Vec::new();
for sid in session_ids.iter() {
    if let Err(e) = self.permission_dao.delete_session(sid.as_ref()).await {
        tracing::warn!(error=?e, session_id=%sid, assembly_id=%id,
                       "cascade delete_session failed; defense-in-depth via verify_user_session-Status-Check active");
        failed.push(sid.clone());
    }
}
// failed wird NICHT in der API-Response surfaced — Status-Close war erfolgreich.
// Der nächste Helfer-Request mit einer dieser failed-Sessions wird durch
// extract_auth_context (Phase 2 D-18) → AssemblyStatus::Closed → 401 abgewiesen.
```

### 6. Per-IP-Rate-Limit für Stats-Endpoint — kein dedicated Layer in Phase 3

**Frage:** Eigener Rate-Limit-Layer für `/api/assembly/{id}/stats`, weil Frontend Phase 4 ~5s polled?

**Resolution:** **Nein, Phase 3 fügt keinen dedicated Layer hinzu.** Begründung:

1. **Bestehender `api_rate_layer`** (60 req/min per IP, lib.rs:484-493) deckt /api/* ab. Bei 5s-Polling sind das 12 req/min/IP — weit unter 60.
2. **In der GV-Realität** sind 5–20 Helfer/Vorstands-Geräte gleichzeitig aktiv. 12×20 = 240 req/min für Stats. Das ist **pro IP** ein Problem nur, wenn ein Reverse-Proxy oder NAT alle hinter einer IP versteckt — das ist eine **Operations-Phase-5-Frage**, nicht Phase 3.
3. **Phase 5 (Generalprobe)** soll das real-world-Polling unter Vereinsheim-WiFi messen (CONTEXT §Deferred). Falls dort Rate-Limit-Hits gemessen werden, wird der Layer **dann** angepasst.

**Wenn der Plan auf der sicheren Seite landen will:** Ein per-IP-Limit wie `60 req/min/IP` (= 1 req/sec gemittelt, 60 burst) explizit auf `/api/assembly/{aid}/stats` legen — wäre additiv, kein bestehender Test bricht. Empfehlung: erst nach Phase 5.

### 7. `stats`-Permission für Helper — identisch mit anderen Attendance-Endpoints

**Frage:** Dürfen Helfer `/api/assembly/{id}/stats` aufrufen?

**Resolution:** **Ja, identisch zu `list_members`/`mark_present`/`mark_absent`.** Phase 4 wird wahrscheinlich Helfer-UI mit Live-Counter haben (z.B. „Du hast 3 von 17 markiert"). `check_assembly_access` greift identisch — kein Sondervertrag (CONTEXT §Discretion explizit).

---

## Konkrete Code-Recommendations

### `AttendanceEntity` (`genossi_dao/src/attendance.rs`)

```rust
use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

/// AttendanceEntity — D-01: 5 columns mirroring the attendance table.
///
/// **Lightweight join** between Assembly and Member with no own identity
/// (D-01: no id/version). Soft-delete-flip via `deleted` (D-09 — first
/// productive use of the soft-delete slot in a GV-aggregate).
///
/// Lifecycle: Toggle-On overwrites with `deleted=NULL`, Toggle-Off sets
/// `deleted=Some(now())`. UPSERT-Reuse-Pattern (D-05) ensures exactly one
/// row per `(assembly_id, member_id)` pair.
///
/// **Not Auditable** (D-08, ATTN-05) — no `Auditable` impl, no `audit_fields()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceEntity {
    pub assembly_id: Uuid,
    pub member_id: Uuid,
    pub marked_at: time::PrimitiveDateTime,
    pub marked_by_user_id: Arc<str>,
    pub deleted: Option<time::PrimitiveDateTime>,
}

/// Reduced 5-field projection for the Helper-View (D-24, ATTN-01).
/// Returned by list_members_for_assembly. NOT a full member record —
/// no PII fields (email, address, bank, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceMemberRow {
    pub member_id: Uuid,
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Arc<str>>,  // serialized as String, not the Salutation enum
    pub title: Option<Arc<str>>,
    pub is_present: bool,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AttendanceDao {
    type Transaction: crate::Transaction;

    /// D-05 atomic toggle-on via SQLite UPSERT.
    /// Idempotent: 5× call → 5× Ok(()) → exactly one row in attendance.
    async fn upsert_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        marked_at: time::PrimitiveDateTime,
        marked_by_user_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// D-06 atomic toggle-off via UPDATE soft-delete.
    /// Idempotent: 5× call on non-existent row → 5× Ok(()) (No-Op).
    async fn soft_delete(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        deleted_at: time::PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// ATTN-01 + ATTN-02: reduced member view filtered by snapshot
    /// membership, with optional substring filter. Single SQL with
    /// JOIN snapshot + LEFT JOIN attendance for is_present.
    async fn list_members_for_assembly(
        &self,
        assembly_id: Uuid,
        search: Option<&str>,
        tx: Self::Transaction,
    ) -> Result<Arc<[AttendanceMemberRow]>, DaoError>;

    /// ASSY-04: `present` counter for stats endpoint.
    async fn count_present_by_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError>;

    /// D-27: snapshot membership check.
    /// Returns true if member_id is in the snapshot of assembly_id.
    /// (Could also be answered by AssemblyMemberSnapshotDao, but co-locating
    /// here avoids a round-trip through two DAOs in mark_present/mark_absent.)
    async fn is_in_snapshot(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<bool, DaoError>;
}
```

### SQLx-DAO-Impl-Skelett (`genossi_dao_impl_sqlite/src/attendance.rs`)

```rust
use async_trait::async_trait;
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::assembly::parse_datetime;
use crate::TransactionImpl;

fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}

#[derive(Debug, sqlx::FromRow)]
struct AttendanceMemberRowDb {
    id: Vec<u8>,
    member_number: i64,
    first_name: String,
    last_name: String,
    salutation: Option<String>,
    title: Option<String>,
    is_present: i64,    // SQLite gibt CASE WHEN als INT
}

pub struct AttendanceDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AttendanceDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttendanceDao for AttendanceDaoImpl {
    type Transaction = TransactionImpl;

    async fn upsert_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        marked_at: PrimitiveDateTime,
        marked_by_user_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        sqlx::query(
            "INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted) \
             VALUES (?, ?, ?, ?, NULL) \
             ON CONFLICT(assembly_id, member_id) DO UPDATE SET \
                marked_at = excluded.marked_at, \
                marked_by_user_id = excluded.marked_by_user_id, \
                deleted = NULL",
        )
        .bind(assembly_id.as_bytes().to_vec())
        .bind(member_id.as_bytes().to_vec())
        .bind(format_dt(&marked_at)?)
        .bind(marked_by_user_id.to_string())
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }

    async fn soft_delete(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        deleted_at: PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // Idempotent: rows_affected wird IGNORIERT (0 = noch nie markiert ODER bereits absent).
        sqlx::query(
            "UPDATE attendance SET deleted = ? WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(format_dt(&deleted_at)?)
        .bind(assembly_id.as_bytes().to_vec())
        .bind(member_id.as_bytes().to_vec())
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }

    async fn list_members_for_assembly(
        &self,
        assembly_id: Uuid,
        search: Option<&str>,
        tx: Self::Transaction,
    ) -> Result<Arc<[AttendanceMemberRow]>, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let pattern = search.map(|s| format!("%{}%", s.trim()));
        let rows = sqlx::query_as::<_, AttendanceMemberRowDb>(
            "SELECT \
                m.id, m.member_number, m.first_name, m.last_name, \
                m.salutation, m.title, \
                CASE WHEN a.assembly_id IS NOT NULL AND a.deleted IS NULL THEN 1 ELSE 0 END AS is_present \
             FROM assembly_member_snapshot s \
             JOIN member m ON m.id = s.member_id AND m.deleted IS NULL \
             LEFT JOIN attendance a \
                 ON a.assembly_id = s.assembly_id AND a.member_id = m.id \
             WHERE s.assembly_id = ? \
               AND ( ? IS NULL \
                     OR (m.last_name || ' ' || m.first_name) LIKE ? COLLATE NOCASE \
                     OR CAST(m.member_number AS TEXT) LIKE ? \
                   ) \
             ORDER BY m.last_name COLLATE NOCASE, m.first_name COLLATE NOCASE",
        )
        .bind(aid)
        .bind(pattern.as_deref())   // Erstes ? für Filter-NULL-Check
        .bind(pattern.as_deref())   // Zweites ? für Name-LIKE
        .bind(pattern.as_deref())   // Drittes ? für Mitgliedsnummer-LIKE
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let result: Vec<AttendanceMemberRow> = rows.into_iter().map(|r| {
            AttendanceMemberRow {
                member_id: Uuid::from_slice(&r.id).expect("invalid member uuid"),
                member_number: r.member_number,
                first_name: Arc::from(r.first_name.as_str()),
                last_name: Arc::from(r.last_name.as_str()),
                salutation: r.salutation.map(|s| Arc::from(s.as_str())),
                title: r.title.map(|s| Arc::from(s.as_str())),
                is_present: r.is_present != 0,
            }
        }).collect();
        Ok(Arc::from(result))
    }

    async fn count_present_by_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM attendance WHERE assembly_id = ? AND deleted IS NULL",
        )
        .bind(aid)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(count as u64)
    }

    async fn is_in_snapshot(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<bool, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let mid = member_id.as_bytes().to_vec();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM assembly_member_snapshot \
             WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(aid)
        .bind(mid)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(count > 0)
    }
}
```

### `HelperTokenDao::list_session_ids_for_assembly` (Erweiterung)

**Trait-Erweiterung** in `genossi_dao/src/helper_token.rs` (zwischen Zeile 173 und Ende des Trait-Blocks):

```rust
/// Cascade-Discovery for AssemblyServiceImpl::close_assembly (Phase 3 D-12).
/// Returns all currently-bound helper-session ids for the given assembly.
/// Filters out null session_ids (revoked or never-redeemed tokens) and
/// soft-deleted token rows.
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError>;
```

**SQLx-Impl** in `genossi_dao_impl_sqlite/src/helper_token.rs`:

```rust
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError> {
    let aid = assembly_id.as_bytes().to_vec();
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT session_id FROM helper_token \
         WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL",
    )
    .bind(aid)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(rows.into_iter().map(|s| Arc::from(s.as_str())).collect())
}
```

### `AttendanceServiceImpl::mark_present` Body

```rust
async fn mark_present(
    &self,
    assembly_id: Uuid,
    member_id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<(), ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    // Permission funnel (D-17/D-18). Loads assembly once, returns it.
    let _assembly = self.check_assembly_access(assembly_id, context.clone(), tx.clone()).await?;

    // D-27: snapshot membership check — Helfer und Vorstand dürfen nur
    // Snapshot-Mitglieder markieren (kein arbitrary mid).
    if !self
        .attendance_dao
        .is_in_snapshot(assembly_id, member_id, tx.clone())
        .await?
    {
        return Err(ServiceError::EntityNotFound(member_id));
    }

    // current_user_id — used as marked_by_user_id (D-01).
    // For Helper: liefert "helper:<token_id>" (Phase 2 D-17).
    // For Vorstand: liefert die OIDC-User-ID.
    let user_id = self
        .permission_service
        .current_user_id(context)
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());

    let now = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());

    self.attendance_dao
        .upsert_present(assembly_id, member_id, now_pdt, &user_id, tx.clone())
        .await?;

    self.transaction_dao.commit(tx).await?;
    Ok(())
}
```

### `AttendanceServiceImpl::mark_absent` Body

```rust
async fn mark_absent(
    &self,
    assembly_id: Uuid,
    member_id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<(), ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let _assembly = self.check_assembly_access(assembly_id, context.clone(), tx.clone()).await?;

    // D-27: snapshot-check — but für mark_absent ist das eigentlich nur eine
    // Konsistenzprüfung. Idempotenz: Service liefert immer 200 OK, auch wenn
    // mid nicht im Snapshot — alternative wäre 404. CONTEXT D-26 sagt 404 für
    // member_id-not-in-snapshot, also zähle das hier noch dazu.
    if !self
        .attendance_dao
        .is_in_snapshot(assembly_id, member_id, tx.clone())
        .await?
    {
        return Err(ServiceError::EntityNotFound(member_id));
    }

    let now = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());

    // Idempotent: 0-affected rows ist OK.
    self.attendance_dao
        .soft_delete(assembly_id, member_id, now_pdt, tx.clone())
        .await?;

    self.transaction_dao.commit(tx).await?;
    Ok(())
}
```

### `AssemblyServiceImpl::close_assembly` Cascade-Erweiterung

```rust
async fn close_assembly(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    if entity.status != AssemblyStatus::Open {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot close assembly: status is '{}', expected 'Open'",
            entity.status.as_str()
        ))));
    }

    let now_offset = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
    entity.status = AssemblyStatus::Closed;
    entity.closed_at = Some(now_pdt);

    crate::audited_update!(self, self.assembly_dao, id, &entity, ASSEMBLY_PROCESS_CLOSE, &user_id, tx);

    // D-11/D-12 — Phase 3 cascade extension.
    // Discover session_ids INSIDE the tx (the helper_token table is read with
    // the same snapshot the audited_update sees).
    let session_ids = self
        .helper_token_dao
        .list_session_ids_for_assembly(id, tx.clone())
        .await?;

    // Commit the assembly-status TX BEFORE touching pool-based delete_session
    // (siehe ⚠ DECISION CONFLICT 2 + helper_token.rs:316-325 Phase-2-Pattern).
    self.transaction_dao.commit(tx).await?;

    // D-13/D-14: Continue-on-Error. Defense-in-depth via verify_user_session.
    for sid in session_ids.iter() {
        if let Err(e) = self.permission_dao.delete_session(sid.as_ref()).await {
            tracing::warn!(
                error = ?e, session_id = %sid, assembly_id = %id,
                "cascade delete_session failed; defense-in-depth via verify_user_session-Status-Check active"
            );
        }
    }

    Ok(Assembly::from(&entity))
}
```

**Neue Dependency in `AssemblyServiceDeps`** (in `gen_service_impl!`-Macro-Block in `genossi_service_impl/src/assembly.rs:50-60`):

```rust
gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        // NEU Phase 3 (D-16):
        HelperTokenDao: HelperTokenDao<Transaction = Self::Transaction> = helper_token_dao,
        // NEU Phase 3 (Cascade calls delete_session, which is on PermissionDao):
        PermissionDao: PermissionDao = permission_dao,
    }
}
```

### Reduzierter TO `AttendanceMemberTO` (`genossi_rest_types/src/lib.rs`)

```rust
/// Reduced helper-view of a member for the attendance UI (D-24, ATTN-01).
///
/// **PII-Leak Guard:** This TO has EXACTLY 5 string/scalar fields plus
/// `is_present`. NO email, NO address, NO IBAN, NO comments. The
/// `From<&AttendanceMemberRow>` impl explicitly does NOT delegate to
/// MemberTO::from — that would risk silently inheriting new fields when
/// MemberEntity grows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceMemberTO {
    /// Mitgliedsnummer (ATTN-01).
    pub member_number: i64,
    /// Vorname (ATTN-01).
    pub first_name: String,
    /// Nachname (ATTN-01).
    pub last_name: String,
    /// Anrede ("Herr"/"Frau"/"Firma" or null).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<String>,
    /// Akademischer Titel.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    /// Aktueller Anwesenheits-Status (ATTN-03/04).
    pub is_present: bool,
    /// Member-ID — frontend braucht das für PUT/DELETE-Requests auf
    /// `/api/attendance/{aid}/{mid}`. Kein PII (UUID).
    pub member_id: Uuid,
}

impl From<&genossi_dao::attendance::AttendanceMemberRow> for AttendanceMemberTO {
    fn from(r: &genossi_dao::attendance::AttendanceMemberRow) -> Self {
        Self {
            member_number: r.member_number,
            first_name: r.first_name.to_string(),
            last_name: r.last_name.to_string(),
            salutation: r.salutation.as_deref().map(String::from),
            title: r.title.as_deref().map(String::from),
            is_present: r.is_present,
            member_id: r.member_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceStatsTO {
    pub present: u64,
    pub total: u64,
}
```

**Wichtig:** `From<&MemberEntity>` ist **nicht** vorgesehen, weil die DAO-Reihenfolge Service-Layer-Kontrolle liefert: der DAO produziert `AttendanceMemberRow` (Pre-Joined-Projection), Service mappt zu `AttendanceMemberTO`. Falls MemberEntity in Zukunft wächst, bricht der Pfad nicht still — `AttendanceMemberRow` ist ein expliziter 7-Feld-DTO.

---

## Common Pitfalls

### Pitfall 1: SQLx `query_as!` vs `query_as::<_, T>` für UPSERT-RETURNING

**What goes wrong:** `sqlx::query_as!` (Macro mit Compile-time-Check) hat bekannten Bug bei RETURNING-Klauseln in SQLite — Nullability wird falsch inferiert (siehe Kommentar in `helper_token.rs:213-215`: "Pitfall 1: SQLx 0.8 RETURNING-nullability bug").

**Why it happens:** SQLx-Macro versucht prepared-statement-introspection, kommt mit RETURNING in SQLite nicht zurecht.

**How to avoid:** Phase 3 nutzt **kein RETURNING** im UPSERT (D-05 INSERT … ON CONFLICT … OHNE RETURNING). Damit ist das Risiko vermieden. Falls ein Plan-Detail später RETURNING braucht: nutze `query_as::<_, RowType>(...).fetch_optional(...)` (verbatim Pattern aus `helper_token.rs:217-228`).

### Pitfall 2: SQLx `cargo sqlx prepare` braucht es überhaupt?

**What goes wrong:** Wenn Phase 3 `query!`/`query_as!`-Macros nutzt, müsste `DATABASE_URL=sqlite:genossi.db cargo sqlx prepare` nach Schema-Änderung laufen — sonst bricht die Compilation.

**Why it happens:** Macros checken zur Compile-Zeit gegen ein offline-Cache (`.sqlx/`-Directory) oder gegen DATABASE_URL.

**How to avoid:** Phase 3 nutzt durchgehend `sqlx::query`, `sqlx::query_as::<_, T>(...)`, `sqlx::query_scalar` — **alle non-Macro-Varianten** (Pattern aus Phase 2 helper_token.rs verifiziert). Damit ist `cargo sqlx prepare` **nicht erforderlich**, und das `.sqlx/`-Directory wird nicht touched. Phase 3 fügt nur eine neue Migration hinzu, keine Compile-Time-Query-Checks. [VERIFIED: grep `query!\|query_as!` in genossi_dao_impl_sqlite/src/ — keine Treffer.]

### Pitfall 3: SQLite WAL-Mode vs. UPSERT-Concurrency

**What goes wrong:** SQLite WAL-Mode lässt Reads parallel laufen, aber Writes serialisieren weiterhin auf einem einzigen `BEGIN IMMEDIATE`-Lock. Bei zwei parallelen UPSERTs auf dieselbe Row stehen sie in der Queue — kein Deadlock, aber Wartezeit.

**Why it happens:** SQLite ist single-writer-DB. Concurrent Writes serialisieren via `sqlite3_busy_timeout`-Polling.

**How to avoid:** **Default-Behavior reicht.** Bei Race-Test (SYNC-02 mit `tokio::join!`) wird einer der zwei UPSERTs warten, beide kommen aber zum Erfolg (200 OK), weil UPSERT atomisch ist. Wartezeit ist sub-millisecond für in-memory-DB. **Wichtig für Test:** **kein** `expect_no_busy_timeout` — der Test verifiziert nur *Endzustand* (`stats.present == 1`), nicht Latenz.

[ASSUMED] Falls in der Produktion sehr viele parallele Toggles (>10 req/sec auf same row) auftreten, könnte die SQLite-Schreibsperre messbar werden. Bei realistischen GV-Größen (2–5 simultane Helfer-Markierungen) **kein Issue**.

### Pitfall 4: mockall-Erweiterung um `list_session_ids_for_assembly` ohne bestehende Tests zu brechen

**What goes wrong:** `MockHelperTokenDao` ist via `#[automock]` auto-generiert (`genossi_dao/src/helper_token.rs:73`). Wenn die neue Method dem Trait hinzugefügt wird, generiert `mockall` automatisch eine `expect_list_session_ids_for_assembly()`-Builder-Method. Bestehende Tests, die `MockHelperTokenDao::new()` nutzen, **brechen nicht**, solange die neue Method in keinem Test-Pfad aufgerufen wird (mockall toleriert nicht-erwartete Methoden, solange sie nicht aufgerufen werden).

**Why it happens:** `#[automock]` ist non-strict bei Default — Methods, die im Test nicht erwartet werden und nicht aufgerufen werden, lösen keinen Test-Fehler aus.

**How to avoid:** **Neue Test-Pfade in `assembly.rs::close_assembly`-Tests** (Phase 3) **müssen** `expect_list_session_ids_for_assembly` und `expect_delete_session` setzen. Bestehende Phase-1-Tests von `close_assembly` (`assembly.rs:838-859 test_close_assembly_from_preparation_returns_conflict`) bleiben grün, weil sie short-circuiten bevor die neuen DAOs erreicht werden.

**Aber:** Die `AssemblyServiceImpl`-Tests in `assembly.rs` (Phase 1) verwenden ein **handgeschriebenes mock_set** (siehe Zeile 397-466 — `mock! { pub TestAssemblyDao { ... } }` statt `MockAssemblyDao`). Das **handgeschriebene mock_set** muss ebenfalls um `HelperTokenDao` und `PermissionDao` erweitert werden. Das ist ein **mechanisches Refactoring**:

```rust
// Add inside assembly.rs::tests:
mock! {
    pub TestHelperTokenDao {}
    #[async_trait]
    impl HelperTokenDao for TestHelperTokenDao {
        type Transaction = TestTransaction;
        async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
        async fn create(&self, entity: &HelperTokenEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
        async fn update(&self, entity: &HelperTokenEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
        async fn atomic_redeem(&self, token_hash: &str, used_at: time::PrimitiveDateTime, tx: TestTransaction) -> Result<Option<(Uuid, Uuid)>, DaoError>;
        async fn set_session_id(&self, token_id: Uuid, session_id: &str, tx: TestTransaction) -> Result<(), DaoError>;
        async fn lookup_status(&self, token_hash: &str, tx: TestTransaction) -> Result<Option<(Option<time::PrimitiveDateTime>, Option<time::PrimitiveDateTime>)>, DaoError>;
        async fn all_for_assembly(&self, assembly_id: Uuid, tx: TestTransaction) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
        async fn list_session_ids_for_assembly(&self, assembly_id: Uuid, tx: TestTransaction) -> Result<Vec<Arc<str>>, DaoError>;
    }
}

mock! {
    pub TestPermissionDao {}
    #[async_trait]
    impl PermissionDao for TestPermissionDao {
        type Transaction = TestTransaction;
        async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError>;
        // ... alle anderen Methods abkupfern aus genossi_dao/src/permission.rs:8-93
        async fn delete_session(&self, session_id: &str) -> Result<(), DaoError>;
        // ...
    }
}
```

Das **handgeschriebene mock_set** macht die Test-Wartung schwerer, ist aber projektgegeben (siehe Comment: "genossi_dao::Mock*Dao types hardcode `Transaction = MockTransaction` via #[automock] so we cannot re-target them; we re-roll the mocks here against TestTransaction.").

### Pitfall 5: `time::PrimitiveDateTime`-Serialization in `AttendanceMemberTO`

**What goes wrong:** Falls jemand versehentlich ein Datetime-Feld in `AttendanceMemberTO` einfügt (z.B. `marked_at`), muss der ISO8601-serde verwendet werden — sonst serialisiert `time::PrimitiveDateTime` als JSON-Array statt String, was Frontend-Code bricht.

**Why it happens:** `time` Crate hat default-Serde-Derive, das ein nicht-standard-Format produziert.

**How to avoid:** **Phase 3 hat NULL Datetime-Felder in den TOs** (D-24 — explizit nur 4 Stamm-Felder + is_present + member_id). Wenn das Plan-Detail dennoch `marked_at` exposen soll (z.B. für „zuletzt markiert von …"-Debug), muss `#[serde(with = "iso8601_datetime", default)]` genutzt werden — Pattern in `AssemblyTO` (`genossi_rest_types/src/lib.rs:1042-1075`).

### Pitfall 6: PII-Leak-Guard für AttendanceMemberTO

**What goes wrong:** Ein Refactor fügt `AttendanceMemberTO::from(&MemberTO)` hinzu, weil das praktisch erscheint — und damit kommen ALLE neuen MemberTO-Felder (z.B. eine zukünftige `bank_account_iban`-Erweiterung) automatisch in den Helper-View.

**Why it happens:** Convenience-Konvertierung verschleiert die explizite 5-Field-Auswahl.

**How to avoid:**
1. **`From<&MemberTO> for AttendanceMemberTO` ist VERBOTEN.** Plan-Plan muss diese Direktive in den Code einbauen (z.B. als Doc-Comment auf `AttendanceMemberTO`, ggf. plus einen Compile-Test der einen `From<&MemberTO>`-Impl rejected — aber das ist Plan-Detail).
2. **Statisches PII-Leak-Test in E2E:**
   ```rust
   #[tokio::test]
   async fn test_attendance_member_to_has_no_pii_fields() {
       let server = setup_with_open_gv_and_member().await;
       let client = reqwest::Client::new();
       let resp = client.get(server.url(&format!("/api/attendance/{}/members", aid)))
           .send().await.unwrap();
       assert_eq!(resp.status(), StatusCode::OK);
       let json: serde_json::Value = resp.json().await.unwrap();
       let members = json.as_array().unwrap();
       assert!(!members.is_empty(), "test setup must include at least one snapshot member");
       let m = &members[0];
       // Whitelist-Approach: nur diese Keys dürfen vorkommen.
       let allowed = ["member_number", "first_name", "last_name", "salutation", "title", "is_present", "member_id"];
       for (key, _) in m.as_object().unwrap() {
           assert!(allowed.contains(&key.as_str()),
               "AttendanceMemberTO leaked unauthorized field: '{}'", key);
       }
       // Blacklist-Approach (defense-in-depth): explizit verbotene PII-Keys.
       for forbidden in ["email", "iban", "bank_account", "street", "house_number",
                         "postal_code", "city", "comment", "join_date", "exit_date"] {
           assert!(m.get(forbidden).is_none(),
               "AttendanceMemberTO leaked PII field '{}'", forbidden);
       }
   }
   ```

### Pitfall 7: Hash-Chain-Stabilitäts-Test (ATTN-05)

**What goes wrong:** Ein Bug fügt versehentlich ein `audited_create!` oder `audited_update!` für attendance hinzu (z.B. weil ein Generator-Tool das Pattern aus `helper_token.rs` kopiert). Dann hat der ATTN-05-Test 100 audit-Einträge mehr als erwartet.

**Why it happens:** Audit-Macros sind „opt-in", aber leicht versehentlich aktivierbar.

**How to avoid:** **Test-Mechanik wie in HLPR-07** (`e2e_tests.rs:9171-9279`). Konkret:

```rust
#[tokio::test]
async fn test_attendance_toggle_burst_does_not_pollute_audit_chain() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let assembly_id = create_open_assembly_with_members(&client, &server, 1).await;
    let member_id = first_snapshot_member_id(&client, &server, assembly_id).await;

    // Snapshot der Audit-Log-Größe VOR den Toggles.
    let resp_before = client.get(server.url("/api/audit?entity_type=attendance"))
        .send().await.unwrap();
    let before_paged: serde_json::Value = resp_before.json().await.unwrap();
    let count_before = before_paged["total"].as_u64().unwrap_or(0);

    // 50 PUT + 50 DELETE im Wechsel = 100 Toggles.
    let url = server.url(&format!("/api/attendance/{}/{}", assembly_id, member_id));
    for i in 0..100 {
        let resp = if i % 2 == 0 {
            client.put(&url).send().await.unwrap()
        } else {
            client.delete(&url).send().await.unwrap()
        };
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Audit-Log-Größe NACH den Toggles — muss UNVERÄNDERT sein (D-08).
    let resp_after = client.get(server.url("/api/audit?entity_type=attendance"))
        .send().await.unwrap();
    let after_paged: serde_json::Value = resp_after.json().await.unwrap();
    let count_after = after_paged["total"].as_u64().unwrap_or(0);
    assert_eq!(count_before, count_after,
        "ATTN-05: 100 attendance toggles must NOT add audit entries (count_before={}, count_after={})",
        count_before, count_after);

    // Defense-in-Depth: hash-chain bleibt valid (alle anderen Audit-Einträge intakt).
    let verify_resp = client.get(server.url("/api/audit/verify")).send().await.unwrap();
    let verify: genossi_rest_types::VerifyResponseTO = verify_resp.json().await.unwrap();
    assert!(verify.valid, "hash chain must remain valid");
    assert!(verify.broken_links.is_empty(), "no broken links allowed");
}
```

**Wichtig:** `audit?entity_type=attendance` muss mit Pagination-Query funktionieren — der Endpoint existiert seit Phase 1+2 (siehe `e2e_tests.rs:9258-9278`). Falls `total` nicht im paged-Response steht (sondern nur `entries`-Array), nimm `entries.as_array().unwrap().len() as u64`. Plan-Detail.

### Pitfall 8: Cascade-Test braucht ein Test-Setup mit echter Helfer-Session in der DB

**What goes wrong:** Ein naiver Cascade-Test versucht „erzeuge fake session manuell + close + verify session weg", scheitert aber weil mock_auth-build die Session-Discrimination via Cookie-Format `helper:<aid>:<tid>` macht (`MockSessionServiceImpl::extract_auth_context`, session.rs:1082-1101) — die DB-Session-Row ist für die Cookie-Erkennung im mock_auth-build **irrelevant**.

**Why it happens:** Der mock_auth-Pfad nutzt **kein** `verify_user_session` — der Cookie ist self-contained. Der echte OIDC-Pfad nutzt `verify_user_session` und liest die DB-Session-Row.

**How to avoid:** Cascade-Test in **mock_auth-build** kann nicht direkt via Cookie-Reject testen (siehe Kommentar `e2e_tests.rs:9156-9168`). Stattdessen verifiziert er den Cascade indirekt:

1. **Setup:** GV → öffnen → Helper-Token → Redeem (echter Pfad: schreibt session-Row in DB + setzt session_id auf helper_token).
2. **Pre-Close-Verifikation:** `SELECT COUNT(*) FROM session WHERE id IN (SELECT session_id FROM helper_token WHERE assembly_id = ?)` > 0.
3. **Action:** `POST /api/assembly/{aid}/close` → 200 OK.
4. **Post-Close-Verifikation:** Selbe Query → 0 Sessions (oder zumindest die spezifische Session weg).

**Test-Implementierung-Skelett:**

```rust
#[tokio::test]
async fn test_close_assembly_cascade_invalidates_helper_sessions() {
    let server = setup().await;
    let pool = server.pool().clone();   // ⚠ Erweiterung von TestServer: pool exposen
    let client = reqwest::Client::new();
    let assembly_id = create_open_assembly_for_helper_test(&client, &server).await;
    let (token_id, code) = create_helper_token_for_test(&client, &server, assembly_id, "Cascade-Anna").await;

    // Redeem: echter Pfad — schreibt session-Row in DB, setzt session_id auf helper_token.
    let resp = client.post(server.url("/api/helper/redeem"))
        .json(&serde_json::json!({ "code": code })).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Pre-Close: prüfe dass die Session in der DB existiert.
    let session_id_before: Option<String> = sqlx::query_scalar(
        "SELECT session_id FROM helper_token WHERE id = ?")
        .bind(token_id.as_bytes().to_vec())
        .fetch_one(&*pool).await.unwrap();
    let sid = session_id_before.expect("session_id muss nach Redeem gesetzt sein");
    let session_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session WHERE id = ?")
        .bind(&sid).fetch_one(&*pool).await.unwrap();
    assert_eq!(session_count_before, 1, "session-Row muss vor close existieren");

    // Close.
    let resp = client.post(server.url(&format!("/api/assembly/{}/close", assembly_id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Post-Close: session ist weg.
    let session_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session WHERE id = ?")
        .bind(&sid).fetch_one(&*pool).await.unwrap();
    assert_eq!(session_count_after, 0,
        "Cascade muss die session-Row löschen (D-11/D-12)");

    // Bonus: helper_token.session_id sollte SET NULL sein (FK-ON-DELETE-Verhalten).
    // Aber: PRAGMA foreign_keys ist im prod-Pool OFF (siehe assembly_member_snapshot.rs:34-44 WR-03).
    // Also kann die session_id auf der helper_token-Row weiter zeigen.
    // Nicht in diesem Test asserten — out-of-scope.
}
```

**⚠ Bei Test-Setup:** Der `TestServer` muss seinen `pool: Arc<SqlitePool>` exposed haben, damit der Test direkte SQL-Queries machen kann. Falls das nicht so ist (siehe `genossi_rest/src/test_server.rs`), muss das Plan-Detail ein `TestServer::pool()`-Getter hinzufügen oder das Setup-Pattern aus `e2e_tests.rs:3447 setup_with_pool` adoptieren.

---

## Vorstand-Post-Close-Edit-Test (ASSY-06)

```rust
#[tokio::test]
async fn test_vorstand_can_edit_attendance_after_close() {
    let server = setup_with_member().await;
    let client = reqwest::Client::new();
    let assembly_id = create_open_assembly_with_members(&client, &server, 1).await;
    let member_id = first_snapshot_member_id(&client, &server, assembly_id).await;

    // 1) Vorstand markiert Mitglied als anwesend (Open).
    let resp = client.put(server.url(&format!("/api/attendance/{}/{}", assembly_id, member_id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2) GV schließen.
    let resp = client.post(server.url(&format!("/api/assembly/{}/close", assembly_id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3) Vorstand entfernt nachträglich Anwesenheit (Closed).
    let resp = client.delete(server.url(&format!("/api/attendance/{}/{}", assembly_id, member_id)))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK,
        "ASSY-06: Vorstand muss nach close noch DELETE können (D-20)");

    // 4) Stats reflektiert die Änderung.
    let resp = client.get(server.url(&format!("/api/assembly/{}/stats", assembly_id)))
        .send().await.unwrap();
    let stats: AttendanceStatsTO = resp.json().await.unwrap();
    assert_eq!(stats.present, 0, "post-close-DELETE muss stats aktualisieren");
    assert_eq!(stats.total, 1);

    // 5) GV-Status bleibt Closed (kein Re-Open).
    let resp = client.get(server.url(&format!("/api/assembly/{}", assembly_id)))
        .send().await.unwrap();
    let detail: AssemblyDetailTO = resp.json().await.unwrap();
    assert_eq!(detail.assembly.status, AssemblyStatusTO::Closed,
        "ASSY-06: post-close-edit darf Status nicht ändern (D-20)");
}
```

---

## Open Questions

### 1. Helper-Discrimination im AttendanceServiceImpl — wie kommt die typisierte `AuthContext`-Variante in `check_assembly_access`?

**What we know:**
- `Self::Context` ist `MockContext` (mock_auth) bzw. `AuthenticatedContext` (oidc) — **nicht** `AuthContext`.
- `AuthContext::Helper` wird in `extract_auth_context` (session.rs:161) erzeugt und in den Axum-`Extension<Context>`-Layer gespeist (über die `context_extractor`-Middleware in lib.rs:619).
- Der REST-Handler erhält dann `Extension(context): Extension<Context>` — und `Context = MockContext|AuthenticatedContext`, **nicht** AuthContext.

**What's unclear:** Wie kommt die typisierte `AuthContext::Helper`-Variante in den Service? Pfad-Optionen:

- **A.** Auth-Middleware-Erweiterung: `Extension<AuthContext>` parallel zu `Extension<Context>` einfügen. Service nimmt beide entgegen.
- **B.** `ClaimContext`-Trait-Erweiterung um eine `as_helper(&self) -> Option<(Arc<str>, Uuid)>`-Method. `MockContext` und `AuthenticatedContext` implementieren das mit den jeweiligen Claim-Sources. Service ruft `ctx.as_helper()` auf, ohne `AuthContext` direkt zu kennen.
- **C.** Der Service erhält **AuthContext** als zusätzlichen Parameter (z.B. eine eigene `AttendanceAuthContext`-Variante). REST-Handler ruft `extract_auth_context_typed(...)` auf, der intern auf den `extract_session_from_cookie + verify_user_session`-Pfad zugreift, um die typisierte Variante zu rekonstruieren.

**Recommendation:** **Option B** — `ClaimContext::as_helper(&self) -> Option<(Arc<str>, Uuid)>`. Bestehende `ClaimContext`-Trait existiert bereits in `genossi_service/src/claim_context.rs`. Erweiterung wäre ein-Method, mock_auth-Build ruft `Option::None`, oidc-Build parst die claims-JSON und liefert `Some(...)` falls `kind == "helper"`. Damit ist die Discriminator-Logik typsicher und an einem zentralen Ort (parallel zu Phase-2-D-15/D-16-Pattern).

**Diese Frage ist plan-relevant** — siehe Plan/Researcher entscheidet vor dem Service-Impl-Plan.

### 2. `AttendanceMemberRow.salutation` — Enum oder String?

**What we know:** `MemberEntity.salutation: Option<Salutation>` (DAO-Layer, mit `Herr/Frau/Firma`-Enum). REST-Response soll laut ATTN-01 nur den **String** zeigen (Helfer brauchen kein Enum).

**What's unclear:** Soll `AttendanceMemberRow` schon mit `String` arbeiten oder `Option<Salutation>` aus der DB lesen und Service den String-Konvertierungs-Step machen?

**Recommendation:** **String** im `AttendanceMemberRow` direkt — der DAO macht `m.salutation as TEXT`-Read, gibt String. Spart eine Konvertierung im Service-Layer und matched die TO-Erwartung 1:1. **Riski-Cut:** Falls die Salutation in der DB als ISO-Code (numerisch) gespeichert wäre, würde das brechen — sie wird aber als TEXT gespeichert (siehe `member.rs:24-34 from_str`).

### 3. Behaviour von `mark_absent` für nicht-Snapshot-Member

**What we know:** D-26 sagt 404 für member_id-not-in-snapshot. D-27 sagt „mark_present/mark_absent müssen prüfen".

**What's unclear:** Soll `mark_absent` für ein Member, das im Snapshot ist aber **nie present markiert wurde** (also keine attendance-Row existiert), 200 oder 404 liefern?

**Recommendation:** **200 OK.** Begründung: D-06 sagt explizit „bei nicht-existierender Row (kein Match): No-Op, Service gibt 200 OK zurück (idempotent — fünfmaliges DELETE auf nicht-vorhandenem Eintrag liefert 5×200, ATTN-04 erfüllt)". Snapshot-Membership-Check passiert **vor** dem UPDATE, also der 404-Pfad greift nur wenn das Member nicht im Snapshot ist (z.B. Helfer versucht Anwesenheit für einen Nicht-GV-Teilnehmer zu setzen).

---

## Suggested File-Modification List (geordnet für Plan-Decomposition)

> Reihenfolge respektiert Compile-Order und Test-Isolation. Empfohlene 6 Plans für Phase 3 (analog zur Phase-2-Decomposition mit 8 Plans).

### Plan 1: Migration + AttendanceDao trait + sqlite-Impl

| Action | Path | Notes |
|--------|------|-------|
| CREATE | `migrations/sqlite/20260504000000_create_attendance_table.sql` | Schema mit Composite-PK, FK ON DELETE RESTRICT, partial index |
| CREATE | `genossi_dao/src/attendance.rs` | `AttendanceEntity`, `AttendanceMemberRow`, `AttendanceDao`-Trait + `#[automock]` |
| MODIFY | `genossi_dao/src/lib.rs` | `pub mod attendance;` hinzufügen |
| CREATE | `genossi_dao_impl_sqlite/src/attendance.rs` | `AttendanceDaoImpl` mit 5 Methods + Tests |
| MODIFY | `genossi_dao_impl_sqlite/src/lib.rs` | `pub mod attendance;` hinzufügen |
| CREATE | Tests in `genossi_dao_impl_sqlite/src/attendance.rs::tests` | UPSERT-Idempotenz, soft-delete, list-with-filter, count |

### Plan 2: HelperTokenDao-Erweiterung

| Action | Path | Notes |
|--------|------|-------|
| MODIFY | `genossi_dao/src/helper_token.rs` | `list_session_ids_for_assembly`-Method zum Trait |
| MODIFY | `genossi_dao_impl_sqlite/src/helper_token.rs` | SQLx-Impl der neuen Method + Test (mit Setup-DB inkl. helper_token-Rows mit/ohne session_id) |

### Plan 3: AssemblyServiceImpl Cascade-Erweiterung

| Action | Path | Notes |
|--------|------|-------|
| MODIFY | `genossi_service_impl/src/assembly.rs` | `gen_service_impl!`-Macro um `HelperTokenDao` + `PermissionDao` erweitern; `close_assembly`-Body um Cascade-Loop ergänzen (siehe Pattern 5) |
| MODIFY | `genossi_service_impl/src/assembly.rs::tests` | Neue Mocks für `TestHelperTokenDao` + `TestPermissionDao`, `TestDeps`-Update |
| ADD TEST | Neue Unit-Tests: `test_close_assembly_cascade_loops_through_session_ids`, `test_close_assembly_cascade_continues_on_delete_session_error` |

### Plan 4: AttendanceService trait + Domain-Types + REST-TOs

| Action | Path | Notes |
|--------|------|-------|
| CREATE | `genossi_service/src/attendance.rs` | `AttendanceService`-Trait + `AttendanceMember`, `AttendanceStats` Domain-Types + `#[automock]` |
| MODIFY | `genossi_service/src/lib.rs` | `pub mod attendance;` |
| MODIFY | `genossi_rest_types/src/lib.rs` | `AttendanceMemberTO`, `AttendanceStatsTO` mit `From<&AttendanceMemberRow>` impl |
| ADD TEST | `genossi_rest_types/src/lib.rs::tests` | Test der 5-Field-Begrenzung, kein PII serialisiert |

### Plan 5: AttendanceServiceImpl + ClaimContext-Erweiterung (Open Q1-Auflösung)

| Action | Path | Notes |
|--------|------|-------|
| MODIFY | `genossi_service/src/claim_context.rs` | `as_helper(&self) -> Option<(Arc<str>, Uuid)>`-Method (Recommendation Open Q1) |
| MODIFY | `genossi_service/src/permission.rs` | `MockContext::as_helper()` returns None |
| MODIFY | `genossi_service/src/auth_types.rs` | `AuthenticatedContext::as_helper()` parst claims-JSON für `kind == "helper"` |
| CREATE | `genossi_service_impl/src/attendance.rs` | `AttendanceServiceImpl` + `check_assembly_access` + 4 Methods + Tests mit `mock!{}`-Pattern (analog zu assembly.rs:397-466) |

### Plan 6: REST-Handler + Router + DI-Wiring + E2E

| Action | Path | Notes |
|--------|------|-------|
| CREATE | `genossi_rest/src/attendance.rs` | 4 Handler (list, mark_present, mark_absent, stats) + `AttendanceRestState`-Trait + `generate_route` + `ApiDoc` + differential `map_attendance_error` (Pattern 7) |
| MODIFY | `genossi_rest/src/lib.rs` | `pub mod attendance;` + Router-Nest + ApiDoc-Schemas registrieren |
| MODIFY | `genossi_bin/src/lib.rs` | `AttendanceServiceImpl`-Wiring (in `RestStateImpl::new`); `RestStateImpl`-Field `attendance_service`; `AttendanceRestState`-Impl; `helper_token_dao` zu `AssemblyServiceImpl`-Constructor hinzufügen (für Cascade) |
| MODIFY | `genossi_bin/tests/e2e_tests.rs` | Neue Tests: race (SYNC-02), cascade (D-11), PII-leak (ATTN-01), hash-chain-stability (ATTN-05), post-close-edit (ASSY-06), idempotency burst (ATTN-03/04), Helfer-vs-Vorstand-Permission (ATTN-06) |

**Geschätzte Plan-Anzahl:** 6 Plans (analog zur Granularität von Phase 2 mit 8 Plans).

---

## Source Hierarchy

### Primary (HIGH confidence)
- `/home/neosam/programming/rust/projects/genossi3/.planning/phases/03-attendance-aggregat-cascade-invalidation/03-CONTEXT.md` — alle 28 Locked Decisions
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` — projektweite Conventions
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/assembly.rs` — `close_assembly`-Vorlage (Phase 1)
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/helper_token.rs` — Atomic-DAO-Pattern + Pool-vs-TX-Caveat (Zeile 316-325)
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao/src/helper_token.rs` — DAO-Trait-Pattern, Auditable-Pattern (das Phase 3 NICHT nutzt)
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao_impl_sqlite/src/helper_token.rs` — Atomic-Redeem SQL-Pattern (Vorlage für UPSERT)
- `/home/neosam/programming/rust/projects/genossi3/genossi_dao/src/permission.rs` — `delete_session`-Signatur (kein tx-Argument!)
- `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/session.rs` — `extract_auth_context`/`verify_user_session`-Defense-in-Depth (D-14)
- `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/lib.rs` — `RestError`-Variants inkl. `Forbidden`/`Gone`, `error_handler`, Router-Pattern
- `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/helper_token.rs` — Differential-Error-Mapping-Pattern (Zeile 295-313)
- `/home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/e2e_tests.rs` — `tokio::join!`-Race-Pattern (HLPR-04, Zeile 8784-8819), Audit-Verify-Pattern (HLPR-07, 9171-9279)
- `/home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs` — `RestStateImpl`-Wiring-Pattern
- `/home/neosam/programming/rust/projects/genossi3/migrations/sqlite/20260503000000_create_helper_token_table.sql` — Migration-Vorbild Phase 2
- `/home/neosam/programming/rust/projects/genossi3/migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql` — Migration-Vorbild leichtgewichtige Join-Tabelle
- `/home/neosam/programming/rust/projects/genossi3/.planning/codebase/{ARCHITECTURE,STACK,CONVENTIONS,TESTING}.md` — Codebase-Maps

### Secondary (MEDIUM confidence)
- SQLite UPSERT spec (docs.sqlite.org/lang_upsert.html) — UPSERT seit 3.24.0 (2018-06-04) [CITED]
- SQLite LIKE/COLLATE NOCASE-Limitations bei Unicode (docs.sqlite.org/lang_expr.html#like) [CITED]

### Tertiary (LOW confidence)
- `[ASSUMED]` SQLite-WAL + Concurrent-UPSERT-Performance bei realistischen GV-Größen (sub-millisecond) — basiert auf Allgemein-Wissen, nicht in dieser Codebase gemessen.
- `[ASSUMED]` `PRAGMA foreign_keys = ON`-Aktivierung ist Plan-Detail, nicht Pflicht für Phase 3.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SQLite WAL + 2 parallele UPSERTs auf same row → sub-millisecond Wartezeit, beide kommen zum Erfolg | §Pitfall 3 | Race-Test könnte minimal langsamer sein als erwartet (weiterhin grün, aber Test-Latency-Watcher könnte triggern). Mitigation: Test-Asserts nur auf Endzustand, nicht auf Latenz. |
| A2 | Phase 3 aktiviert NICHT `PRAGMA foreign_keys = ON` global | §Discretion 2, §Pitfall 8 | Falls aktiviert: helper_token.session_id wird `SET NULL` (D-01 ON DELETE SET NULL) — Cascade-Test würde dann diesen Pfad zusätzlich beobachten. Kein funktionaler Bruch, nur Test-Asserts möglicherweise präziser. |
| A3 | `MemberEntity.salutation` und `.title` werden in der DB als TEXT gespeichert (nicht numeric) | Open Q2 | Falls als numeric: `AttendanceDaoImpl::list_members_for_assembly` müsste Salutation:: from_str-Konvertierung im Rust machen. Mitigation: existing schema in member.rs:24-34 confirmed TEXT-Pattern. |
| A4 | `audit?entity_type=attendance` Endpoint funktioniert mit Pagination und liefert `total`-Field | §Pitfall 7 | Falls `total` nicht im Response: Test nutzt `entries.len()` stattdessen. Plan-Detail. |
| A5 | `TestServer::pool()`-Getter existiert ODER ein analoges `setup_with_pool`-Pattern | §Pitfall 8 Cascade-Test | Falls nicht: Plan-Detail muss den Helper hinzufügen. Pattern ist in `e2e_tests.rs:3447 setup_with_pool` vorhanden. |
| A6 | `ClaimContext`-Trait kann um eine `as_helper`-Method erweitert werden, ohne bestehende Tests zu brechen | Open Q1 | Default-Impl `fn as_helper(&self) -> Option<...> { None }` lässt alle bestehenden Implementierungen (MockContext, AuthenticatedContext) trivially passieren. Mitigation: Default-Impl im Trait. |
| A7 | `marked_by_user_id` als plain TEXT-Spalte (nicht FK) ist akzeptabel — synthetic IDs `helper:<uuid>` sind keine FK-konformen User-IDs | §AttendanceEntity D-01 | Falls Audit-Forensik später sagt „wir wollen FK auf user(name)": muss eine Migration `marked_by_user_id` referenzieren. **Mitigation:** D-01 spezifiziert explizit „TEXT" und Phase-2-D-17 nutzt synthetic IDs derselben Form. |

**Wenn diese Assumptions falsch sind:** Plan-Detail muss adjustieren — alle Risiken sind low-impact, kein Bruch der gelockten D-01..D-28-Decisions.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `INSERT … OR IGNORE` (alt SQLite-Idiom für race-free insert) | `INSERT … ON CONFLICT DO UPDATE` (UPSERT) | SQLite 3.24.0, 2018 | UPSERT erlaubt expressives „update on conflict" mit excluded.*-Zugriff statt nur Ignore — exakt der Phase-3-D-05-Use-Case. |
| `DELETE FROM attendance WHERE …` (Hard Delete) | `UPDATE attendance SET deleted = ?` (Soft Delete) | Genossi-Konvention seit Phase 1 | Bewahrt Historie für Vorstand-Inspektion (D-03 begründet). |
| `SELECT all members + filter in Rust` | `SELECT … WHERE LIKE … COLLATE NOCASE` | Best Practice für Suchen | Index-fähig, einer Query, vermeidet Memory-Allocation für full-table-load. |

**Deprecated/outdated:**
- Keine — Phase 3 nutzt durchgehend etablierte Idiome aus Phase 1+2.

---

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack (sqlx 0.8 UPSERT, axum 0.8.3, tokio 1.35) | HIGH | Verifiziert gegen workspace-Cargo.toml und STACK.md |
| Architecture Patterns (UPSERT, soft-delete, JOIN+filter) | HIGH | Direkte Adoption aus Phase-1+2-Code; alle SQL-Snippets verifiziert gegen helper_token + assembly_member_snapshot |
| Permission-Funnel `check_assembly_access` | MEDIUM | Locked decisions klar (D-17/D-18), aber Discrimination-Mechanismus (Helper vs. Vorstand) braucht Open-Q1-Resolution vor Implementation |
| Cascade in close_assembly | HIGH | Pattern dokumentiert + DECISION-CONFLICT-2 explizit aufgedeckt mit empfohlener Continue-on-Error-Resolution |
| Common Pitfalls (mockall-Erweiterung, PII-leak, hash-chain-stability) | HIGH | Alle 8 Pitfalls direkt aus Phase-1+2-Code abgeleitet; je ein Test-Pattern pro Pitfall vorgeschlagen |
| FK-ON-DELETE-Empfehlung RESTRICT | MEDIUM | Konsistent mit Phase-2 (helper_token), aber FK-Enforcement de facto off (PRAGMA foreign_keys default off) — siehe A2 |
| Discretion-Resolutions (UNIQUE, LIKE, Race-Test) | HIGH | Alle technisch verifiziert gegen SQLite-Specs und HLPR-04-Test-Pattern |
| Test-Strategie (E2E + Unit) | HIGH | Bestehende Patterns aus Phase 1+2 reichen aus; kein neues Test-Framework |

**Research date:** 2026-05-03
**Valid until:** 2026-06-03 (30 Tage; Phase 3 ist eng am bestehenden Code orientiert, keine fast-moving externen Dependencies)

---

## RESEARCH COMPLETE

**Phase:** 3 — Attendance-Aggregat + Cascade-Invalidation
**Confidence:** HIGH

### Key Findings

1. **Alle 28 CONTEXT-Decisions sind technisch sauber umsetzbar** — die SQLite-Version (≥ 3.24, im Projekt etabliert seit Phase 2) unterstützt UPSERT direkt; das `assembly_member_snapshot`-Schema ist das passende Vorbild; das `gen_service_impl!`-Macro generiert das `AttendanceServiceImpl`-Skelett mit den 6 Deps aus D-23 trivially.

2. **⚠ Zwei DECISION-CONFLICTS aufgedeckt, beide mit minimalinvasiver Resolution:**
   - **Conflict 1 (D-26 vs. existing 401-Mapping):** Helper braucht ein lokales `map_attendance_error`, das `PermissionDenied → RestError::Forbidden(403)` mappt — Pattern existiert bereits bei `redeem_helper_token` (`helper_token.rs:303-311`).
   - **Conflict 2 (D-15 vs. PermissionDao::delete_session ohne tx):** Cascade-Reihenfolge muss angepasst werden — TX zuerst committen (Status=Closed persistent), dann Continue-on-Error für die Session-DELETEs. Defense-in-Depth via Phase-2-D-18-Status-Check (D-14 in 03-CONTEXT) deckt jeden Fail-Forward-Edge-Case ab.

3. **Critical PII-Leak-Guard:** `AttendanceMemberTO` MUSS direkt aus `AttendanceMemberRow` (nicht aus `MemberTO`) gebaut werden. E2E-Test mit Whitelist+Blacklist auf JSON-Keys empfohlen.

4. **Open Question 1 ist plan-relevant:** Wie kommt die typisierte `AuthContext::Helper`-Variante in `check_assembly_access`? Empfehlung: `ClaimContext`-Trait um eine `as_helper(&self) -> Option<(Arc<str>, Uuid)>`-Method erweitern. Default-Impl `None` bricht keine bestehenden Implementierungen.

5. **Test-Infrastruktur ist 100% wiederverwendbar.** `tokio::join!`-Race-Test (HLPR-04), Audit-Verify-Test (HLPR-07), Cascade-Test mit pool-direktem-SQL — alle Patterns existieren in `e2e_tests.rs`. Keine neue Test-Library, kein neues Setup.

6. **Keine neuen Workspace-Dependencies.** Phase 3 ist reine Komposition von sqlx 0.8 UPSERT, time 0.3, uuid 1.6, axum 0.8.3, mockall 0.13 — alle bereits aktiv.

7. **Plan-Decomposition-Empfehlung:** 6 Plans (Migration+DAO → HelperTokenDao-Erweiterung → AssemblyService-Cascade → Service-Trait+TOs → ServiceImpl+ClaimContext → REST+E2E) — analog zur Phase-2-Granularität.

### File Created
`/home/neosam/programming/rust/projects/genossi3/.planning/phases/03-attendance-aggregat-cascade-invalidation/03-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | Verifiziert gegen Cargo.toml |
| Architecture | HIGH | Direkte Adoption Phase-1+2-Pattern |
| Permission-Funnel Discrimination | MEDIUM | Open-Q1 bleibt Plan-relevant |
| Cascade-Strategie | HIGH | DECISION-CONFLICT 2 offen aufgedeckt + Resolution dokumentiert |
| Pitfalls (8 Stück) | HIGH | Jedes mit Test-Pattern + Code-Schnipsel |

### Open Questions
1. **Helper-Discrimination-Mechanismus** — empfohlen: `ClaimContext::as_helper()` (siehe Open Q1).
2. **`AttendanceMemberRow.salutation`** als String oder Enum (siehe Open Q2).
3. **`mark_absent` für member-not-yet-present** → 200 OK (Idempotenz, siehe Open Q3).

### Ready for Planning
Research complete. Planner kann auf dieser Basis 6 Plans erstellen. Open Question 1 sollte vor Plan 5 (AttendanceServiceImpl) geklärt sein — entweder via Quick-Discussion mit User oder durch Recommendation-Adoption (`ClaimContext::as_helper()` mit Default `None`). DECISION-CONFLICTS müssen vor Plan 1 (Migration) bzw. Plan 3 (Cascade) berücksichtigt werden — beide haben empfohlene Resolutions im Research dokumentiert.
