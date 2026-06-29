---
phase: 17-service-rest-uebertrag-cascade
plan: 03
subsystem: rest/membership-adjust
tags: [rust, axum, rest, dto, utoipa, openapi, transfer]
requires:
  - "MembershipAdjustService::transfer_shares (Plan 17-01 Trait, Plan 17-02 Impl)"
  - "MemberTO + MemberActionTO + iso8601_date_required (existing in genossi_rest_types)"
  - "RestStateDef + error_handler + extract_auth_context (existing in genossi_rest)"
provides:
  - "HTTP-Endpoint POST /api/members/{from_id}/transfer-shares"
  - "TransferSharesRequestTO + TransferSharesResponseTO DTOs"
  - "OpenAPI/Swagger-UI-Sichtbarkeit (paths + components erweitert)"
affects:
  - "Plan 17-04 (E2E): kann auf den HTTP-Endpoint zugreifen (reqwest POST mit JSON-Body, JSON-Response-Parsing via TransferSharesResponseTO)"
  - "Phase 18 (Frontend): kann Service-Stub via genossi-frontend/src/service/membership_adjust.rs anbinden (Round-Trip refresht beide Member-Detail-Views aus from + to)"
tech-stack:
  added: []
  patterns:
    - "1:1 Spiegelung des partial_repayment-Patterns (DTO-Layout, Handler-Body, ApiDoc-Eintraege)"
    - "Path-Parameter-Naming {from_id} fuer Sender-Klarheit (statt generisches {id})"
    - "PermissionDenied -> 401 (D-15-12 Codebase-Mapping), NICHT 403"
    - "Sub-Route VOR /{id}-Catch-All (D-14-08-Lesson / C-17-CF-06)"
key-files:
  created: []
  modified:
    - genossi_rest_types/src/lib.rs
    - genossi_rest/src/membership_adjust.rs
    - genossi_rest/src/member.rs
decisions:
  - "Path-Parameter heisst {from_id} statt {id} — Handler-Signatur macht klar, welche der beiden Member-IDs der Sender ist (Body hat to_member_id; Path muss dann self-erklaerend from_id sein)"
  - "Sub-Route eingefuegt direkt nach /{id}/partial-repayment und VOR den drei /{id}-Catch-All-Routes (defensive Konvention, D-14-08-Lesson)"
  - "TransferSharesResponseTO enthaelt BEIDE Members (from + to) statt nur from — Frontend-Round-Trip-Optimierung (Phase 18 kann beide Detail-Views ohne weiteren GET refreshen, inkl. neuer version + ggf. exit_date)"
  - "Tag-Beschreibung der ApiDoc auf 'Phase 15-17 v1.2 membership-adjust endpoints' aktualisiert (war 'Phase 15-16')"
  - "KEIN 403-Eintrag in responses(...) — Codebase mappt ServiceError::PermissionDenied global auf 401 (Phase 15 D-15-12-Lesson, gleicher Grep-Gate wie partial_repayment + cancel_membership)"
metrics:
  duration: "~20min"
  completed: "2026-06-06"
  tasks: 3
  files_modified: 3
---

# Phase 17 Plan 03: REST-Endpoint `POST /api/members/{from_id}/transfer-shares` Summary

Macht den in Plan 17-02 vollstaendig implementierten Service als HTTP-Endpoint erreichbar: Zwei DTOs in `genossi_rest_types`, ein Axum-Handler mit Utoipa-OpenAPI-Annotation in `genossi_rest/src/membership_adjust.rs` und die korrekt geordnete Sub-Route in `genossi_rest/src/member.rs::generate_route` (VOR `/{id}`-Catch-All). Plan 17-04 (E2E) kann den Endpoint direkt mit `reqwest` ansprechen; Phase 18 (Frontend) bekommt Round-Trip-Updates fuer beide Mitglieder in einer einzigen Response.

## Was wurde gebaut

### Task 1 — DTOs `TransferSharesRequestTO` + `TransferSharesResponseTO`

- `TransferSharesRequestTO` in `genossi_rest_types/src/lib.rs` Zeilen 576-596:
  - `pub to_member_id: Uuid`
  - `pub shares: i32` mit `#[schema(example = 2)]`
  - `pub transfer_date: time::Date` mit `iso8601_date_required`-Serde + `#[schema(example = "2026-06-15")]`
  - Doc-Kommentar dokumentiert TRSF-01, TRSF-07, D-17-01, D-17-08 (self-transfer-block)
