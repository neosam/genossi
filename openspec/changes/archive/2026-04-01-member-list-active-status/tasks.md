## 1. i18n Keys

- [x] 1.1 Add i18n keys: `ReferenceDate`, `Active`, `Inactive`, `OnlyActiveMembers` to `mod.rs`, `de.rs`, `en.rs`

## 2. Member List Page

- [x] 2.1 Add a `reference_date` signal defaulting to today in the Members component
- [x] 2.2 Add a date picker input bound to `reference_date` between the search bar and table
- [x] 2.3 Add a `only_active` boolean signal and a checkbox toggle next to the date picker
- [x] 2.4 Add an `is_active(member, date)` helper function that checks `join_date <= date AND (exit_date IS NULL OR exit_date > date)`
- [x] 2.5 Add "Active" column header to the table
- [x] 2.6 Add active/inactive badge cell to each row using green/red styling
- [x] 2.7 Apply the `only_active` filter to the filtered members list when the toggle is enabled
