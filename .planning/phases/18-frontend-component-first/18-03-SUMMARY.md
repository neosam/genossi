---
phase: 18-frontend-component-first
plan: 03
subsystem: frontend-component
tags:
  - frontend
  - component
  - member-search
  - refactor
  - phase-18
  - L-5-mitigation
dependency_graph:
  requires:
    - Phase 12 MemberSearch component (existing)
    - rest-types::MemberTO with PartialEq derive (already present in HEAD via Plan 18-01)
  provides:
    - MemberSearch component with optional members_override prop
    - Unblocks Plan 18-05 (TransferSubView with get_transfer_recipients-Adapter)
  affects:
    - genossi-frontend/src/component/member_search.rs (signature + body + tests)
tech_stack:
  added: []
  patterns:
    - "Optional prop with #[props(default)] for backward-compatible component extension"
    - "Override-or-fallback pattern: members_override = None → global MEMBERS signal"
    - "Owned-clone for short-lived signal-read guards (lifetime-safety vs MAX_RESULTS=10 cost)"
key_files:
  created: []
  modified:
    - genossi-frontend/src/component/member_search.rs
decisions:
  - "Owned Vec<MemberTO> clone over reference juggling — MEMBERS.read() guard lifetime would not survive a match-arm borrow; clone cost is trivial (≤10 entries used)"
  - "PartialEq for MemberTO already in HEAD (Plan 18-01) — no rest-types change needed, props compile cleanly"
metrics:
  duration: "≈10 minutes"
  completed: 2026-06-07
requirements:
  - UI-02
---

# Phase 18 Plan 03: MemberSearch members_override-Prop Summary

`MemberSearch`-Component um optionalen `members_override: Option<Vec<MemberTO>>`-Prop erweitert (L-5-Mitigation aus PATTERNS.md) — Backward-kompatibel, Phase-12-Callsites unverändert, Plan 05 (TransferSubView) entblockt.

## Vorher / Nachher — Component-Signatur

### Vorher (Phase 12, `member_search.rs:41-46`)
```rust
#[component]
pub fn MemberSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
    exclude_id: Option<Uuid>,
) -> Element { ... }
```

### Nachher (Phase 18, `member_search.rs:41-49`)
```rust
#[component]
pub fn MemberSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
    exclude_id: Option<Uuid>,
    // Phase 18 L-5: optional override; None = use global MEMBERS (Phase 12 default).
    #[props(default)]
    members_override: Option<Vec<MemberTO>>,
) -> Element { ... }
```

### Body-Änderung (Zeilen 53-61)
```rust
// Phase 18 L-5: override-list takes precedence over global MEMBERS.
// We clone into an owned Vec to avoid lifetime issues across the match arms
// (MEMBERS.read() returns a guard that doesn't live long enough to outlive
// the match). MAX_RESULTS=10 keeps the clone cost trivial.
let members_owned: Vec<MemberTO> = match members_override.clone() {
    Some(list) => list,
    None => MEMBERS.read().items.clone(),
};
let members: &[MemberTO] = members_owned.as_slice();
```

`filter_members(&[MemberTO], &str, Option<Uuid>) -> Vec<&MemberTO>` Pure-Function-Signatur **unverändert**.

## Test-Count-Vergleich

| Vor Plan 03 | Nach Plan 03 |
|---|---|
| 8 Tests grün | **10 Tests grün** (8 bestehende + 2 neue) |

Neu hinzugefügt am Ende von `mod tests`:
- `test_filter_members_works_with_custom_list` — `filter_members` über benutzerdefinierter Slice (TransferSubView-Pattern)
- `test_filter_members_empty_override_returns_nothing` — leere Override-Liste → kein Resultat

Verification-Output (`cargo test --bin genossi-frontend component::member_search`):
```
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 225 filtered out
```

## Plan 05 (TransferSubView) Entblockt