- `TransferSharesResponseTO` in `genossi_rest_types/src/lib.rs` Zeilen 597-608:
  - `pub actions: Vec<MemberActionTO>`
  - `pub from: MemberTO`
  - `pub to: MemberTO`
  - Doc-Kommentar dokumentiert C-17-CF-07 (2 oder 3 Actions) + Round-Trip-Rationale
- Imports waren bereits am Top-of-File vorhanden (`Uuid`, `Serialize`, `Deserialize`, `ToSchema`, `iso8601_date_required`, `MemberTO`, `MemberActionTO`, `time::Date`).

**Commit:** `21b8024` — feat(17-03): TransferSharesRequestTO + TransferSharesResponseTO definieren

### Task 2 — Handler `transfer_shares` + ApiDoc-Registrierung

- Handler `transfer_shares<RestState: RestStateDef>` in `genossi_rest/src/membership_adjust.rs` Zeilen 185-245:
  - `#[instrument(skip(rest_state))]` + `#[utoipa::path(...)]` Annotation
  - Path-Parameter `from_id: Uuid`, Body `Json<TransferSharesRequestTO>`
  - Ruft `rest_state.membership_adjust_service().transfer_shares(from_id, req.to_member_id, req.shares, req.transfer_date, ctx, None).await?` auf
  - Wrappt Tuple-Result `(Vec<MemberAction>, Member, Member)` zu `TransferSharesResponseTO` und schickt als 200/application-json
  - Utoipa-`responses(...)` listet **200, 400, 401, 404, 409, 500** (D-17-10) — kein 403 (Phase 15 D-15-12 Codebase-Mapping)
- Imports erweitert um `TransferSharesRequestTO, TransferSharesResponseTO` (Zeile 26).
- ApiDoc-Struct in Zeilen 247-261 erweitert:
  - **Vorher:** `paths(cancel_membership, increase_shares, partial_repayment)` + 5 Components + Tag "Phase 15-16"
  - **Nachher:** `paths(cancel_membership, increase_shares, partial_repayment, transfer_shares)` + 7 Components (inkl. `TransferSharesRequestTO`, `TransferSharesResponseTO`) + Tag "Phase 15-17"

**Commit:** `23b7e55` — feat(17-03): transfer_shares Axum-Handler + ApiDoc-Registrierung

### Task 3 — Sub-Route in `member.rs::generate_route` (VOR `/{id}`-Catch-All)

- Neue Route-Registrierung in `genossi_rest/src/member.rs` Zeilen 78-85:
  ```rust
  // Phase 17 v1.2 (D-17 / C-17-CF-06): Sub-Route fuer Uebertrag.
  // MUSS vor /{id} registriert sein (D-14-08-Lesson) — axum-Routing-Defense.
  // Path-Parameter heisst {from_id} statt {id}, damit der Handler-Signatur
  // klar macht, welche der beiden Member-IDs der Sender ist.
  .route(
      "/{from_id}/transfer-shares",
      post(crate::membership_adjust::transfer_shares::<RestState>),
  )
  ```
- Ordering-Beweis: `grep -n '/transfer-shares\|"/{id}"' genossi_rest/src/member.rs` zeigt:
  - **Zeile 83:** `"/{from_id}/transfer-shares"` (neue Route)
  - **Zeile 87:** `.route("/{id}", get(...))` (erste `/{id}`-Catch-All)
  - **Zeile 89, 90:** weitere `/{id}` Routes
  - awk-Check `ts<id` => exit 0 (transfer-shares deklariert vor `/{id}`)
- Pitfall-1-Comment Z. 30-36 (existing) bleibt unveraendert; neuer Kommentar dokumentiert die spezifische D-14-08-Lesson fuer Phase 17.

**Commit:** `6e580ec` — feat(17-03): Sub-Route /{from_id}/transfer-shares registrieren (vor /{id})

## ApiDoc-Diff (vorher / nachher)

