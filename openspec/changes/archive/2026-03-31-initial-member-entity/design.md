## Context

The codebase is a Rust workspace with a clean layered architecture (DAO → Service → REST → Binary → Frontend). It currently implements an inventory management domain (Inventurly) but the infrastructure (OIDC, RBAC, sessions, Swagger, soft deletes, versioning) is domain-agnostic. We need to strip the Inventurly domain and replace it with a cooperative member management domain (Genossi), while preserving all infrastructure patterns documented in `openspec/specs/architecture/patterns.md`.

The current codebase has 7 crates (`inventurly_dao`, `inventurly_dao_impl_sqlite`, `inventurly_service`, `inventurly_service_impl`, `inventurly_rest`, `inventurly_rest_types`, `inventurly_bin`) plus a Dioxus frontend (`inventurly-frontend/`).

## Goals / Non-Goals

**Goals:**
- Rename all crates from `inventurly_*` to `genossi_*`
- Remove all Inventurly domain code (entities, services, endpoints, pages)
- Implement `Member` entity through all layers following existing patterns
- Maintain fully working OIDC authentication and RBAC
- Maintain Swagger UI at `/swagger-ui/`
- Provide member list and detail pages in the frontend
- All existing tests pass (adapted for new domain)
- E2E test infrastructure remains functional

**Non-Goals:**
- Member actions/events (Aufstockung, Austritt, Rückzahlung) — future change
- Document management (BE/BB tracking) — future change
- Computed shares/balance validation against actions — future change
- CSV import from existing Excel — future change (but data model supports it)
- Multi-language frontend — keep i18n infrastructure but only DE + EN for now

## Decisions

### 1. Rename strategy: Find-and-replace across entire workspace

**Decision**: Global rename `inventurly` → `genossi` / `Inventurly` → `Genossi` across all files, directories, and Cargo.toml references.

**Rationale**: A clean break is simpler than maintaining dual naming. The patterns doc captures everything we need to know about the old code. Since this is a fresh start (no users, no deployed instances to migrate), there's no backwards-compatibility concern.

**Alternative considered**: Keep `inventurly_*` crate names and only change the domain — rejected because the product name changes and dual naming would be confusing long-term.

### 2. Member number: Application-assigned integer, separate from UUID

**Decision**: `member_number: i64` as a unique, user-visible identifier alongside the system `id: UUID`. The member_number is assigned by the application (auto-increment or user-provided during import).

**Rationale**: The cooperative has existing member numbers from their Excel sheet. These are the official identifiers used in correspondence and legal documents. UUIDs are used internally for API operations and security (preventing enumeration).

**Alternative considered**: Using UUID as the only identifier — rejected because member numbers have legal/organizational significance.

### 3. Date fields: Use `time::Date` for join/exit dates, not `PrimitiveDateTime`

**Decision**: `join_date` and `exit_date` use `time::Date` (date only, no time component). System fields (`created`, `deleted`) remain `PrimitiveDateTime`.

**Rationale**: A member joins on a date, not at a specific time. Using Date avoids timezone confusion and matches the real-world semantics.

### 4. Balance storage: Cents as i64

**Decision**: `current_balance` stored as `i64` in cents. Displayed as Euro in the frontend.

**Rationale**: Consistent with the existing Price pattern in the codebase. Avoids floating-point issues. i64 gives enough range for any realistic cooperative balance.

### 5. Address fields: Flat on Member, all optional

**Decision**: Address fields (`street`, `house_number`, `postal_code`, `city`) are flat fields on the Member entity, all `Option<String>`.

**Rationale**: A separate Address entity would be over-engineering for a single-address-per-member model. Optional because some members (companies) may have incomplete address data during import.

**Alternative considered**: Separate `Address` entity — rejected, no current need for multiple addresses per member.

### 6. Frontend: Minimal but functional

**Decision**: Two pages — member list (with search/filter) and member detail (view/edit/create). Follow existing Dioxus patterns with GlobalSignal state management.

**Rationale**: Matches the existing page patterns (Products list → Product detail). Sufficient for the initial use case of managing the member list.

### 7. RBAC privileges for members

**Decision**: Two privileges: `view_members` and `manage_members`. Admin role gets both, a potential "viewer" role gets only `view_members`.

**Rationale**: Simple two-level access matches the cooperative's needs. Can be extended later without schema changes.

### 8. Fresh migration set

**Decision**: Remove all existing Inventurly migrations. Create a single initial migration that sets up the `member` table plus the existing auth/permission tables.

**Rationale**: Clean slate. No need to carry migration history from a different product. The auth tables (user, session, role, privilege, etc.) need to be recreated in the new migration set.

## Risks / Trade-offs

- **Risk**: Renaming all crates could break build in subtle ways (path references, feature flags, sqlx prepared queries)
  → Mitigation: Build and test after each major step. The `cargo build` and `cargo test` cycles will catch issues immediately.

- **Risk**: Losing useful infrastructure code during the strip
  → Mitigation: Patterns are documented in `openspec/specs/architecture/patterns.md`. Git history preserves everything.

- **Risk**: `member_number` uniqueness during import — what if Excel has gaps or duplicates?
  → Mitigation: `member_number` has a UNIQUE constraint. Import (future change) will validate and report conflicts.

- **Trade-off**: `current_shares` and `current_balance` are manually maintained fields that should eventually be computed from actions. Storing them now means potential drift.
  → Accepted: This is the explicit migration strategy — store now, validate later when actions exist.

## Open Questions

- Should `member_number` be auto-assigned on create, or always user-provided? (Leaning toward: user-provided for import, auto-increment for new members created in the app)
