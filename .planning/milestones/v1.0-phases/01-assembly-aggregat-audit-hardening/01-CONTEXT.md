# Phase 1: Assembly-Aggregat + Audit-Hardening - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Backend-Aggregat für die Generalversammlungs-Lifecycle: Vorstand kann eine `Assembly` anlegen, öffnen (mit Member-Universe-Snapshot, der das stabile `Y` für den späteren Counter definiert) und schließen. Alle Lifecycle-Operationen (`create`, `open`, `close`) werden über die bestehende Audit-Hashchain protokolliert; ein E2E-Test gegen `GET /api/audit/verify` belegt eine intakte Chain nach diesen Aktionen.

**Phase 1 liefert NICHT:**
- Anwesenheits-Tabelle, Anwesenheits-Endpoints (Phase 3)
- Helfer-Token, Helfer-Sessions (Phase 2)
- Live-Counter-Endpoint `/api/assembly/{id}/stats` (Phase 3, ASSY-04)
- Cascade-Invalidation der Helfer-Sessions in `close_assembly()` — der Hook ist Phase 3 (kein toter Code in Phase 1)
- Frontend (Phase 4)
- Post-close-Korrektur ASSY-06 — wird per Roadmap-Update nach Phase 3 verschoben

</domain>

<decisions>
## Implementation Decisions

### Member-Universe-Snapshot
- **D-01:** Snapshot wird in eigener Tabelle `assembly_member_snapshot` persistiert. Spalten: `assembly_id` (FK auf `assembly.id`), `member_id` (FK auf `member.id`), `captured_at` (PrimitiveDateTime). Keine eigene `id`/`version`/`deleted` — Snapshot-Einträge sind unveränderlich nach dem Öffnen.
- **D-02:** Aktivitäts-Kriterium beim Snapshot-Befüllen folgt der Logik aus `genossi_dao/src/member.rs:182` (`count_active`): `member.deleted IS NULL AND (member.exit_date IS NULL OR member.exit_date > opened_at) AND member.status = MemberStatus::Normal`. Der Researcher muss diese Filter-Logik exakt vom Member-DAO übernehmen, nicht neu definieren.
- **D-03:** Snapshot speichert nur `(assembly_id, member_id)` — keine eingefrorenen Stamm-Felder (Name/Mitgliedsnummer kommen via JOIN auf `member`). Begründung: Soft-Delete-Konvention macht hard-deletes praktisch unmöglich; Umbenennungen sind in Genossenschaften korrekt-gewollt im Protokoll.
- **D-04:** Y-Berechnung erfolgt ad-hoc via `SELECT COUNT(*) FROM assembly_member_snapshot WHERE assembly_id = ?`. Kein gecachter Wert in der `assembly`-Zeile (kein Cache-Drift-Risiko, SQLite-COUNT mit Index ist unkritisch).

### Assembly-Entity & Lifecycle
- **D-05:** Tabelle: `assembly`. Felder: `id` (BLOB UUID), `created` (PrimitiveDateTime), `deleted` (Option), `version` (UUID, optimistic locking), `name` (TEXT, der Titel), `date` (PrimitiveDateTime — Datum **mit** Uhrzeit), `location` (Option<TEXT>), `status` (TEXT mit Enum-Roundtrip), `opened_at` (Option<PrimitiveDateTime>), `closed_at` (Option<PrimitiveDateTime>).
- **D-06:** Status-Enum `AssemblyStatus` mit Varianten `Preparation`, `Open`, `Closed`. Englische Werte (Bruch mit `MemberStatus`-Pattern, das deutsche Werte verwendet — bewusste Entscheidung, in DISCUSSION-LOG dokumentiert). String-Roundtrip in DB wie bei `MemberStatus`.
- **D-07:** Status-Übergänge sind linear und einseitig: `Preparation → Open → Closed`. Direkter Sprung `Preparation → Closed` ist nicht erlaubt; `Closed → *` ist nicht erlaubt (kein Re-Open). Service-Layer enforced; DAO ist agnostisch.
- **D-08:** Beim `open_assembly`-Call: Service setzt `status = Open`, `opened_at = now()`, **und** befüllt im selben Transaction-Scope die `assembly_member_snapshot`-Tabelle mit allen aktiven Mitgliedern. Beides muss atomar geschehen (eine Transaktion, gemeinsamer Commit).
- **D-09:** Beim `close_assembly`-Call: Service setzt `status = Closed`, `closed_at = now()`. Keine Cascade-Invalidation in Phase 1. Phase 3 erweitert die Methode um den HelperSessionService-Cascade.