```diff
-    paths(cancel_membership, increase_shares, partial_repayment),
+    paths(cancel_membership, increase_shares, partial_repayment, transfer_shares),
     components(schemas(
         CancelMembershipRequestTO,
         IncreaseSharesRequestTO,
         MembershipAdjustResponseTO,
         PartialRepaymentRequestTO,
         PartialRepaymentResponseTO,
+        TransferSharesRequestTO,
+        TransferSharesResponseTO,
     )),
-    tags((name = "Members", description = "Phase 15-16 v1.2 membership-adjust endpoints"))
+    tags((name = "Members", description = "Phase 15-17 v1.2 membership-adjust endpoints"))
```

## Referenz-Punkte fuer Plan 17-04 (E2E)

- **DTO-Schemas** (eingefroren): `genossi_rest_types/src/lib.rs` Zeilen 576-608 — Plan 17-04 nutzt diese fuer `reqwest`-Body-Serialisierung + Response-Deserialisierung.
- **HTTP-Endpoint**: `POST /api/members/{from_id}/transfer-shares` mit `Content-Type: application/json`, Body = `TransferSharesRequestTO`.
- **Response 200**: Body = `TransferSharesResponseTO` mit `actions.len() == 2` (Teil) oder `actions.len() == 3` (Voll).
- **Error-Status-Codes** (D-17-10):
  - 400: ValidationError (self-transfer, shares out of range, transfer_date out of [today.year(), today.year()+1])
  - 401: PermissionDenied (kein Login oder keine admin-Rolle) — **NICHT 403**
  - 404: EntityNotFound (from oder to nicht existent / soft-deleted)
  - 409: Conflict (Recipient already cancelled per PERM-03 / D-17-07; ODER optimistic-locking)
  - 500: SQLITE_BUSY mid-cascade (Race-Test-Verlierer-Pfad)
- **Route-Reihenfolge**: Sub-Route VOR `/{id}`-Catch-All ist Voraussetzung, dass axum nicht "transfer-shares" als UUID parst und 400 zurueckgibt. Plan 17-04 muss das nicht erneut verifizieren.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] `SQLX_OFFLINE=true` fuer cargo build noetig**

- **Found during:** Task 1 Build
- **Issue:** Wie in Plan 17-01 und 17-02 — `genossi_dao_impl_sqlite` braucht SQLite-DB fuer SQLx-Compile-Time-Checks; im Worktree existiert kein `genossi.db`.
- **Fix:** `SQLX_OFFLINE=true` als Env-Variable bei `cargo build`/`cargo clippy`. Kein Code-Change. `.sqlx/`-Verzeichnis liefert die Offline-Query-Daten.
- **Files modified:** keine
- **Impact:** Konsistent mit Plan 17-01/02 — keine neue Auswirkung.

### Plan-Anweisung folgte 1:1

Plan 17-03 war so praezise spezifiziert (PATTERNS.md Sections 3-5 liefern fast wortwoertlich Code-Excerpts), dass kein weiteres Rule-1/2/3-Eingreifen noetig war:

- Imports waren bereits vollstaendig vorhanden (PATTERNS.md "no new module needed" stimmte).
- Path-Parameter-Naming `{from_id}` (Plan-Vorgabe) ist syntaktisch und semantisch sauber — keine Anpassung noetig.
- ApiDoc-Eintraege wurden konsequent alphabetisch erweitert (TransferSharesRequestTO direkt nach PartialRepaymentResponseTO, was die alphabetische Sortierung respektiert).

## Verification Results

