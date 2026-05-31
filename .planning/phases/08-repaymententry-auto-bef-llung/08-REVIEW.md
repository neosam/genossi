---
phase: 08-repaymententry-auto-bef-llung
reviewed: 2026-05-31T10:30:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - genossi_service_impl/src/repayment_entry.rs
  - genossi_service_impl/src/repayment_phase.rs
  - genossi_rest/src/repayment_entry.rs
  - genossi_rest_types/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
findings:
  blocker: 0
  warning: 5
  info: 3
  total: 8
status: issues_found
---

# Phase 08 — Code-Review-Bericht (Gap-Closure: CR-01 / CR-02 / IN-04)

**Reviewed:** 2026-05-31T10:30:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Der Gap-Closure-Patch adressiert die drei aus dem vorherigen Review/Verification stammenden Lücken (CR-01 stale-version, CR-02 404/409-Trennung, IN-04 fehlende Regressionstests) prinzipiell korrekt und gut dokumentiert. Re-Read-Pattern, Sequence-basierte Mockall-Tests und 5 neue E2E-Tests sind solide umgesetzt. Audit-Disziplin (alle Writes via `audited_*!`) wird eingehalten.

**Aber:** Die Re-Read-Fehlerbehandlung selbst widerspricht dem expliziten Plan. Der Source-Kommentar in `batch_toggle_status` (Z. 472-475) sagt klar: "EntityNotFound here is an internal consistency error, not a user-facing race." Der Code mappt es trotzdem auf `ServiceError::EntityNotFound(*entry_id)` → HTTP 404. Das schlägt durch alle sechs Re-Read-Sites (`update_repayment_entry`, `batch_toggle_status`, `create_repayment_phase`, `update_repayment_phase`, `open_repayment_phase`, `close_repayment_phase`) durch. Damit bekommt der Client einen 404 für einen Entry, den der Service in derselben Tx soeben geschrieben hat — das verdeckt einen echten Bug-Class (z.B. DAO-Filter-Regression, Tx-Isolation-Issue, korrupte UUID-Generation) hinter dem 404 und macht Debugging deutlich schwerer. Das ist BLOCKER.

Daneben WARNINGs zu Test-Tiefe (D-08-All-or-Nothing-Daten-Rollback wird im neuen E2E-Test nur über HTTP-Status verifiziert, nicht über Daten-Zustand danach), zur Version-Audit-Lücke (existierendes Pattern, durch Re-Read aber neu sichtbar), zu inkonsistenten Error-Mappings im selben Aggregat, und einer Doku-↔-Impl-Inkonsistenz bei `create_repayment_entry`.

## Blocker Issues

### BL-01: Re-Read mappt internen Konsistenzfehler auf 404 — widerspricht eigenem Kommentar

**Status: RESOLVED (2026-05-31)** — Behoben durch Commits `bc022bf` (Code-Fix) + `87539bb` (Negativtests). Alle sechs Re-Read-Sites mappen die strukturell-unmögliche `None`-Verzweigung jetzt auf `ServiceError::InternalError` (→ HTTP 500) statt `EntityNotFound` (→ HTTP 404). Zwei neue Unit-Tests (`test_update_repayment_entry_rereads_none_yields_internal_error`, `test_update_repayment_phase_rereads_none_yields_internal_error`) verifizieren das Verhalten via mockall-Sequence (Re-Read → `Ok(None)`). Alle 23/26 lib-Tests grün.

**Files:**
- `genossi_service_impl/src/repayment_entry.rs:275, 480`
- `genossi_service_impl/src/repayment_phase.rs:148, 239, 387, 499`

**Issue:**
An sechs Stellen wird das Re-Read-Ergebnis mit `.ok_or(ServiceError::EntityNotFound(id))?` versehen. Über `From<ServiceError> for RestError` (`genossi_rest/src/lib.rs:102`) wird das zu `RestError::NotFound` → HTTP 404 "Not found".

Der Source-Kommentar bei `batch_toggle_status` (`repayment_entry.rs:472-475`) sagt explizit:

> "Re-Read runs in the same transaction — soft-delete in the same Tx is impossible (single-writer per service method), so EntityNotFound here is an internal consistency error, not a user-facing race."

