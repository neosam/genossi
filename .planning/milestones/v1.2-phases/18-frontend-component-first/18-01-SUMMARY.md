---
phase: 18-frontend-component-first
plan: 01
subsystem: frontend
tags:
  - frontend
  - dto
  - rest-types
  - landmine-mitigation
dependency_graph:
  requires: []
  provides:
    - MemberSlimTO
    - CancelMembershipRequestTO
    - IncreaseSharesRequestTO
    - MembershipAdjustResponseTO
    - PartialRepaymentRequestTO
    - PartialRepaymentResponseTO
    - TransferSharesRequestTO
    - TransferSharesResponseTO
  affects:
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/membership_adjust_modal.rs
tech_stack:
  added:
    - serde_json (frontend rest-types crate)
  patterns:
    - Zero-Coupling Pattern (serde_json::Value für entry/phase im PartialRepaymentResponseTO)
    - Landmine L-2 Mitigation (Frontend rest-types als handgepflegte Teilkopie)
    - skip-if-none Serde-Semantik für optionale Felder
key_files:
  created: []
  modified:
    - genossi-frontend/rest-types/Cargo.toml
    - genossi-frontend/rest-types/src/lib.rs
    - genossi-frontend/Cargo.lock
decisions:
  - "8 neue DTOs als handgepflegte Teilkopie ohne utoipa::ToSchema und ohne iso8601_date_required (Default time::Date-Serde matched YYYY-MM-DD)"
  - "PartialRepaymentResponseTO.entry und .phase als serde_json::Value (Zero-Coupling) — Frontend braucht nur drei Felder, vollstaendige Repayment-TOs leben weiterhin in api.rs"
  - "Rule 1 Bugfix: bestehender make_member-Test-Helper fehlte status-Feld, blockierte Test-Kompilation"
metrics:
  duration_minutes: 8
  completed_date: 2026-06-07
  tasks_completed: 2
  files_modified: 3
  tests_added: 9
  total_tests_passing: 18
---

# Phase 18 Plan 01: Frontend rest-types DTOs Summary

8 Phase-15/16/17-Request/Response-DTOs als handgepflegte Teilkopie im Frontend-`rest-types`-Crate ohne `utoipa`-Backend-Abhaengigkeit hinzugefuegt (Landmine L-2 Mitigation), abgesichert durch 9 JSON-Roundtrip-Tests.

## Objective Achieved

Wave 2 (Plan 18-05 `api.rs` + Plan 18-06 `MembershipAdjustModal`) ist jetzt entblockt: alle benoetigten Request- und Response-Shapes existieren in `rest_types::{...}` ohne Frontend-Build-Brueche durch `utoipa::ToSchema`-Derives.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 8 DTOs + serde_json dep in rest-types | 0d0d3ff | rest-types/Cargo.toml, rest-types/src/lib.rs, Cargo.lock |
| 2 | 9 JSON-Roundtrip-Tests + Helper-Bugfix | 15447e0 | rest-types/src/lib.rs |

## Added DTOs

| DTO | Backend-Source | Frontend-Zweck |
|-----|----------------|----------------|
| `MemberSlimTO` | Phase 14 D-14-12 | DSGVO-konformer 7-Feld-Slim-TO fuer Transfer-Recipients-Liste |
| `CancelMembershipRequestTO` | Phase 15 D-15-11 | Request-Body fuer `POST /api/members/{id}/cancel` |
| `IncreaseSharesRequestTO` | Phase 15 D-15-15 | Request-Body fuer `POST /api/members/{id}/increase-shares` |
| `MembershipAdjustResponseTO` | Phase 15 D-15-11/D-15-15 | Shared Response-Shape (Cancel + Increase) `{ action, member }` |
| `PartialRepaymentRequestTO` | Phase 16 D-16-16 | Request-Body fuer `POST /api/members/{id}/partial-repayment` |
| `PartialRepaymentResponseTO` | Phase 16 D-16-16 | Response-Shape `{ entry, member, phase }` (Zero-Coupling via `serde_json::Value`) |
| `TransferSharesRequestTO` | Phase 17 C-17-CF-07 | Request-Body fuer `POST /api/members/{from_id}/transfer-shares` |
| `TransferSharesResponseTO` | Phase 17 C-17-CF-07 | Response-Shape `{ actions, from, to }` (2 actions bei Teil-, 3 bei Voll-Uebertrag) |

## Test Output (`cargo test --lib phase_18_dtos_tests`)

```
running 9 tests
test phase_18_dtos_tests::cancel_request_serializes_iso_date ... ok
test phase_18_dtos_tests::increase_request_roundtrip_with_shares ... ok
test phase_18_dtos_tests::member_slim_to_skips_none_fields ... ok
test phase_18_dtos_tests::member_slim_to_roundtrip ... ok
test phase_18_dtos_tests::partial_repayment_response_phase_none_skips_phase ... ok
test phase_18_dtos_tests::membership_adjust_response_roundtrip_nested ... ok
test phase_18_dtos_tests::partial_repayment_response_phase_some_serializes_phase ... ok
test phase_18_dtos_tests::transfer_request_full_roundtrip ... ok
test phase_18_dtos_tests::transfer_response_two_actions_roundtrip ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
```

Vollstaendiger Test-Run der Crate liefert `18 passed; 0 failed` (9 neu + 9 bestehend).

## Key Decisions Made

1. **Zero-Coupling fuer PartialRepaymentResponseTO**: `entry` und `phase` als `serde_json::Value` typisiert. Begruendung: Die vollstaendigen `RepaymentEntryTO`/`RepaymentPhaseTO` leben aus historischen Gruenden in `genossi-frontend/src/api.rs`, nicht in `rest-types`. Frontend braucht nur drei Felder (`entry.id`, `phase.id`, `phase.fiscal_year`). `serde_json::Value` vermeidet einen `rest-types` → `api.rs`-Reverse-Dependency-Bruch und haelt das Crate clean.

