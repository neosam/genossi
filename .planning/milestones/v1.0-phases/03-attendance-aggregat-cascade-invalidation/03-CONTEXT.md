# Phase 3: Attendance-Aggregat + Cascade-Invalidation - Context

**Gathered:** 2026-05-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Backend-Aggregat für die Anwesenheits-Erfassung während einer offenen Generalversammlung. Phase 3 liefert eine eigene `attendance`-Tabelle (Join Assembly↔Member, leichtgewichtig, ohne Audit), eine reduzierte (DSGVO-konforme) Helfer-Mitgliederliste mit Substring-Suche, idempotente PUT/DELETE-Toggle-Endpoints, einen Live-Stats-Endpoint `{present, total}`, einen Vorstand-Post-Close-Edit-Pfad (über dieselben Endpoints) sowie eine aktive Cascade-Invalidation aller Helfer-Sessions in `close_assembly`. Außerdem wird der Phase-2-D-20-Stub (`AuthContext::Helper` → `PermissionDenied`) durch eine positive Permission-Branch ersetzt: Helfer dürfen Attendance-Endpoints aufrufen, wenn `claims.assembly_id == endpoint.assembly_id` UND die Assembly Status `Open` hat; Vorstand mit OIDC-Session erreicht denselben View über `admin`-Privilege.

**Phase 3 liefert NICHT:**
- Frontend für Anwesenheits-UI, Live-Counter, QR-Scanner, Manual-Code-Eingabe-UI (Phase 4)
- Audit-Log für Live-Anwesenheits-Markierungen (vom User explizit ausgeschlossen, ATTN-05)
- Audit-Log für reguläre Toggle-Aufrufe — auch nicht für Vorstand-Post-Close-Edit (entschieden in Discussion: derselbe Service-Pfad ohne Audit-Macros, GV-Status bleibt automatisch `Closed` da kein status-write-path)
- Re-Open-Pfad für geschlossene GVs (Out-of-Scope, REQUIREMENTS §Out of Scope)
- Bulk-Mark-Endpoints für mehrere Mitglieder gleichzeitig (nicht REQUIREMENTS-relevant — Helfer markieren einzeln)
- Pagination der Mitgliederliste (Substring-Suche reicht für Genossenschafts-Größenordnungen)
- Permissions feiner als `admin` (z. B. eigene `attendance.access`-Privilege) — `admin` reicht, konsistent mit Phase 1+2

</domain>

<decisions>
## Implementation Decisions

