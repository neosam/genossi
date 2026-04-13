## Context

The member list in `genossi-frontend/src/page/members.rs` renders a hardcoded HTML table with 10 fixed columns. `MemberTO` has ~20 fields, many of which (email, company, address fields, bank account) are only accessible on the detail page. There is no user-specific storage in the system — the existing `config-store` is global key-value without user association.

## Goals / Non-Goals

**Goals:**
- Users can select which columns appear in the member list
- Column preferences persist per user in the backend
- Reusable `user_preferences` backend infrastructure for future per-user settings
- Sensible default columns for users without saved preferences

**Non-Goals:**
- Column reordering (drag & drop) — columns follow a fixed order based on field definition
- Column width customization
- Shared/team column presets
- Inline editing (separate change)

## Decisions

### Decision 1: Dedicated `user_preferences` table vs. extending `config_store`

**Choice**: New `user_preferences` table with `(user_id, key)` composite unique constraint.

**Rationale**: The existing `config_store` is a global singleton store (key → value) without user association. Adding `user_id` would change its semantics. A separate table keeps concerns clean and follows the existing pattern where each domain has its own DAO/service/REST layer.

**Alternative considered**: Adding a `user_id` column to `config_store` — rejected because it would break the existing global config API and mix two different concepts.

### Decision 2: JSON value storage vs. normalized columns

**Choice**: Store preferences as JSON text in a `value` column, keyed by a string `key`.

**Rationale**: Different preferences have different shapes (column list = array of strings, future preferences = booleans, numbers, objects). A generic JSON store avoids schema migrations for each new preference type. The backend validates JSON structure per key as needed.

**Schema**:
```sql
CREATE TABLE user_preferences (
    id          BLOB PRIMARY KEY,
    user_id     BLOB NOT NULL,
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    created     TEXT NOT NULL,
    deleted     TEXT,
    version     BLOB NOT NULL,
    UNIQUE(user_id, key)
);
```

### Decision 3: REST API shape

**Choice**: Simple key-based endpoints scoped to the authenticated user.

```
GET  /api/user-preferences/{key}   → UserPreferenceTO (or 404)
PUT  /api/user-preferences/{key}   → UserPreferenceTO (upsert)
```

No list endpoint initially — preferences are fetched by key when needed. The user is derived from the auth context, so no `user_id` in the URL.

**Alternative considered**: Generic `GET /api/user-preferences` returning all preferences — deferred since the frontend only needs one key at a time.

### Decision 4: Column definition as data

**Choice**: Define all available columns as a static registry in the frontend with metadata:

```rust
struct ColumnDef {
    key: &'static str,        // "member_number", "last_name", ...
    label_key: &'static str,  // i18n key
    editable: bool,           // for future inline-edit feature
    render: fn(&MemberTO) -> String,
}
```

The column picker and table renderer both use this registry. Selected columns are stored as `Vec<String>` of keys.

**Rationale**: Single source of truth for column metadata. Adding a new column means adding one entry. The `editable` flag prepares for the inline-editing change without implementing it.

### Decision 5: Column picker UI

**Choice**: Popover/dropdown triggered by a "Spalten" button in the toolbar, showing checkboxes for all available columns.

**Rationale**: Stays within the existing toolbar pattern. No modal needed — the popover is lightweight and can be dismissed by clicking outside.

## Risks / Trade-offs

- **[User has no preferences yet]** → Default column set applied automatically. Default = current 10 columns so existing behavior is preserved.
- **[Column key renamed in code]** → Stored preferences reference old key, column silently disappears. Mitigation: Unknown keys are ignored during rendering; user sees default for missing columns.
- **[Performance of per-row rendering with dynamic columns]** → Negligible for typical member counts (<10k). No mitigation needed.