Bestätigt: `MembershipAdjustModal`-TransferSubView kann den Empfänger-Picker mit `MemberSearch { members_override: Some(adapted), … }` realisieren, wobei `adapted` aus `get_transfer_recipients`-Response über einen `MemberSlimTO → MemberTO`-Adapter (Plan 05 Detail) entsteht.

Default-Pfad (`members_override = None`) bleibt für alle Phase-12-Callsites (z. B. `repayment_phase_details.rs`) identisch — keine Migration nötig.

## Acceptance-Criteria-Verifikation

| Kriterium | Resultat |
|---|---|
| `grep -c "members_override: Option<Vec<MemberTO>>" member_search.rs == 1` | ✓ 1 |
| `grep -c "Phase 18" member_search.rs >= 1` | ✓ 4 |
| `grep -c "fn test_filter_members_works_with_custom_list" == 1` | ✓ 1 |
| `grep -c "fn test_filter_members_empty_override_returns_nothing" == 1` | ✓ 1 |
| `cargo check` exit 0 | ✓ (24 unrelated warnings) |
| 10 Tests grün (8 + 2) | ✓ |
| Pure-Function `filter_members<'a>` UNVERÄNDERT | ✓ |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] PartialEq für `MemberTO` in `rest-types/src/lib.rs`**
- **Found during:** Task 1 (`cargo check` schlug fehl mit `binary operation == cannot be applied to type Option<Vec<MemberTO>>`)
- **Issue:** Dioxus `#[component]`-Makro erzeugt `PartialEq`-basiertes Props-Diffing → `Option<Vec<MemberTO>>`-Prop kompiliert nur, wenn `MemberTO: PartialEq`.
- **Fix:** `PartialEq` zu `MemberTO`-Derive hinzugefügt: `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]`. Alle Felder (Uuid, i64, String, Option<Salutation/MemberStatus/Date/PrimitiveDateTime>, bool, i32) sind bereits `PartialEq`.
- **Files modified:** `genossi-frontend/rest-types/src/lib.rs` Zeile 188.
- **Status:** Auf HEAD bereits durch parallelen Plan-18-01-Commit (`0d0d3ff`) eingeführt — meine Edit war ein No-Op (kein Diff). Funktional korrekt: `cargo check` ist grün.

### Concurrency Note (kein Plan-Bug, sondern Worktree-Setup)

**Bündelung fremder Änderungen in Plan-Commit `4fc6899`**

- **Beobachtung:** Parallel laufende Worktree-Agenten (Plan 18-02, 18-05) committen in denselben Git-Index. Zum Zeitpunkt meines `git commit`-Aufrufs lagen ~122 Zeilen `genossi-frontend/src/api.rs`-Änderungen (aus Plan-05-Vorarbeit) ungestaged im Working-Dir; obwohl ich explizit nur `member_search.rs` mit `git add` stagte, hat Git unter konkurrenter Index-Mutation diese Änderungen mitgenommen.
- **Konsequenz:** Commit `4fc6899` enthält `member_search.rs` (meine Arbeit) UND `api.rs` (Plan-05-Vorarbeit). Beide Veränderungen sind unabhängig korrekt und kompilieren; die `api.rs`-Änderungen sind sowieso für Plan 06/07-Konsum geplant.
- **Mitigation:** Orchestrator merged Worktree-Streams sowieso zusammen — die Bündelung ist semantisch unschädlich, nur die Commit-Atomarität ist abgeschwächt. Kein Rollback (würde fremde Arbeit zerstören).

## Verification Steps Run

1. `cd genossi-frontend && cargo check` — ✓ kompiliert (24 unrelated warnings)
2. `cd genossi-frontend && cargo test --bin genossi-frontend component::member_search` — ✓ 10 passed
3. Acceptance-Criteria-greps — ✓ alle erfüllt

## Self-Check: PASSED

Files created:
- FOUND: `.planning/phases/18-frontend-component-first/18-03-SUMMARY.md`

Files modified:
- FOUND: `genossi-frontend/src/component/member_search.rs` (Commit `4fc6899`)

Commits:
- FOUND: `4fc6899 feat(18-03): add members_override prop to MemberSearch`
