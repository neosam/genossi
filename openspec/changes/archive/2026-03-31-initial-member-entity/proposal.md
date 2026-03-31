## Why

The project is being repurposed from an inventory management system (Inventurly) into a cooperative member management system (Genossi). The cooperative currently manages its member list in an Excel spreadsheet, which lacks access control, audit trails, and multi-user collaboration. A proper web application with OIDC authentication and role-based access is needed to manage members, their shares, and balances reliably.

## What Changes

- **BREAKING**: Remove all Inventurly domain entities (Product, Rack, Container, Inventur, Measurements, Reports, CSV Import, Duplicate Detection, Price Import, Deposit EAN Import)
- **BREAKING**: Rename all crates from `inventurly_*` to `genossi_*`
- Introduce `Member` entity through all layers (DAO, Service, REST, Frontend)
- Member fields: member_number, name, address, email, company, join/exit dates, shares, balance, bank account, comment
- Keep OIDC integration, RBAC, session management, Swagger UI fully intact
- Keep existing architecture patterns (soft deletes, versioning, trait-based DI, feature flags)
- Frontend: Dioxus-based member list and detail/edit pages
- Swagger UI remains at `/swagger-ui/`

## Capabilities

### New Capabilities
- `member-management`: CRUD operations for cooperative members including member number, personal data, address, share tracking, and balance management

### Modified Capabilities
_(none - all Inventurly capabilities are being removed, not modified)_

## Impact

- **All crates renamed**: `inventurly_dao` → `genossi_dao`, `inventurly_service` → `genossi_service`, etc.
- **Database**: New `member` table replaces all Inventurly tables. Existing migrations removed, fresh migration set.
- **REST API**: All `/api/products`, `/api/racks`, `/api/inventur`, etc. endpoints removed. New `/api/members` endpoint.
- **Frontend**: All Inventurly pages/components removed. New member list and member detail pages.
- **Dependencies**: No new external dependencies. All existing infrastructure dependencies remain.
- **OIDC/Auth**: No changes to authentication flow. RBAC privileges will be updated for member management.
- **Swagger/OpenAPI**: Updated to reflect new Genossi API with member endpoints.