Genau das wird im Code aber nicht umgesetzt. Stattdessen wird der Client mit einem 404 belogen für einen Entry, den der Service-Layer in derselben Tx erfolgreich erstellt/aktualisiert hat. Mögliche reale Ursachen für `None` auf der Re-Read-Seite:
- Tx-Isolation-Regression im SQLx/SQLite-Layer
- Bug im DAO-Filter `deleted IS NULL` (z.B. Race wenn ein Cleanup-Job `deleted` in einer Parallel-Tx schreibt)
- Korruption der id zwischen `audited_*!`-Argument und Re-Read-Argument
- Ein zukünftiger Refactor, der `audited_update!` async-detached macht (theoretisch)

In allen Fällen ist die korrekte Reaktion ein 500er mit `InternalError`, NICHT ein 404, der dem Frontend signalisiert "die Ressource gibt's nicht mehr — User soll die Liste neu laden". Bei `batch_toggle_status` ist es besonders giftig: der Client sieht jetzt zwei verschiedene Bedeutungen von 404 (echtes Stale-ID-Szenario vs. interne Inkonsistenz), die nicht unterscheidbar sind — genau der Trade-off, der mit CR-02 explizit beseitigt werden sollte.

**Fix:**
```rust
// Pattern für alle sechs Re-Read-Stellen:
let refreshed = self
    .repayment_entry_dao  // oder repayment_phase_dao
    .find_by_id(id, tx.clone())
    .await?
    .ok_or_else(|| {
        ServiceError::InternalError(Arc::from(format!(
            "Re-Read after audited_update! returned None for id={} — \
             internal consistency error (same-tx invariant violated)",
            id
        )))
    })?;
```

Zusätzlich: das `genossi_service::ServiceError::InternalError(_)`-Match in `From<ServiceError> for RestError` (`genossi_rest/src/lib.rs:112`) fällt heute in den `_ => RestError::InternalError(...)`-Catch-all. Das ist ok — es resultiert korrekterweise in HTTP 500. Es muss kein Mapping-Code geändert werden.

**Test-Implikation:** Die zwei Unit-Tests `test_update_repayment_entry_rereads_after_audited_update_returns_new_version` (`repayment_entry.rs:1827`) und `test_batch_toggle_status_rereads_each_entry_returns_new_versions` (`repayment_entry.rs:1913`) sowie die Phase-Tests verifizieren NUR den Happy-Path. Es fehlt ein Negativtest "Re-Read returns None → 500, nicht 404" — bitte mit Fix mitliefern. Mockall-Setup: nach dem `expect_update` einfach einen `expect_find_by_id().returning(|_, _| Ok(None))` setzen und auf `ServiceError::InternalError(_)` matchen.

## Warnings

### WR-01: E2E-Test für CR-02 prüft nur HTTP-Status, nicht D-08-Rollback

**File:** `genossi_bin/tests/e2e_tests.rs:11908-11942`

**Issue:**
`test_batch_toggle_with_unknown_entry_id_returns_404` schickt `[real_id, fake_id]` — der erste Entry wird verarbeitet (find_by_id OK + audited_update! läuft inkl. Audit-Log-Insert in derselben Tx), dann fehlt der zweite. Die D-08-Garantie "all-or-nothing" greift via Tx-Drop ohne commit. Der Test verifiziert NUR den HTTP-Status (404), aber NICHT, dass:

1. Der erste Entry nach dem Rollback noch Status `Open` hat (nicht `Contacted`).
2. Im Audit-Log KEIN Eintrag für den ersten Entry mit `process=repayment-entry.batch-toggle` steht (Rollback hat die Audit-Inserts ebenfalls verworfen).

Beides sind die eigentlichen Garantien, die D-08 schützen sollte. Ohne diese Checks würde eine zukünftige Auto-Commit-Regression (z.B. jemand baut "commit after each entry" ein für vermeintliche "Robustheit") unentdeckt bleiben.

**Fix:**
Nach dem 404-Assert eine GET-Roundtrip + Audit-Verify einbauen:

