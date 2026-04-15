## Context

The member action form on the member detail page (`member_details.rs`) uses a single `shares_change` i32 input field. Users must enter negative values for Verkauf and Übertragung Abgabe, which is unintuitive. The field is also shown for Note actions where it must always be 0.

The backend API expects `shares_change` as a signed integer — positive for increases, negative for decreases. This contract will not change.

## Goals / Non-Goals

**Goals:**
- Users only enter positive numbers in the shares field
- The frontend automatically applies the correct sign based on action type before API submission
- The shares field is hidden for all action types where it must be 0 (status actions + Note)
- Dynamic labels communicate what the number means in context
- Editing existing actions displays the absolute value, not the raw negative

**Non-Goals:**
- No backend/API changes
- No changes to validation logic in the service layer
- No changes to the action list display (existing +/- coloring stays as-is)

## Decisions

### Decision 1: Sign conversion in the submit handler

The sign flip happens in the existing submit handler (around line 230 of `member_details.rs`), where `shares_change` is already conditionally set for status actions. For Verkauf and Übertragung Abgabe, the positive input value will be negated before building the `MemberActionTO`.

**Rationale:** Minimal change, single location, already has type-based branching logic.

**Alternative considered:** Converting in a separate helper function — unnecessary indirection for a simple negation.

### Decision 2: Extend `is_status_action()` check to include Note for field visibility

The existing `if !is_status` check that hides the shares field will be extended to also cover `Note` actions. Rather than modifying `is_status_action()` (which has semantic meaning beyond the form), a new condition `needs_shares_input()` or an inline check will determine field visibility.

**Rationale:** `is_status_action()` is used elsewhere with its current meaning. A dedicated check for "does this type need a shares input" is clearer.

### Decision 3: Dynamic i18n labels per action type category

New i18n keys for shares field labels:
- `SharesAdd` — for Aufstockung ("Anteile hinzufügen" / "Shares to add" / "Podíly k přidání")
- `SharesRemove` — for Verkauf ("Anteile abgeben" / "Shares to remove" / "Podíly k odebrání")
- `SharesReceive` — for Übertragung Empfang ("Anteile empfangen" / "Shares to receive" / "Podíly k přijetí")
- `SharesTransfer` — for Übertragung Abgabe ("Anteile übertragen" / "Shares to transfer" / "Podíly k převodu")

**Rationale:** Explicit per-category labels are clearer than trying to derive text from the action type name.

### Decision 4: Absolute value on edit load

When loading an existing action for editing (the click handler around line 921), `action_shares_change` will be set to `action.shares_change.abs()` instead of the raw value. The sign will be re-applied on save based on the action type, same as for new actions.

**Rationale:** Consistent behavior — the form always shows positive values regardless of create vs. edit mode.

### Decision 5: HTML min attribute on input

The shares input gets `min: "1"` to prevent the user from entering 0 or negative values via the number spinner. The `oninput` handler will additionally clamp parsed values to `max(1, value)`.

**Rationale:** Belt and suspenders — HTML constraint for UX, code constraint for correctness.

## Risks / Trade-offs

- **[Risk] User enters 0 via direct keyboard input** → Mitigated by clamping in `oninput` handler and backend validation as safety net.
- **[Trade-off] Existing actions with negative values displayed as positive in edit mode** → Acceptable because the action type already communicates the direction. The action list still shows signed values with color coding.