### Audit-Hardening
- **D-10:** `Assembly` implementiert den `Auditable`-Trait (`genossi_dao/src/auditable.rs`). `entity_type() = "assembly"`. `audit_fields()` enthält die Daten-Felder (name, date, location, status, opened_at, closed_at) — id/version/created/deleted ausgeschlossen wie bei Member/Application.
- **D-11:** Lifecycle-Calls verwenden die bestehenden Macros `audited_create!` (für `create_assembly`) und `audited_update!` (für `open_assembly`, `close_assembly`). Pro Lifecycle-Action wird ein dedizierter **Process-Identifier** als `$process` an das Macro übergeben: `"assembly.create"`, `"assembly.open"`, `"assembly.close"`. Die Hash-Chain-Struktur bleibt identisch zur bestehenden Audit-Logik; der Process-String erlaubt klare Filter im Audit-Log-Endpoint.
- **D-12:** CI-E2E-Test wird als zusätzlicher Test-Fall in `genossi_bin/tests/e2e_tests.rs` (kein neues File). Test-Sequenz: create → open → close gegen Test-Server; danach `GET /api/audit/verify` aufrufen, erwartet leere Mismatch-Liste und mind. 3 Assembly-Lifecycle-Audit-Einträge in der Hash-Chain.

### REST-Layer & Permissions
- **D-13:** Endpoints durchgängig englisch: `POST /api/assembly` (create), `PUT /api/assembly/{id}` (update Stamm-Daten in Status `Preparation`), `POST /api/assembly/{id}/open`, `POST /api/assembly/{id}/close`, `GET /api/assembly`, `GET /api/assembly/{id}` (mit Snapshot-Liste oder zumindest Snapshot-Count).
- **D-14:** Permission-Check: Alle Endpoints fordern die bestehende `admin`-Permission. Keine neue `manage_assemblies`-Permission. Permission-Aufruf folgt dem Member/Application-Pattern via `PermissionService`.
- **D-15:** Migration-Filename: `YYYYMMDDHHMMSS_create_assembly_table.sql` und `YYYYMMDDHHMMSS_create_assembly_member_snapshot_table.sql` (englisch, konsistent mit bestehenden Migrations).

### Naming
- **D-16:** Code-Identifier durchgängig englisch: `Assembly`, `AssemblyEntity`, `AssemblyDao`, `AssemblyService`, `AssemblyServiceImpl`, `AssemblyTO`. Tabelle `assembly`. Endpoint `/api/assembly`.
- **D-17:** Status-Werte englisch: `"Preparation"`, `"Open"`, `"Closed"`. Frontend (Phase 4) übernimmt i18n-Mapping zu deutschen UI-Labels (`Vorbereitung`, `Offen`, `Geschlossen`) — nicht in Phase 1.

### Claude's Discretion
- Cascade-Invalidation-Hook in `close_assembly`: Claude entschied in Abstimmung mit YAGNI-Prinzip, dass Phase 1 *keine* Listener-Trait-Vorbereitung enthält. Phase 3 erweitert die Methode direkt — Genossi-Konvention erlaubt das ohne Architektur-Bruch.
- Index-Strategie auf `assembly_member_snapshot`: Claude wählt während des Plans (vermutlich `(assembly_id)` für COUNT-Queries und ggf. `(assembly_id, member_id)` UNIQUE für Snapshot-Idempotenz).
- ON-DELETE-Verhalten der FKs: Claude wählt während des Plans (vermutlich `RESTRICT` auf `assembly` und `member`, weil Soft-Delete die Norm ist; Hard-Delete soll fehlschlagen).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level Locking-Dokumente
- `.planning/PROJECT.md` — Core Value, Active Requirements (10 Punkte), Constraints (Audit-Pflicht für bestehende Entitäten, neue GV-Entitäten **nicht**), Key Decisions (GV als eigene Entität, Snapshot beim Öffnen, GV-Status final).
- `.planning/REQUIREMENTS.md` §Assembly — ASSY-01..ASSY-07 als Abnahme-Kriterien dieser Phase. **Achtung:** ASSY-06 wird per Roadmap-Update nach Phase 3 verschoben (siehe Deferred Ideas).
- `.planning/ROADMAP.md` §Phase 1 — Goal, Success Criteria, Hard Constraints. Hard Constraint Phase 1: Member-Universe-Snapshot beim Öffnen, Audit-Macros für create/open/close.
- `.planning/STATE.md` §Accumulated Context — Key Decisions Tabelle mit Phase-Zuordnung; Skills/Conventions to Apply.

### Codebase-Maps (Bestands-Architektur)
- `.planning/codebase/ARCHITECTURE.md` — Schicht-Struktur, Audit-Datenfluss, Anti-Patterns (Hard Delete, Service Creating Own Transaction).
- `.planning/codebase/STACK.md` — Versionierungen (Tokio, Axum, SQLx, Utoipa).
- `.planning/codebase/CONVENTIONS.md` — Naming, Error-Handling, Modul-Aufbau (zu lesen vor jedem neuen DAO/Service).
- `.planning/codebase/TESTING.md` — Test-Pattern (Mockall, e2e_tests.rs mit echtem HTTP-Server + In-Memory-SQLite).