2. **Default-`time::Date`-Serde statt Custom `iso8601_date_required`**: Das `serde-human-readable`-Feature von `time` 0.3 liefert direkt `YYYY-MM-DD`, exakt das gleiche Format wie das Backend-`iso8601_date_required`-Modul. Verifiziert durch `cancel_request_serializes_iso_date`-Test. Spart Custom-Serde-Modul-Duplikation.

3. **Keine `utoipa::ToSchema`-Derives**: Frontend-Crate hat keine `utoipa`-Dependency (WASM-Build). Backend-Derives werden bewusst NICHT 1:1 uebernommen. Inline-Doc-Kommentar im neuen Block dokumentiert die Differenz fuer kuenftige Reviews.

## Hinweis zu Repayment-TOs

`RepaymentEntryTO` und `RepaymentPhaseTO` werden NICHT nach `rest-types` kopiert — sie leben in `genossi-frontend/src/api.rs` (historisch dort, vor der `rest-types`-Crate-Extraktion entstanden). `PartialRepaymentResponseTO` umgeht das Problem mit `serde_json::Value` als zero-coupling Pattern. Eine spaetere Phase koennte beide TOs migrieren, was aber Rule-4-architectural-change waere und nicht in Plan-Scope ist.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] serde_json fehlt in rest-types/Cargo.toml**
- **Found during:** Task 1 (vor dem Schreiben der Tests)
- **Issue:** Plan-Action-Block nutzt `serde_json::Value` im `PartialRepaymentResponseTO`, aber `rest-types/Cargo.toml` listet keine `serde_json`-Dependency. `cargo check` schlaegt sonst sofort fehl.
- **Fix:** `serde_json = "1.0"` in `[dependencies]` hinzugefuegt; loest Cargo.lock-Aenderung (1 Zeile) im uebergeordneten `genossi-frontend/Cargo.lock` aus.
- **Files modified:** `genossi-frontend/rest-types/Cargo.toml`, `genossi-frontend/Cargo.lock`
- **Commit:** 0d0d3ff

**2. [Rule 1 - Bug] Existing `make_member` test helper missed `status` field**
- **Found during:** Task 2 (`cargo test --lib phase_18_dtos_tests` schlug E0063 fehl)
- **Issue:** Der pre-existente Test-Helper in `mod tests` initialisierte alle `MemberTO`-Felder ausser `status` — Rust erwartet zur Struct-Init-Time ALLE Felder (`#[serde(default)]` greift nur bei Deserialisierung). Pre-existing latent bug: Existing Tests werden nur kompiliert wenn `cargo test` laeuft; `cargo check` (Task 1's Verify) entdeckt das nicht.
- **Fix:** `status: MemberStatusTO::Normal` ins `make_member`-Fixture eingefuegt.
- **Files modified:** `genossi-frontend/rest-types/src/lib.rs` (1 Zeile in pre-existing test fixture)
- **Commit:** 15447e0

### Cleanup notes

- Beim Ausfuehren von `cargo check`/`cargo test` aus dem `genossi-frontend/rest-types/`-Verzeichnis heraus wurde eine standalone `Cargo.lock` erzeugt (das rest-types-Crate ist ein Pfad-Dependency ohne `[workspace]`-Setup). Diese standalone `Cargo.lock` wurde vor dem Commit entfernt (`git rm --cached` + Filesystem-Loeschung); der uebergeordnete `genossi-frontend/Cargo.lock` ist die Single-Source-of-Truth.

## Field-Order Adaptation

Plan-`<action>`-Test-Fixture `sample_action()` hatte Felder in dieser Reihenfolge: `id, member_id, action_type, date, effective_date, shares_change, transfer_member_id, note, ...`. Frontend-`MemberActionTO` (Z. 351-387) hat aber: `id, member_id, action_type, date, shares_change, transfer_member_id, effective_date, comment, ...`. Der Plan-ANMERKUNG-Block sagt explizit "Executor passt fixture an existierende Definition an" — exakt das geschehen: `note` → `comment`, Reihenfolge `effective_date` ↔ `shares_change` getauscht. Test verifiziert das Schema der neuen DTOs, nicht das von `MemberActionTO`.

## Threat Surface Scan

Keine neuen Threat-Flags. Die Plan-`<threat_model>`-Mitigation (`T-18-01-01` DTO-Schema-Mismatch) ist durch die 9 Roundtrip-Tests erfuellt. Keine neuen Netzwerk-Endpoints, Auth-Pfade, File-Zugriffe oder Schema-Aenderungen an Trust-Boundaries.

## Self-Check: PASSED

- File `genossi-frontend/rest-types/src/lib.rs` exists with 8 new DTOs + 9 tests
- Commit 0d0d3ff exists (Task 1)
- Commit 15447e0 exists (Task 2)
- `cargo test --lib phase_18_dtos_tests` → `9 passed; 0 failed`
- `cargo test --lib` (full crate) → `18 passed; 0 failed`

## Confirmation

Wave 2 (Plan 18-05 `api.rs`-Wiring + Plan 18-06 `MembershipAdjustModal`) kann jetzt importieren:

```rust
use rest_types::{
    MemberSlimTO,
    CancelMembershipRequestTO, IncreaseSharesRequestTO, MembershipAdjustResponseTO,
    PartialRepaymentRequestTO, PartialRepaymentResponseTO,
    TransferSharesRequestTO, TransferSharesResponseTO,
};
```

Landmine L-2 (Frontend-rest-types ist eine handgepflegte Teilkopie) ist fuer Phase 18 geschlossen.