### Attendance-Tabellen-Schema
- **D-01:** Tabelle: `attendance`. Felder: `assembly_id` (BLOB UUID, FK auf `assembly.id`, RESTRICT — Soft-Delete-Konvention), `member_id` (BLOB UUID, FK auf `member.id`, RESTRICT), `marked_at` (PrimitiveDateTime — letzter Toggle-On-Zeitpunkt, wird beim UPSERT überschrieben), `marked_by_user_id` (TEXT — synthetic `helper:<token_id>` ODER OIDC-Vorstands-User-ID, wird beim UPSERT überschrieben), `deleted` (Option<PrimitiveDateTime>, Soft-Delete für Toggle-Off). KEIN `id`/`version`-Feld — Anwesenheit hat keine eigene Identität jenseits des `(assembly_id, member_id)`-Pairs und braucht kein Optimistic-Locking (Idempotenz löst Concurrency).
- **D-02:** Schema-Vorbild: `assembly_member_snapshot` (`genossi_dao/src/assembly_member_snapshot.rs`) — leichtgewichtige Join-Tabelle ohne Aggregat-Overhead, NICHT `assembly.rs` (vollwertiges Aggregat).
- **D-03:** Toggle-Off-Modell: **Soft-Delete-Flip** (`UPDATE attendance SET deleted = ? WHERE assembly_id = ? AND member_id = ?`) statt Hard-Delete. Vorteil: bewahrt Historie „wer hat zuletzt eingetragen" für Vorstand-Inspektion bei Helfer-Beschwerde; Toggle-On-Reuse spart einen INSERT-Roundtrip.
- **D-04:** UNIQUE-Index: `(assembly_id, member_id)` als regulärer UNIQUE (nicht partial) — durch UPSERT-Reuse-Pattern (D-05) existiert immer genau eine Row pro Pair, mit oder ohne `deleted`. ROADMAP-Hard-Constraint „UNIQUE WHERE deleted IS NULL" ist semantisch erfüllt; ob die UNIQUE-Definition technisch das WHERE braucht, ist Plan-Detail (für UPSERT-`ON CONFLICT(assembly_id, member_id)` muss der Constraint ohne WHERE stehen — SQLite-Anforderung).
- **D-05:** Atomarer Toggle-On via SQLite UPSERT: `INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted) VALUES (?, ?, ?, ?, NULL) ON CONFLICT(assembly_id, member_id) DO UPDATE SET marked_at = excluded.marked_at, marked_by_user_id = excluded.marked_by_user_id, deleted = NULL`. Ein einziges SQL-Statement; race-frei; fünfmaliges PUT erzeugt einen Eintrag (ATTN-03 erfüllt). SQLite ≥ 3.24 (vorausgesetzt im Projekt).
- **D-06:** Atomarer Toggle-Off via UPDATE-Soft-Delete: `UPDATE attendance SET deleted = ? WHERE assembly_id = ? AND member_id = ?`. Bei nicht-existierender Row (kein Match): No-Op, Service gibt 200 OK zurück (idempotent — fünfmaliges DELETE auf nicht-vorhandenem Eintrag liefert 5×200, ATTN-04 erfüllt).
- **D-07:** **Kein `unmarked_by`-Feld**. Begründung: minimaler Schema-Footprint; Vorstand braucht in Phase 3 nur „wer hat zuletzt eingetragen". Wer-hat-ausgetragen ist Edge-Case ohne Verband-Anforderung.
- **D-08:** **Kein Audit** — keine `Auditable`-Impl, keine `audited_*!`-Macros. Auch nicht für Vorstand-Post-Close-Edit. Konformität mit ATTN-05 (vom User explizit ausgeschlossen) und PROJECT.md-Constraint („neue GV-Entitäten benötigen Audit nicht").
- **D-09:** Soft-Delete-Slot wird produktiv genutzt (anders als bei `assembly`/`helper_token`, wo `deleted` reserviert ist) — Toggle-Off ist der einzige Schreib-Pfad, der ihn setzt; Toggle-On überschreibt ihn auf `NULL`. Phase 3 enthält damit den ersten echten Soft-Delete-Use-Case der GV-Aggregate.
- **D-10:** Migration-Filename: `YYYYMMDDHHMMSS_create_attendance_table.sql` (englisch, konsistent mit Phase-1/2-Konvention).

### Cascade-Invalidation in close_assembly
- **D-11:** `close_assembly` (in `genossi_service_impl/src/assembly.rs`) wird um eine aktive Cascade ergänzt: nach dem `audited_update!`-Aufruf, im selben Transaction-Scope, lädt der Service alle Helfer-Sessions dieser GV und löscht sie via `permission_dao.delete_session(sid, tx)`. Erfüllt ROADMAP-Phase-3-SC#8 („nach Schließen schlägt jeder Helfer-Request mit 401 fehl").
- **D-12:** Cascade-Discovery via `helper_token.session_id`: Neue DAO-Method `HelperTokenDao::list_session_ids_for_assembly(assembly_id, tx) -> Vec<Arc<str>>`, die `SELECT session_id FROM helper_token WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL` ausführt. Vorteil: nutzt direkt den `session_id`-FK, der in Phase 2 (D-01) genau dafür eingeführt wurde; PermissionDao bekommt keine helper-spezifische Method (Schicht-Trennung sauber).
- **D-13:** Kein neuer `HelperSessionService`-Wrapper — `AssemblyServiceImpl::close_assembly` orchestriert direkt: `list_session_ids_for_assembly()` → for each `permission_dao.delete_session(sid, tx)`. O(N) DELETE-Aufrufe statt einem SQL — bei realistischen GVs (5–20 Helfer) trivial; vermeidet Overengineering.
- **D-14:** Phase-2-D-18-Status-Check (im `verify_user_session`-Pfad) bleibt **als Defense-in-Depth** bestehen. Begründung: deckt Race zwischen Cascade-DELETE und gleichzeitigem Helfer-Request ab; deckt edge case „Status per Direkt-DB-Edit gesetzt" ab; Phase-2-Test (HLPR-05 Cascade-pragma) muss nicht angefasst werden.
- **D-15:** Reihenfolge im close_assembly-tx: (1) Status-Check (`Open` zwingend), (2) Status-Update + audited_update! (`assembly.close`), (3) `list_session_ids_for_assembly` + Loop `delete_session`, (4) Commit. Falls eine Session-DELETE fehlschlägt: Transaction-Rollback (close_assembly als Ganzes scheitert) — kein partial state. Plan/Researcher detailliert die Error-Strategie.
- **D-16:** Wo die neue DAO-Method gemockt wird im Test (von `close_assembly`-Unit-Tests): bestehende `MockHelperTokenDao` muss um `expect_list_session_ids_for_assembly` erweitert werden; AssemblyServiceImpl bekommt damit eine neue Dependency `HelperTokenDao` (zusätzlich zu Member/Snapshot/AuditLog/Permission/Uuid/TransactionDao).

### Permission-Branch für AuthContext::Helper
- **D-17:** Method `check_assembly_access(assembly_id: Uuid, ctx: Authentication<Ctx>, tx: Transaction)` lebt im **AttendanceServiceImpl**, NICHT im PermissionService. Begründung: AttendanceService hat ohnehin `AssemblyDao`-Dep für `stats` und Status-Lookup; hier wird Assembly genau einmal geladen und das Ergebnis sowohl für Permission als auch für Endpoint-Logik verwendet. PermissionService bleibt frei von Domain-Aggregat-Dependencies.
- **D-18:** Implementierung von `check_assembly_access`:
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
  Genau diese Method wird vor jedem Attendance-Endpoint-Call (list_members, mark_present, mark_absent, stats) aufgerufen.
- **D-19:** Vorstand (Mock/OIDC) → reicht `admin`-Privilege. Keine neue Privilege-Konstante (`attendance.access` o. ä.). Konsistent mit Phase 1 (D-14) und Phase 2 (D-21). Erfüllt ATTN-06 („Helfer-View ist auch für eingeloggte Vorstands-User direkt aufrufbar").
- **D-20:** Vorstand-Post-Close-Edit (ASSY-06): Vorstand kann auch nach `Closed`-Status PUT/DELETE auf `/api/attendance/{aid}/{mid}` aufrufen, weil `check_assembly_access` für admin-Branch KEINEN Status-Check macht. Helfer-Branch macht den Status-Check (Status=Open zwingend), bekommt also nach GV-Schluss 403. **Kein status-write-path** im AttendanceService → GV bleibt `Closed`, ASSY-06-SC#9 automatisch erfüllt.

### Toggle-Endpoint-Design + Service-Layout
- **D-21:** Endpoints REST-konform, getrennt nach Verb:
  - `GET /api/attendance/{assembly_id}/members?q={substring}` — reduzierte Mitgliederliste (4 Felder), optionaler Substring-Filter auf Name oder Mitgliedsnummer; ATTN-01, ATTN-02, ATTN-06.
  - `PUT /api/attendance/{assembly_id}/{member_id}` — Toggle-On (idempotent, fünfmal 200 OK, ein Eintrag). Body leer.
  - `DELETE /api/attendance/{assembly_id}/{member_id}` — Toggle-Off (idempotent, Soft-Delete-Flip, 200 OK auch wenn nichts existiert).
  - `GET /api/assembly/{assembly_id}/stats` — Live-Counter `{present: u64, total: u64}`. Lebt unter `/api/assembly/...`, nicht `/api/attendance/...`, weil semantisch ein Assembly-Aspekt — aber Implementation in AttendanceService (D-23).
- **D-22:** Neuer Service `AttendanceService` mit Methods: `list_members(assembly_id, search, ctx, tx)`, `mark_present(aid, mid, ctx, tx)`, `mark_absent(aid, mid, ctx, tx)`, `stats(aid, ctx, tx)`. Liegt in `genossi_service/src/attendance.rs` (Trait) + `genossi_service_impl/src/attendance.rs` (Impl). Folgt dem Genossi-Pattern (MemberService getrennt von MemberActionService).
- **D-23:** AttendanceServiceImpl-Dependencies (via `gen_service_impl!`): `AttendanceDao` (neu), `AssemblyDao` (für Status-Lookup in check_assembly_access + Stats), `MemberDao` (für reduzierten Member-View + Search), `AssemblyMemberSnapshotDao` (für `total` in stats), `PermissionService` (für admin-Fallback in check_assembly_access), `TransactionDao`. **Kein UuidService**, **kein AuditLogDao** (D-08 — kein Audit).
- **D-24:** Reduzierter Member-View — eigenes TO `AttendanceMemberTO` mit nur `member_number`, `first_name`, `last_name`, `salutation`, `title` plus `is_present: bool`. NICHT `MemberTO` mit serde-skip (ROADMAP-Hard-Constraint Phase 3, expliziter Verbots). Konvertierung: `From<&MemberEntity>` für Stamm-Felder + `is_present` aus separatem AttendanceDao-Lookup.
- **D-25:** Substring-Search wird im DAO ausgeführt (`AttendanceDao::list_members_for_assembly(assembly_id, search: Option<&str>, tx)`) statt im Service-Memory-Filter. Begründung: SQL `WHERE` mit `LIKE '%?%' OR member_number LIKE '%?%'` ist effizienter als alle Members laden + filter; auch wenn Genossenschaften meist klein sind, vermeidet das einen schlechten Pattern für späteres Wachstum. Substring-Match auf `last_name || first_name || CAST(member_number AS TEXT)`.
- **D-26:** Permission-Status-Codes für Attendance-Endpoints:
  - **403 Forbidden** — Helfer mit falscher `assembly_id` ODER Helfer auf geschlossener GV (D-18-Branch); nicht-admin User auf admin-only Endpoints
  - **404 Not Found** — `assembly_id` existiert nicht; `member_id` ist nicht im snapshot der Assembly (Helfer darf nur snapshot-Mitglieder markieren)
  - **200 OK** — Erfolgs-Toggle (PUT und DELETE beide 200, kein 204 — konsistent mit Genossi-Konvention)
- **D-27:** `mark_present`/`mark_absent` müssen prüfen, dass `member_id` im `assembly_member_snapshot` der GV ist (vermeidet Anwesenheit für nicht-Mitglieder). Falls nicht: 404. Lookup einmal in der Methode, vor dem UPSERT.

### Naming
- **D-28:** Code-Identifier durchgängig englisch, konsistent mit Phase 1/2: `Attendance`, `AttendanceEntity`, `AttendanceDao`, `AttendanceService`, `AttendanceServiceImpl`, `AttendanceMemberTO`, `AttendanceStatsTO`. Tabelle `attendance`. Endpoints englisch (D-21).

### Claude's Discretion
- **UNIQUE-Index-WHERE-Clause** (D-04): plain `UNIQUE(assembly_id, member_id)` vs. partial `UNIQUE(...) WHERE deleted IS NULL`. SQLite-UPSERT braucht plain UNIQUE für `ON CONFLICT(assembly_id, member_id)`-Targeting. Plan/Researcher finalisiert.
- **FK-ON-DELETE-Verhalten** für `attendance.assembly_id` und `attendance.member_id`: vermutlich `RESTRICT` (Soft-Delete ist Norm), aber Plan wägt mit Cleanup-Job-Implikationen ab.
- **Search-Min-Length / Max-Results**: kein Minimum; kein Pagination in Phase 3 (D-25). Falls Plan-Researcher Performance-Sorgen findet (z. B. >500 Mitglieder), nachziehen.
- **Stats-Polling-Rate-Limit**: Frontend Phase 4 polled `~5s` (ROADMAP Phase 4 SC#3); aktuelle `tower_governor`-Konfig vermutlich okay. Plan entscheidet, ob spezifisches Per-IP-Limit für Stats-Endpoint.
- **Reihenfolge der Filter im Substring-LIKE-Query** (D-25): `last_name || first_name || member_number` vs. exakter Match auf member_number first → LIKE second. Plan wählt aus Performance-Sicht.
- **Test-Strategie für UPSERT-Race** (D-05): Race-Test mit zwei parallel `tokio::join!`-PUT-Requests auf demselben (aid, mid) — exakt eine Row, keine Errors (SYNC-02-Verifikation). Plan/E2E-Researcher detailliert.
- **`stats`-Permission-Branch für Helper** (D-21): Helfer dürfen Live-Counter sehen (wird in Phase 4 für Helfer-UI ggf. gebraucht). check_assembly_access greift identisch — kein Sondervertrag.
- **Error-Strategie bei `delete_session`-Fehler in Cascade** (D-15): Rollback der gesamten close_assembly-tx vs. Continue-on-Error mit Log. Plan entscheidet — Rollback ist defensiv besser, aber falls eine schon-tote Session 404 zurückgibt, würde das ein erfolgreiches close_assembly blockieren.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level Locking-Dokumente
- `.planning/PROJECT.md` — Core Value, Active Requirements (Helfer sehen reduzierte Liste, idempotenter Toggle, Live-Counter, Cascade-Invalidation, Vorstand-Post-Close-Korrektur), Constraints (neue GV-Entitäten benötigen Audit NICHT), Key Decisions (Anwesenheit als Join-Tabelle, ohne Audit, Sync nur per Refresh, Helfer-View für Vorstand).
- `.planning/REQUIREMENTS.md` §Anwesenheits-Erfassung — ATTN-01..ATTN-06 als Abnahme-Kriterien dieser Phase. §Sync — SYNC-02 (Idempotenz schluckt Doppel-Markierung). §Assembly — ASSY-04 (Live-Counter), ASSY-06 (Vorstand-Post-Close-Edit) auch in Phase 3.
- `.planning/ROADMAP.md` §Phase 3 — Goal, 9 Success Criteria (inkl. SC#7 Live-Counter+SYNC-02-Race-Test, SC#8 Cascade-Invalidation, SC#9 Post-Close-Edit), Hard Constraints Phase 3: eigenes `AttendanceMemberTO` mit nur 4 Feldern (NICHT `MemberTO` mit serde-skip), idempotenter PUT, `UNIQUE(assembly_id, member_id) WHERE deleted IS NULL`, Cascade-Invalidation in `close_assembly`.
- `.planning/STATE.md` §Accumulated Context — Key-Decisions-Tabelle und Skills-to-Apply (`audited_*!`-Macros NICHT für Anwesenheit; Component-First für späteres Frontend; Soft-Delete als Standard).
- `.planning/phases/01-assembly-aggregat-audit-hardening/01-CONTEXT.md` — Phase-1-Decisions, insbesondere `assembly`-Tabelle als FK-Ziel (D-05), Status-Werte englisch (D-06/D-17), `assembly_member_snapshot`-Schema als Vorbild für leichtgewichtige Join-Tabelle (D-01..D-04), `close_assembly`-Methode (D-09 — wird in Phase 3 erweitert), Process-String-Punkt-Notation (D-11).
- `.planning/phases/02-helfer-token-session-authcontext-helper/02-CONTEXT.md` — Phase-2-Decisions, insbesondere `AuthContext::Helper`-Variante (D-14), Claims-JSON-Schema mit `kind="helper"` (D-16), synthetische User-IDs `helper:<token_id>` (D-17), Verify-Status-Check (D-18 — bleibt als Defense-in-Depth in Phase 3), `D-20`-Stub (`PermissionDenied`) der in Phase 3 durch `check_assembly_access` (D-17/D-18 in 03-CONTEXT) ersetzt wird, `helper_token.session_id`-FK (D-01) — Anker für Cascade-Discovery.

### Codebase-Maps (Bestands-Architektur)
- `.planning/codebase/ARCHITECTURE.md` — Schicht-Struktur, Audit-Datenfluss, Anti-Patterns (Service Creating Own Transaction, Hard Delete Without Audit Trail, Inline RSX in Pages — letzteres Phase-4-relevant).
- `.planning/codebase/STACK.md` — Versionierungen (`tokio` 1.35, `axum` 0.8.3, `sqlx` 0.8 mit SQLite-UPSERT-Support, `uuid` 1.6, `time` 0.3, `tracing` 0.1, `tower_governor` 0.6).
- `.planning/codebase/INTEGRATIONS.md` — Session-Cookie-Pattern (`app_session`), Auth-Middleware-Chain.
- `.planning/codebase/CONVENTIONS.md` — Naming (snake_case files, PascalCase types, `*Impl`-Suffix), Error-Handling-Konvention für `RestError`-Codes.
- `.planning/codebase/TESTING.md` — E2E-Pattern mit `start_test_server()`, Mockall-Pattern.

### Bestehende Patterns als Vorlage
- `genossi_dao/src/assembly_member_snapshot.rs` — leichtgewichtige Join-Tabelle ohne Aggregat-Overhead; **Schema-Vorbild für `attendance.rs`** (D-02). Ergänzungen für Phase 3: `deleted`-Spalte, `marked_at`/`marked_by_user_id`-Spalten, UPSERT-Method.
- `genossi_dao/src/assembly.rs` — vollwertiges Aggregat als KONTRAST (zeigt was Phase 3 NICHT braucht: id/version, audited).
- `genossi_dao/src/helper_token.rs` — `session_id`-Feld (Phase 2 D-01) ist der Cascade-Anker (D-12). Phase 3 fügt `list_session_ids_for_assembly`-Method hinzu.
- `genossi_dao/src/member.rs:76-80` — `MemberEntity`-Felder (`member_number`, `first_name`, `last_name`, `salutation`, `title`); reduzierte Auswahl wandert in `AttendanceMemberTO` (D-24).
- `genossi_dao/src/permission.rs:90,93` — `PermissionDao::delete_session(session_id)` (Cascade-Schritt D-13); `delete_sessions_for_user(user_id)` (alternative die NICHT verwendet wird).
- `genossi_service/src/auth_types.rs:108-111` — `AuthContext::Helper { session_id, assembly_id }` (Phase 2 D-14); Phase 3 nutzt `assembly_id` für `check_assembly_access` (D-17).
- `genossi_service_impl/src/permission.rs:28-48` — `check_permission` als Vorlage; **NICHT erweitern** (D-17). `check_assembly_access` lebt stattdessen im AttendanceServiceImpl.
- `genossi_service_impl/src/session.rs:189-219` — `verify_user_session` mit Helper-Claims-Discriminator + Status-Check (Phase 2 D-18, bleibt Defense-in-Depth — D-14 in 03-CONTEXT).
- `genossi_service_impl/src/assembly.rs:254-304` — `close_assembly` Methode; Phase 3 erweitert um Cascade nach `audited_update!`-Aufruf (D-11..D-15).
- `genossi_service_impl/src/macros.rs` — `gen_service_impl!`-Macro für AttendanceServiceImpl-Skeleton (D-22).
- `genossi_rest/src/assembly.rs` — Vorlage für Axum-Handler-Stil; AttendanceHandler folgen demselben Muster (D-21).
- `genossi_bin/src/lib.rs` — DI-Wiring (`RestStateImpl`); neue `AttendanceServiceImpl` landet hier mit Deps gemäß D-23.
- `genossi_bin/tests/e2e_tests.rs` — E2E-Pattern; Race-Test für SYNC-02 (D-05) und Cascade-Test für SC#8 (D-11) bauen darauf auf.
- `migrations/sqlite/20260413000000_create_application_table.sql` — Vorlage für Migration-Struktur eines neuen Aggregats (D-10).

### CLAUDE.md (Projekt-Konventionen)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Architecture Overview — Layer-Struktur, ISO8601-Datetime-Handling, Soft-Delete-Pattern (Phase 3 ist erster echter Use-Case).
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Audit Log System — explizit dokumentiert für Member/MemberAction/MemberDocument/Application; Phase-3-Attendance-Aggregat ist davon **ausgenommen** (D-08).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`gen_service_impl!`-Macro** (`genossi_service_impl/src/macros.rs`) — generiert `AttendanceServiceImpl`-Struct mit Deps-Skeleton (D-22, D-23).
- **`assembly_member_snapshot`-Schema** (`migrations/sqlite/*_create_assembly_member_snapshot_table.sql`) — Migration-Vorbild für Phase 3; Phase 3 erweitert um `deleted`/`marked_at`/`marked_by_user_id` (D-01).
- **SQLite-UPSERT** (`sqlx 0.8` + SQLite ≥ 3.24) — `INSERT ... ON CONFLICT DO UPDATE` ist im Projekt schon etabliert (z. B. `permission.rs::ensure_user_exists`); D-05 wendet das auf attendance an.
- **`MemberEntity`-Felder** (`genossi_dao/src/member.rs:76-80`) — `member_number`/`first_name`/`last_name`/`salutation`/`title` werden 1:1 in `AttendanceMemberTO` gespiegelt; keine neue Datenmodellierung (D-24).
- **`PermissionDao::delete_session(session_id, tx)`** (`genossi_dao/src/permission.rs:90`) — Cascade-Building-Block (D-13); kein neuer DAO-Code in PermissionDao.
- **`HelperTokenDao` + `helper_token.session_id`** (Phase 2, `genossi_dao/src/helper_token.rs`) — `session_id`-Spalte ist der Cascade-Anker; Phase 3 fügt `list_session_ids_for_assembly`-Method hinzu (D-12).
- **`AuthContext::Helper { session_id, assembly_id }`** (`genossi_service/src/auth_types.rs:108`) — `assembly_id` wird in `check_assembly_access` mit dem Endpoint-Parameter verglichen (D-18).
- **`AssemblyMemberSnapshotDao::count_by_assembly_id(id, tx)`** — liefert `total` für `stats`-Endpoint (D-21); bereits in Phase 1 implementiert.
- **E2E-Test-Pattern** (`genossi_bin/tests/e2e_tests.rs`) — Race-Test mit `tokio::join!` (Phase-2-HLPR-04-Vorlage) wird für SYNC-02-Race und Cascade-Test wiederverwendet.
- **`tower_governor` 0.6** (`Cargo.toml`) — Rate-Limiting-Middleware für Stats-Polling-Schutz, falls nötig (Claude's Discretion).

### Established Patterns
- **Trait-basierte DI** — `AttendanceDao` als Trait, `AttendanceDaoImpl` als SQLite-Impl; AttendanceService generic über `AttendanceServiceDeps` (D-22).
- **Optional-Transaction-Argument** im Service-Layer — Caller entscheidet Transaction-Scope; Service nutzt `transaction_dao.use_transaction(tx)`.
- **Soft-Delete-Konvention** — `deleted: Option<PrimitiveDateTime>`; alle Queries filtern `WHERE deleted IS NULL`. Phase 3 ist erster Use-Case der die Spalte aktiv nutzt (D-09).
- **TO/Entity-Trennung** — `AttendanceEntity` für DAO; `AttendanceMemberTO`, `AttendanceStatsTO` für REST mit ISO8601-serde wo nötig (D-24).
- **`error_handler()`-Wrapper** in REST-Handlern (`genossi_rest/src/member.rs`-Vorbild) — `ServiceError` → `RestError` → HTTP-Status (D-26).
- **`#[instrument(skip(rest_state))]`** auf REST-Handlern für Tracing-Spans.

### Integration Points
- `genossi_bin/src/lib.rs` `RestStateImpl::new()` — neue `AttendanceServiceImpl` wird hier instanziiert. Benötigt: `AttendanceDao` (neu), `AssemblyDao`, `MemberDao`, `AssemblyMemberSnapshotDao`, `PermissionService`, `TransactionDao` — siehe D-23.
- `genossi_rest/src/lib.rs` Router — neue Route-Gruppen `/api/attendance/{aid}/members`, `/api/attendance/{aid}/{mid}` (PUT+DELETE), `/api/assembly/{aid}/stats` (GET) mit Auth-Middleware. Utoipa-Schemas für `AttendanceMemberTO`, `AttendanceStatsTO`, optionalen `?q=`-Query-Param.
- `genossi_service_impl/src/assembly.rs:254-304` — `close_assembly` wird erweitert um Cascade-Loop nach dem `audited_update!`-Block (D-11..D-15). Neue Dependency `HelperTokenDao` für AssemblyServiceImpl.
- `genossi_service/src/assembly.rs` — `AssemblyServiceDeps` bekommt `HelperTokenDao`-Eintrag (für die neue `list_session_ids_for_assembly`-Method); konfliktfrei mit Phase 1 Tests, weil neue Trait-Methode keine bestehenden Tests bricht.
- `migrations/sqlite/` — eine neue Migration-File (`attendance`-Tabelle); auto-Run beim Server-Start via SQLx (D-10).

</code_context>

<specifics>
## Specific Ideas

- **`AttendanceServiceImpl::check_assembly_access` als zentrale Permission-Funnel**: ALLE vier Endpoints (list_members, mark_present, mark_absent, stats) rufen genau diese eine Method als ersten Schritt. Verhindert „neuer Endpoint vergisst Status-Check"-Bug. Die Method lädt assembly EINMAL und gibt sie ggf. zurück (oder cached sie im Method-Scope), sodass nachfolgende Endpoint-Logik ohne Re-Load arbeitet.
- **Cascade-Test-Sequenz für SC#8**: GV anlegen → öffnen → Helfer-Token erzeugen → Helfer-Redeem → Anwesenheits-PUT (verifiziert Helfer-Session funktioniert) → close_assembly → erneut Anwesenheits-PUT → erwartet 401. Zusätzlich verify dass `helper_token.session_id`-Einträge der Assembly nun auf invalide Sessions zeigen (oder via FK-Cleanup auf NULL gesetzt sind).
- **Race-Test-Sequenz für SYNC-02**: zwei `tokio::join!`-PUT-Requests von zwei verschiedenen Helfer-Sessions auf demselben (assembly_id, member_id) → exakt eine Row in attendance, beide bekommen 200 OK, kein Doppel-Eintrag, kein Error. Test verifiziert gegen die DB den COUNT (== 1).
- **Hash-Chain-Stabilitäts-Test für ATTN-05**: nach 100 Toggles `GET /api/audit/verify` aufrufen → leere Mismatch-Liste UND Anzahl der Audit-Einträge unverändert vor/nach dem Toggle-Burst (kein attendance-bezogener Eintrag in der Hash-Chain).
- **Reduzierter-Member-View-PII-Test für ATTN-01**: GET `/api/attendance/{aid}/members`-Response auf JSON-Ebene auf Felder `iban`, `bank_account`, `street`, `house_number`, `postal_code`, `city`, `email`, `join_date`, `exit_date`, `comment` etc. durchsuchen → keiner darf vorhanden sein. Statisch via Schema-Assertion oder dynamisch via JSON-Field-Iteration.
- **Vorstand-Post-Close-Edit-Test für ASSY-06**: GV → öffnen → schließen → admin-Context PUT/DELETE auf attendance → Status bleibt `Closed`, Eintrag wird hinzugefügt/entfernt, GV-Stats aktualisieren sich.

</specifics>

<deferred>
## Deferred Ideas

### Phase 4 (Frontend)
- **Anwesenheits-UI-Components**: `AttendanceRow`, `AttendanceSearch`, `LiveCounter` — wiederverwendbar zwischen Helfer- und Vorstand-Page (Phase 4 Hard Constraint, Component-First).
- **Live-Counter-Polling**: ~5s Polling von `/api/assembly/{id}/stats`. Backend-Vertrag (D-21) steht fertig.
- **Connection-Banner**: Frontend zeigt bei Verbindungsverlust an; Anwesenheits-Markierungen erst nach 200-OK-Response visuell bestätigen (kein Optimistic-UI).
- **Manual-Code-Eingabe-UI** (HLPR-03): Frontend-Pfad `/helper`, baut auf Phase-2-Redeem-Endpoint auf.

### Phase 5 (Operations)
- **Stats-Polling-Verhalten unter realer GV-Last**: Generalprobe verifiziert ob `~5s`-Polling beim realen Vereinsheim-WiFi okay ist; ggf. Rate-Limit anpassen.

### Spätere Phasen / Out of Scope
- **Bulk-Mark-Endpoint** für mehrere Mitglieder gleichzeitig — nicht in REQUIREMENTS gefordert; Helfer markieren einzeln nach Sichtkontakt (Out-of-Scope-Konsistenz mit „Self-Check-in für Mitglieder per persönlichem QR-Code" in REQUIREMENTS §Out of Scope).
- **Pagination für Mitgliederliste** — Genossenschaften sind klein (typisch <500 Mitglieder); Substring-Suche reicht. Falls Performance-Issue in Phase 5 auftaucht, nachziehen.
- **Eigene `attendance.access`-Privilege** statt admin — nur falls Vorstands-Rolle sich später aufspaltet (Schriftführer ohne admin); Phase 3 nutzt admin (D-19).
- **Audit-Log für Vorstand-Post-Close-Edit** — explizit ausgeschlossen in dieser Diskussion (D-08); falls Vorstand später nachfragt „wer hat nachgemeldet?", kann eine Erweiterung mit `audited_*!`-Macros + Process `attendance.post_close_edit` angefügt werden.
- **`unmarked_by`-Feld** für „wer hat ausgetragen" — explizit verworfen (D-07); falls Vorstand später nachfragt, ist das eine reine Migrations-Ergänzung.
- **Stats für geschlossene GVs (Statistik-View)** — `stats`-Endpoint funktioniert auch nach `Closed` (kein Status-Check für admin), aber UI/Reporting für historische GVs ist v2 (REQUIREMENTS §v2 EXPO-01/02).
- **Pro-IP-Rate-Limit speziell für Stats-Polling** — Plan-Discretion; aktuelle `tower_governor`-Konfig vermutlich okay.

### Reviewed Todos (not folded)
None — discussion stayed within phase scope.

</deferred>

---

*Phase: 3-Attendance-Aggregat + Cascade-Invalidation*
*Context gathered: 2026-05-03*