### Bestehende Patterns als Vorlage
- `genossi_dao/src/member.rs` — Vorlage für `MemberStatus`-Enum-Pattern (D-06), `count_active`-Filter-Logik (D-02, Zeile 182).
- `genossi_dao/src/application.rs` — Vorlage für ein zweites auditiertes Aggregat mit Status-Lifecycle (`ApplicationStatus`).
- `genossi_dao/src/auditable.rs` — `Auditable`-Trait-Definition (D-10).
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!`, `audited_update!` (D-11).
- `genossi_service_impl/src/audit_log.rs` — Hash-Chain-Berechnung, `compute_entry_hash()`.
- `genossi_service_impl/src/member.rs` — Vorlage für Service-Implementation mit Audit-Macros, Permission-Check, Optional-Transaction-Pattern.
- `genossi_rest/src/member.rs` — Vorlage für Axum-Handler, OpenAPI-Annotation, Error-Handler.
- `genossi_bin/src/lib.rs` — Vorlage für DI-Wiring (RestStateImpl); neue AssemblyService landet hier.
- `genossi_bin/tests/e2e_tests.rs` — E2E-Test-Pattern für CI-Härtung (D-12).
- `migrations/sqlite/20260413000000_create_application_table.sql` — Vorlage für Migration-Struktur eines neuen Aggregats.

### Audit-System-Endpoints (Verify-Check)
- `genossi_rest/src/audit_log.rs` — `GET /api/audit`, `GET /api/audit/{entity_type}/{entity_id}`, `GET /api/audit/verify`. Letzterer ist Ziel des E2E-Tests (D-12).

### CLAUDE.md (Projekt-Konventionen)
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Audit Log System — Schritte zum Hinzufügen von Audit für neue Entitäten (Auditable-Trait → AuditLogDao-Dependency via `gen_service_impl!` → Audit-Macros → Wiring in `genossi_bin/src/lib.rs`).
- `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` §Architecture Overview — Layer-Struktur, ISO8601-Datetime-Handling, Soft-Delete-Pattern.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`gen_service_impl!` Macro** (`genossi_service_impl/src/macros.rs`) — Generiert `*Impl`-Struct mit DI-Feldern und Default-Constructor; `AssemblyServiceImpl` folgt diesem Pattern.
- **`audited_create!` / `audited_update!`** (`genossi_service_impl/src/audit_macros.rs`) — Atomare DAO+Audit-Operations; Phase 1 ruft sie mit Process-String "assembly.create" / "assembly.open" / "assembly.close" auf.
- **`MemberStatus`-Enum-Pattern** (`genossi_dao/src/member.rs:38-69`) — Vorlage für `AssemblyStatus`: Rust-Enum + as_str()/from_str() für DB-Roundtrip. **Abweichung:** englische Werte (D-06).
- **`count_active`-Filter** (`genossi_dao/src/member.rs:182`) — Exakte Logik für Snapshot-Befüllung (D-02). Researcher kopiert das Filter-Predikat in den Snapshot-Befüll-Pfad.
- **Axum-Handler-Pattern** mit `error_handler()`-Wrapper, `#[instrument(skip(rest_state))]`, Utoipa-`#[utoipa::path(...)]` aus `genossi_rest/src/member.rs`.
- **E2E-Test-Pattern** in `genossi_bin/tests/e2e_tests.rs` — `start_test_server()`, `reqwest::Client`, In-Memory-SQLite. Phase 1 erweitert um Assembly-Lifecycle-Test-Fall (D-12).

### Established Patterns
- **Trait-basierte DI** — `AssemblyDao` als Trait, `AssemblyDaoImpl` als SQLite-Impl; Service generic über `Deps: AssemblyServiceDeps`.
- **Optional-Transaction-Argument** im Service-Layer — Caller (REST-Handler) entscheidet Transaktions-Scope; Service nutzt `transaction_dao.use_transaction(tx)`.
- **Soft-Delete-Konvention** — `deleted: Option<PrimitiveDateTime>`; alle Queries filtern `WHERE deleted IS NULL`.
- **Optimistic Locking** — `version: Uuid`; Service prüft Version vor jedem Update; Mismatch → `ConflictError`.
- **TO/Entity-Trennung** — `AssemblyTO` für REST mit ISO8601-serde, `AssemblyEntity` für DAO; `impl From<&AssemblyEntity> for AssemblyTO`.
- **Mock-Auth-Feature** für Tests, `OIDC` für Produktion — bestehende Auth-Middleware liefert `Context` über `Extension<Context>`.

