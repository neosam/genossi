## Context

The member list page (`genossi-frontend/src/page/members.rs`) has three filter checkboxes: "Only active members", "Only exited members", and "Only pending migration". All default to `false`. The "Only active" and "Only exited" filters are currently mutually exclusive (enabling one disables the other). The reference date picker already exists and is used for the `is_active()` calculation.

## Goals / Non-Goals

**Goals:**
- Default "Only active members" to checked on page load
- Replace the "Only exited members" filter with a year-based exit filter using the reference date's year
- Allow combining "Only active" with "Exited in year" (they are no longer mutually exclusive)

**Non-Goals:**
- No separate year input field — reuse the reference date
- No backend filtering changes
- No changes to the "Only pending migration" filter

## Decisions

**1. Derive year from reference date**
The exit year filter uses `reference_date.year()` to determine the year. This avoids adding a new input field and naturally integrates with the existing reference date picker. Changing the reference date to a date in 2025 will filter exits in 2025.

**2. Remove mutual exclusion between active and exit filters**
Currently `only_active` and `only_exited` toggle each other off. With year-based exit filtering, these are no longer contradictory — a member can be active today but have an exit_date later this year. Remove the mutual exclusion logic.

**3. Dynamic label with year**
The checkbox label shows the year dynamically, e.g., "Ausgetreten in 2026" / "Exited in 2026". This makes the filter behavior immediately clear without needing to mentally connect it to the reference date.

**4. Filter logic**
A member matches the exit-in-year filter if:
- `exit_date` is `Some`
- `exit_date.year() == reference_date.year()`

## Risks / Trade-offs

- [Minimal risk] Small frontend-only change. The existing filter pattern is well-established.
- [UX consideration] Users must understand that the exit year comes from the reference date. The dynamic label mitigates this.