```rust
// Nach dem 404-Assert:
let after: RepaymentEntryTO = client
    .get(server.url(&format!("/api/repayment-entry/{}", real_id)))
    .send().await.unwrap()
    .json().await.unwrap();
assert!(
    matches!(after.status, RepaymentEntryStatusTO::Open),
    "D-08 Rollback: real_id must remain Open after partial-batch failure, got {:?}",
    after.status
);
// Audit-Verify: keine repayment-entry.batch-toggle-Einträge für real_id
let audit_resp = client
    .get(server.url(&format!("/api/audit/RepaymentEntry/{}", real_id)))
    .send().await.unwrap();
let entries: Vec<AuditLogEntryTO> = audit_resp.json().await.unwrap();
assert!(
    !entries.iter().any(|e| e.process == "repayment-entry.batch-toggle"),
    "D-08 Rollback: no batch-toggle audit entries must exist for real_id"
);
```

### WR-02: Re-Read-Argument ist Service-übergebene `entity.id`, nicht DB-Roundtrip-bestätigte ID

**File:** `genossi_service_impl/src/repayment_phase.rs:144-148`

**Issue:**
In `create_repayment_phase` macht der Re-Read `self.repayment_phase_dao.find_by_id(entity.id, ...)`. `entity.id` stammt aus `self.uuid_service.new_v4()` (Zeile 119). Sollte der DAO-Layer jemals die `entity.id` rewriten (heute tut er das nicht, ist aber ein zukünftiger Risk-Vector — z.B. eine `RETURNING id`-Migration), würde der Re-Read fehlschlagen und (gemäß heutigem Code) einen 404 produzieren, dabei wurde das Entity korrekt persistiert.

Auch bei `update_repayment_phase`, `open_repayment_phase`, `close_repayment_phase` wird der Re-Read mit dem `id`-Path-Parameter gemacht — das ist robust, weil dieser nicht vom DAO-Layer manipuliert wird. Im `create`-Pfad ist es subtiler.

**Fix:**
Mit BL-01-Fix mit-erschlagen: durch das Mappen auf `InternalError` wird der Service-Bug visible, und der Komentar dokumentiert das als Invariante.

Optional zusätzlich: in `create_repayment_phase` ein `debug_assert_eq!(refreshed.id, entity.id, "DAO must not rewrite id on insert")` — defensive Programmierung gegen zukünftige DAO-Refactors.

### WR-03: Audit-Diff übersieht `version`-Änderung, weil Service alte `version` an `audited_update!` übergibt

**Files:**
- `genossi_service_impl/src/repayment_entry.rs:255-263`
- `genossi_service_impl/src/repayment_phase.rs:221-229, 287-295, 483-491`
- `genossi_service_impl/src/audit_macros.rs:42-80` (existing, nicht Teil dieses Gap-Closure-Patches, aber relevant)

**Issue:**
`audited_update!` baut den Audit-Diff zwischen `old` (frisch geladen via `find_by_id` im Macro) und `$new_entity` (vom Service übergeben). Im Service-Code:

```rust
let mut entity = self.repayment_entry_dao.find_by_id(id, tx.clone()).await?...;
// entity.version bleibt unverändert!
crate::audited_update!(self, self.repayment_entry_dao, id, &entity, ...);
```

Da der Service `entity.version` NICHT vor dem Macro auf den neuen Wert setzt (er weiß ihn ja nicht — der DAO generiert ihn intern in `Uuid::new_v4()` in `repayment_entry_dao_impl_sqlite.rs:136`), ist im Audit-Diff `old.version == new.version`, und die Version-Änderung wird NICHT geloggt. Das verstößt gegen die Audit-Trail-Intention "jede Feld-Änderung erscheint im Log".

Das ist KEIN neuer Bug des Gap-Closure-Patches (das Pattern war schon in Phase 7), aber durch die neuen Re-Read-Pattern wird es relevanter: der Frontend-Client kriegt jetzt die neue `version` direkt zurück, aber im Audit-Log fehlt der Beweis, wann genau diese version gesetzt wurde. Wenn das absichtlich so ist (weil version intern ist), bitte als ADR/Kommentar festhalten.

**Fix:**
Variante A — `Auditable::audit_fields()` für `RepaymentEntry`/`RepaymentPhase` so anpassen, dass `version` ausgeschlossen ist (saubere Lösung, dokumentiert: "version is internal, not auditable").