### Integration Points
- `genossi_bin/src/lib.rs` `RestStateImpl::new()` — neue `AssemblyServiceImpl` wird hier instanziiert; benötigt `AuditLogDao`-Dependency (gleiches Pattern wie Member/Application).
- `genossi_rest/src/lib.rs` Router — neue Route-Gruppe `/api/assembly/*` wird mit Tower-Middlewares (Auth, CORS) registriert.
- Utoipa-OpenAPI — `AssemblyTO`, `AssemblyStatus`, neue Handler-Schemas in der OpenAPI-Definition registrieren.
- `migrations/sqlite/` — zwei neue Migration-Files (`assembly`-Tabelle, `assembly_member_snapshot`-Tabelle); auto-Run beim Server-Start via SQLx.

</code_context>

<specifics>
## Specific Ideas

- **Aktivitäts-Logik aus `count_active` exakt übernehmen** — kein Re-Implementation. Wenn der Researcher das Filter-Prädikat sieht, soll er es als gemeinsame Funktion extrahieren oder direkt das `MemberDao` für die Snapshot-Befüllung konsultieren (statt SQL-WHERE im Snapshot-DAO zu duplizieren).
- **Process-Identifier `"assembly.open"` etc. mit Punkt-Notation** — bewusste Wahl, weil das im Audit-Log-Endpoint klar gefiltert werden kann (`?process_prefix=assembly.`).
- **Englische Status-Werte sind Bruch mit MemberStatus** — bei künftigen Audits könnte das auffallen; bewusste User-Entscheidung, zur Konsistenz innerhalb des Assembly-Aggregats. Frontend-Mapping zu deutschen Labels passiert erst in Phase 4.
- **CI-E2E-Test-Sequenz:** Erstellen-Öffnen-Schließen einer Assembly, dann `/api/audit/verify` aufrufen. Der Test muss prüfen: (1) HTTP-200 vom verify-Endpoint, (2) Antwort-Mismatch-Liste leer, (3) Audit-Log enthält Einträge mit den drei Process-Strings.

</specifics>

<deferred>
## Deferred Ideas

### Roadmap-Aktualisierung (zwingend nach diesem Discuss durchführen)
- **ASSY-06 (Post-Close-Anwesenheits-Korrektur) wandert von Phase 1 zu Phase 3.** Begründung: Anwesenheits-Tabelle entsteht ohnehin in Phase 3; Vorstand-Edit-Endpoint dort einzubauen ist saubere Trennung. ROADMAP.md muss aktualisiert werden:
  - Phase 1 Requirements-Liste: ASSY-06 entfernen
  - Phase 1 Success Criteria #5 entfernen (oder als „N/A in Phase 1" markieren)
  - Phase 1 Goal-Text: „nachträglich Anwesenheits-Einträge korrigieren" entfernen
  - Phase 3 Requirements-Liste: ASSY-06 hinzufügen
  - Phase 3 Success Criteria: einen Punkt für Vorstand-Post-Close-Edit-Endpoint hinzufügen
  - REQUIREMENTS.md Traceability-Tabelle: ASSY-06 → Phase 3
  - Coverage Summary: ASSY-Verteilung auf Phasen anpassen
- **Empfohlener Befehl:** `/gsd:phase` zur Bearbeitung der ROADMAP, oder manuelle Edit + Commit.

### Phase 1 SC#4 (Persistenz nach Schluss) Anpassung
- ROADMAP Phase 1 SC#4 nennt aktuell „Snapshot, Anwesenheits-Liste-Slot, Anzahl". Da ASSY-06 wandert, wird SC#4 in der ROADMAP-Aktualisierung auf „Snapshot, Anzahl" reduziert. Die Anwesenheits-Listen-Persistenz wandert in Phase 3 SC.

### Spätere Phasen / Out of Scope
- **Cascade-Invalidation der Helfer-Sessions in `close_assembly`** — Phase 3 (sobald HelperSessionService existiert).
- **Live-Counter-Endpoint `GET /api/assembly/{id}/stats`** — Phase 3 (ASSY-04).
- **Frontend für Assembly-Verwaltung** — Phase 4 (mit Component-First-Prinzip).
- **`manage_assemblies`-Permission feiner Granularität** — wenn Vorstand-Rolle sich später in mehrere sub-Rollen aufspaltet (z. B. Schriftführer ohne admin), kann das nachgezogen werden.

### Reviewed Todos (not folded)
None — discussion stayed within phase scope.

</deferred>

---

*Phase: 1-Assembly-Aggregat + Audit-Hardening*
*Context gathered: 2026-05-02*
