## 1. i18n

- [x] 1.1 Add `OnlyPendingMigration` key to `Key` enum in `genossi-frontend/src/i18n/mod.rs`
- [x] 1.2 Add German translation for `OnlyPendingMigration` in `genossi-frontend/src/i18n/de.rs`
- [x] 1.3 Add English translation for `OnlyPendingMigration` in `genossi-frontend/src/i18n/en.rs`

## 2. Frontend Filter

- [x] 2.1 Add `only_pending_migration` signal in the `Members` component in `genossi-frontend/src/page/members.rs`
- [x] 2.2 Add filter checkbox for pending migration in the filter bar (alongside existing active/exited toggles)
- [x] 2.3 Add `.filter()` clause to `filtered_members` that filters by `migrated == false` when toggle is enabled