Variante B — Im Service nach dem Re-Read NOCHMAL die `version` in das Audit-Log explizit schreiben. Verkompliziert die Service-Implementation und Re-Read-Loop deutlich.

Variante C — Status Quo + ADR/Kommentar im `audited_update!`-Macro, der das Verhalten dokumentiert.

Empfehlung: Variante A oder C. Bitte NICHT B.

### WR-04: `update_repayment_entry` liefert 404 mit `member_id`, wenn Member soft-deleted ist

**File:** `genossi_service_impl/src/repayment_entry.rs:228-232`

**Issue:**
Wenn der Client `share_count_to_pay_out` editiert und der zugehörige Member zwischenzeitlich soft-deleted wurde:

```rust
let member = self.member_dao.find_by_id(entity.member_id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(entity.member_id))?;
```

→ HTTP 404. Aber: der Client hat eine `/api/repayment-entry/{id}` PUT-Request gemacht — der RepaymentEntry existiert, der Member existiert "irgendwo" als soft-deleted. Aus REST-Sicht ist 404 hier irreführend ("die Ressource, die du angefragt hast, existiert nicht" — falsch, sie existiert).

Korrekt wäre 409 Conflict mit Body "cannot edit share_count: associated member was deleted" o.ä. (Domain-Konflikt). Ähnliche Konflikt-Klasse wie die anderen 409er in dieser Methode.

**Fix:**
```rust
let member = self.member_dao.find_by_id(entity.member_id, tx.clone()).await?
    .ok_or_else(|| ServiceError::Conflict(Arc::from(format!(
        "Cannot edit share_count: associated member {} not found or deleted",
        entity.member_id
    ))))?;
```

Bitte Unit-Test mitliefern: `test_update_entry_share_count_with_deleted_member_returns_conflict`.

### WR-05: `create_repayment_entry` mappt fehlende Phase auf 409, fehlenden Member auf 404 — inkonsistent

**File:** `genossi_service_impl/src/repayment_entry.rs:115-137`

**Issue:**
```rust
let phase = self.repayment_phase_dao.find_by_id(submission.phase_id, ...).await?
    .ok_or_else(|| ServiceError::Conflict(Arc::from(format!("Phase {} not found", ...))))?;
// ...
let member = self.member_dao.find_by_id(submission.member_id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(submission.member_id))?;
```

Phase-fehlt → 409 Conflict. Member-fehlt → 404 NotFound. Beide sind aus dem Request-Body (`submission.phase_id`, `submission.member_id`). Aus REST-Sicht wäre eine einheitliche Behandlung (beide 404 oder beide 400 "invalid foreign reference") konsistenter.

In der OpenAPI-Doku (`genossi_rest/src/repayment_entry.rs:78`) steht: `(status = 404, description = "Member or Phase not found")` — der Service liefert die Phase aber als 409. Das ist eine Doku-↔-Impl-Inkonsistenz.

**Fix:**
Einheitliche Behandlung wählen. Empfehlung: beide auf `ServiceError::EntityNotFound` mappen, weil das semantisch korrekt ist (eine vom Client gegebene FK existiert nicht). Phase-Status-Check (`phase.status != Open`) bleibt separat und liefert weiterhin 409. OpenAPI dann an die tatsächliche Impl anpassen.

```rust
let phase = self.repayment_phase_dao.find_by_id(submission.phase_id, ...).await?
    .ok_or(ServiceError::EntityNotFound(submission.phase_id))?;
```

## Info

### IN-01: `BatchFailureResponse.failure_id` ist `String` statt `Uuid`

**File:** `genossi_rest_types/src/lib.rs:1419`

**Issue:**
```rust
pub struct BatchFailureResponse {
    pub failure_index: usize,
    pub failure_id: String,  // <-- Uuid serialisiert als String
    pub failure_reason: String,
}
```

Der Service emittiert `entry_id.to_string()` im JSON-Body. Das macht den Round-trip im Frontend unnötig umständlich (String → Uuid parse), und schwächt das Typsystem. Der Kommentar sagt "string form for JSON-portability", aber `serde_json` serialisiert `Uuid` schon korrekt als string — der Wrapper ist überflüssig.

