## 1. i18n: Replace exit filter translation key

- [x] 1.1 Replace `OnlyExitedMembers` with `ExitedInYear` in `Key` enum in `genossi-frontend/src/i18n/mod.rs`
- [x] 1.2 Update German translation in `de.rs`: `"Ausgetreten in"` (year appended in component)
- [x] 1.3 Update English translation in `en.rs`: `"Exited in"` (year appended in component)
- [x] 1.4 Update Czech translation in `cs.rs` if `OnlyExitedMembers` exists there (not present, skipped)

## 2. Frontend: Update filter defaults and logic

- [x] 2.1 Change `only_active` default from `false` to `true` in `genossi-frontend/src/page/members.rs`
- [x] 2.2 Replace `only_exited` signal with `exited_in_year` signal (default `false`)
- [x] 2.3 Replace the exit filter logic: instead of `!is_active(m, &ref_date)`, check `m.exit_date.map(|d| d.year() == ref_date.year()).unwrap_or(false)`
- [x] 2.4 Remove mutual exclusion between active filter and exit-in-year filter (remove the `only_exited.set(false)` / `only_active.set(false)` toggle logic)
- [x] 2.5 Update the exit filter checkbox label to use `ExitedInYear` key + reference date year (e.g., `format!("{} {}", i18n.t(Key::ExitedInYear), ref_date.year())`)

## 3. Tests

- [x] 3.1 Add/update tests verifying the year-based exit filter logic
