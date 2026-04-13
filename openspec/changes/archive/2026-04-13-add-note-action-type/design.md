## Context

The `ActionType` enum currently has 7 variants covering membership status changes and share transactions. Each variant has specific validation rules enforced in the service layer. The proposal adds `Note` as an 8th variant for free-text entries.

The change is straightforward — it follows the exact same pattern as existing action types. No new tables, no new entities, no architectural changes.

## Goals / Non-Goals

**Goals:**
- Add `Note` variant to `ActionType` across all layers
- Enforce validation: `shares_change = 0`, no `transfer_member_id`, no `effective_date`, `comment` required
- Exclude `Note` from migration action count (same as status actions)
- Display `Note` in frontend action list and action editor

**Non-Goals:**
- Rich text or attachments for notes
- Separate notes entity or timeline integration
- Categories or tags for notes

## Decisions

### 1. `Note` follows the same pattern as status actions for validation

`Note` requires `shares_change = 0` and no `transfer_member_id`, similar to `Eintritt`/`Austritt`/`Todesfall`. However, `Note` is not a status action — it does not affect member dates.

**Rationale:** Keeps validation logic consistent. The `is_status_action()` method should NOT include `Note`, since notes don't affect join/exit dates.

### 2. `comment` field is required for `Note` actions

Unlike other action types where `comment` is optional, `Note` actions must have a non-empty `comment` — the comment IS the note content.

**Rationale:** A note without text is meaningless. Enforcing this at the service layer prevents empty notes.

### 3. `Note` excluded from migration action count

Migration status counts "significant" actions (share changes). `Note` actions, like status actions, should not count toward the expected action count.

**Rationale:** Notes are informational and should not break migration validation.

## Risks / Trade-offs

- [Minimal risk] Adding an enum variant requires updating match arms across all layers → Low risk since the compiler will catch missing arms.