**Fix:**
```rust
pub failure_id: Uuid,
```

Service-Layer: `"failure_id": entry_id` (statt `.to_string()`). `serde_json::json!` macht den Rest.

### IN-02: `entry_in_status`-Helper im Unit-Test ignoriert `created`-Argument-Pattern

**File:** `genossi_service_impl/src/repayment_entry.rs:849-865`

**Issue:**
`entry_in_status(member_id, phase_id, status, share_count)` — die Signatur wirkt komisch, weil immer derselbe `make_test_datetime()`-Wert genutzt wird. Bei zukünftigen Tests, die zeit-relevant sind (z.B. "created vor Phase-Open"), müsste die Signatur erweitert werden. Heute kein Bug, nur Test-Smell.

**Fix:**
Optional `created: Option<PrimitiveDateTime>`-Argument, default `make_test_datetime()`. Oder einfach Status quo lassen, wenn keine zeitrelevanten Tests anstehen.

### IN-03: OpenAPI-Doku 409-Beschreibung für PUT update_repayment_entry erwähnt Body-Format unklar

**File:** `genossi_rest/src/repayment_entry.rs:191`

**Issue:**
```
(status = 409, description = "Conflict (PaidOut source/target D-05, version mismatch, or share_count edit on PaidOut ENTR-04)"),
```

Erwähnt drei Konfliktklassen, lässt aber offen, ob der Body strukturiertes JSON ist (analog `BatchFailureResponse`) oder ein plain-text "Cannot update: ..." string. Tatsächlich ist's ein plain-text aus `ServiceError::Conflict(Arc::from("..."))`. Für API-Konsumenten wäre ein `(body = String)` oder ein dedizierter `UpdateEntryConflictResponse`-Schema-Type klarer.

**Fix:**
```rust
(status = 409, description = "Conflict — plain-text body explains reason (e.g. 'Cannot update: entry is PaidOut'; 'PaidOut transition must use Phase-9 mark_paid_out endpoint'; 'Version mismatch'; ENTR-04 share_count guard)"),
```

Oder dedizierten Schema-Type einführen und die fünf möglichen Reasons als Enum dokumentieren.

---

## Bestätigung — Was korrekt umgesetzt ist

Damit das Review fair bleibt: die folgenden Punkte sind sauber implementiert:

- **CR-01 Re-Read-Pattern strukturell korrekt** — gleiche Tx, gleiche ID, Aufrufstelle NACH dem `audited_*!`-Macro VOR `commit`. Sechs Stellen, alle mit gut platzierten Source-Kommentaren mit Verweis auf `member.rs:343-348`. Nur die Fehlerbehandlung (BL-01) ist falsch.
- **CR-02 Toctou-Window geschlossen** — `find_by_id` + `audited_update!`-internes `find_by_id` + `dao.update` laufen alle in `tx.clone()`, derselben SQLite-Tx. Keine Race zwischen Pre-Check und Update.
- **Audit-Disziplin gehalten** — keine direkten `repayment_entry_dao.create/update`-Aufrufe ausserhalb der Macros. Grep über die Service-Files bestätigt das.
- **OpenAPI 404 für batch dokumentiert** — `genossi_rest/src/repayment_entry.rs:264` beschreibt 404 inklusive Hinweis "NOT BatchFailureResponse" und dem Aggregat-Konsistenz-Argument. Body ist tatsächlich plain-text "Not found" (`genossi_rest/src/lib.rs:143`) — passt zu GET/PUT/DELETE auf `/{id}`.
- **Mockall-Sequences im Unit-Test-Update** — die `Sequence::new()`-basierten Setups in den drei modifizierten Tests (`test_update_entry_status_open_to_contacted_succeeds`, `_contacted_to_open_succeeds`, `test_update_repayment_phase_share_value_change_in_open_succeeds` etc.) modellieren die korrekte Call-Reihenfolge (pre-load → macro-internal-load → update → re-read).
- **E2E-Tests für CR-01 prüfen den entscheidenden Scenario** — direkt aufeinanderfolgende PUTs mit der version aus der vorigen Response. Das ist genau die Lücke, die IN-04 beanstandet hat.

---

_Reviewed: 2026-05-31T10:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