| Check                                                                              | Erwartung               | Result        |
| ---------------------------------------------------------------------------------- | ----------------------- | ------------- |
| `cargo build --workspace --all-features` (mit `SQLX_OFFLINE=true`)                 | exit 0                  | exit 0        |
| `cargo clippy -p genossi_rest_types -p genossi_rest --all-targets` Fehler          | 0                       | 0             |
| `grep -c 'pub struct TransferSharesRequestTO' genossi_rest_types/src/lib.rs`       | 1                       | 1             |
| `grep -c 'pub struct TransferSharesResponseTO' genossi_rest_types/src/lib.rs`      | 1                       | 1             |
| `grep -c 'to_member_id: Uuid' genossi_rest_types/src/lib.rs`                       | >= 1                    | 1             |
| `grep -c 'pub shares: i32' genossi_rest_types/src/lib.rs`                          | >= 1                    | 7             |
| `grep -c 'iso8601_date_required' genossi_rest_types/src/lib.rs`                    | >= 4                    | 16            |
| `grep -c 'transfer_date: time::Date' genossi_rest_types/src/lib.rs`                | >= 1                    | 1             |
| `grep -c 'actions: Vec<MemberActionTO>' genossi_rest_types/src/lib.rs`             | >= 1                    | 1             |
| `grep -c 'pub async fn transfer_shares' genossi_rest/src/membership_adjust.rs`     | 1                       | 1             |
| `grep -c 'TransferSharesRequestTO' genossi_rest/src/membership_adjust.rs`          | >= 3                    | 4             |
| `grep -c 'TransferSharesResponseTO' genossi_rest/src/membership_adjust.rs`         | >= 3                    | 4             |
| `grep -c 'paths(cancel_membership, increase_shares, partial_repayment, transfer_shares)' genossi_rest/src/membership_adjust.rs` | 1 | 1 |
| `grep -c '"/{from_id}/transfer-shares"' genossi_rest/src/membership_adjust.rs`     | 1                       | 1             |
| `grep -c 'status = 200' genossi_rest/src/membership_adjust.rs`                     | >= 1                    | 4             |
| `grep -c 'status = 409' genossi_rest/src/membership_adjust.rs`                     | >= 1                    | 3             |
| **WARNING #3 D-15-12 Lesson**: `grep -c 'status = 403' genossi_rest/src/membership_adjust.rs` | **0**          | **0**         |
| `grep -c '/transfer-shares' genossi_rest/src/member.rs`                            | >= 1                    | 1             |
| `grep -c 'crate::membership_adjust::transfer_shares' genossi_rest/src/member.rs`   | >= 1                    | 1             |
| Route-Ordering awk-Check (transfer-shares VOR /{id})                               | exit 0                  | exit 0 (ts=83 < id=87) |
| `grep -c 'Pitfall 1' genossi_rest/src/member.rs`                                   | >= 1                    | 2             |

## Success Criteria Status

- [x] DTOs in `genossi_rest_types/src/lib.rs` definiert (Zeilen 576-608)
- [x] Handler in `genossi_rest/src/membership_adjust.rs` mit korrekten Utoipa-Annotationen (Zeilen 185-245, ApiDoc Zeilen 247-261)
- [x] Sub-Route in `genossi_rest/src/member.rs::generate_route` VOR `/{id}` (Zeilen 78-85, vor `/{id}` Zeile 87)
- [x] Swagger-UI zeigt POST `/api/members/{from_id}/transfer-shares` (Utoipa-Derive ist erfolgreich kompiliert; ApiDoc enthaelt den path-Eintrag)
- [x] Plan 17-04 kann auf den HTTP-Endpoint zugreifen (DTO-Schemas eingefroren + Endpoint mounted)
- [x] **TRSF-01** (Teil-Uebertrag) + **TRSF-07** (Self-Transfer-Block) auf REST-Ebene durch DTO + Service-Pipeline abgebildet

## Threat Flags

Keine neuen Trust-Boundary-Surface-Elemente jenseits des im Plan-`<threat_model>` deklarierten Bestands. Alle 7 STRIDE-Threats sind durch existing Phase-15/17 Mitigations + dieser Plan-Implementierung abgedeckt.

## Self-Check: PASSED

- File `genossi_rest_types/src/lib.rs` → FOUND (mit `TransferSharesRequestTO` Zeile 583, `TransferSharesResponseTO` Zeile 604)
- File `genossi_rest/src/membership_adjust.rs` → FOUND (mit `pub async fn transfer_shares` Zeile 214, ApiDoc-Eintraege Zeilen 249-258)
- File `genossi_rest/src/member.rs` → FOUND (mit `/transfer-shares` Zeile 83 VOR `/{id}` Zeile 87)
- Commit `21b8024` → FOUND in git log (feat(17-03): TransferSharesRequestTO + TransferSharesResponseTO definieren)
- Commit `23b7e55` → FOUND in git log (feat(17-03): transfer_shares Axum-Handler + ApiDoc-Registrierung)
- Commit `6e580ec` → FOUND in git log (feat(17-03): Sub-Route /{from_id}/transfer-shares registrieren (vor /{id}))
