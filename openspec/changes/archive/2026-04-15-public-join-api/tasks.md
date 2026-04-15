## 1. Database & DAO Layer

- [x] 1.1 Create SQLite migration for `applications` table (id, first_name, last_name, salutation, email, street, house_number, postal_code, city, shares, status, created, deleted, version)
- [x] 1.2 Define `ApplicationEntity` and `ApplicationStatus` enum (Offen, Bestätigt, Abgelehnt) in `genossi_dao`
- [x] 1.3 Define `ApplicationDao` trait with `dump_all()`, `create()`, `update()` methods
- [x] 1.4 Implement `ApplicationDao` for SQLite in `genossi_dao_impl_sqlite`
- [x] 1.5 Add unit tests for SQLite DAO implementation

## 2. Service Layer

- [x] 2.1 Define `ApplicationService` trait in `genossi_service` with methods: `submit()`, `list()`, `get()`, `confirm()`, `reject()`
- [x] 2.2 Implement `ApplicationService` in `genossi_service_impl` with validation (required fields, shares >= 1)
- [x] 2.3 Implement `submit()`: create application with status Offen, trigger confirmation mail
- [x] 2.4 Implement `confirm()`: validate status is Offen, create member via existing member service (next member number, join_date=today, Eintritt+Aufstockung), set application status to Bestätigt
- [x] 2.5 Implement `reject()`: validate status is Offen, set application status to Abgelehnt
- [x] 2.6 Implement `list()` with optional status filter and `get()` by ID
- [x] 2.7 Implement confirmation mail logic: read config store values (share_value_cents, bank_iban, bank_name, bank_bic, genossenschaft_name), render mail body, queue via mail infrastructure
- [x] 2.8 Add unit tests for service layer (mock DAO, mock mail, mock member service)

## 3. REST Layer

- [x] 3.1 Define REST types for Application in `genossi_rest_types` (ApplicationResponse, PublicJoinRequest)
- [x] 3.2 Implement `POST /api/public/join` endpoint with API-Key validation from config store
- [x] 3.3 Implement `GET /api/applications` endpoint with optional status query parameter (requires manage_members)
- [x] 3.4 Implement `GET /api/applications/{id}` endpoint (requires manage_members)
- [x] 3.5 Implement `POST /api/applications/{id}/confirm` endpoint (requires manage_members)
- [x] 3.6 Implement `POST /api/applications/{id}/reject` endpoint (requires manage_members)
- [x] 3.7 Add OpenAPI/Utoipa annotations for all new endpoints
- [x] 3.8 Register routes in REST server (public routes under `/api/public/join`, admin routes under `/api/applications`)

## 4. Config & API Key

- [x] 4.1 Implement `POST /api/config/generate-api-key` endpoint that generates UUID v4 and stores as `public_api_key` (type secret)
- [x] 4.2 Add OpenAPI annotation for generate-api-key endpoint

## 5. Wire-up & Integration

- [x] 5.1 Wire ApplicationDao, ApplicationService into dependency injection in `genossi_bin`
- [x] 5.2 Add application routes to the Axum router
- [x] 5.3 Run database migration on startup (automatic via existing migration runner)

## 6. Testing

- [x] 6.1 Add E2E tests: submit application via public endpoint with valid API key
- [x] 6.2 Add E2E tests: submit with missing/invalid API key → 401
- [x] 6.3 Add E2E tests: submit with missing fields → 422
- [x] 6.4 Add E2E tests: list/get applications as admin
- [x] 6.5 Add E2E tests: confirm application → verify member created with correct data
- [x] 6.6 Add E2E tests: confirm already confirmed/rejected → 409
- [x] 6.7 Add E2E tests: reject application, reject already confirmed → 409
