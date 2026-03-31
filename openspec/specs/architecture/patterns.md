# Architecture Patterns Reference

This document captures the repeatable patterns from the original Inventurly codebase, preserved as a blueprint for building new entities in Genossi.

## Layer Overview

```
REST (Axum + Utoipa)          -- Transfer Objects (TO), OpenAPI docs
       |
Service (Traits + Impls)      -- Business logic, validation, permissions
       |
DAO (Traits + SQLite Impl)    -- Data access, soft deletes, versioning
       |
SQLite (sqlx migrations)      -- Schema, indexes
       |
Frontend (Dioxus + Tailwind)  -- Pages, components, global state, i18n
```

---

## 1. DAO Layer

### Entity Struct
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FooEntity {
    pub id: Uuid,
    pub name: Arc<str>,           // Arc<str> for cheap cloning
    pub some_field: i32,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,  // Soft delete
    pub version: Uuid,            // Optimistic locking
}
```

### DAO Trait (3 required + 2 default methods)
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait FooDao {
    type Transaction: crate::Transaction;

    // Required: fetch ALL records (including deleted)
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[FooEntity]>, DaoError>;
    // Required: insert
    async fn create(&self, entity: &FooEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    // Required: update (with version check)
    async fn update(&self, entity: &FooEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    // Default: filter dump_all to active records
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[FooEntity]>, DaoError> {
        let all = self.dump_all(tx).await?;
        Ok(all.iter().filter(|e| e.deleted.is_none()).cloned().collect::<Vec<_>>().into())
    }

    // Default: find by ID among active records
    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<FooEntity>, DaoError> {
        let all = self.dump_all(tx).await?;
        Ok(all.iter().find(|e| e.id == id && e.deleted.is_none()).cloned())
    }
}
```

### SQLite Implementation

**DB row struct:**
```rust
#[derive(Debug, sqlx::FromRow)]
struct FooDb {
    id: Vec<u8>,              // BLOB
    name: String,
    some_field: i32,
    created: String,          // ISO8601 text
    deleted: Option<String>,
    version: Vec<u8>,         // BLOB
}
```

**Conversion (TryFrom):**
```rust
impl TryFrom<&FooDb> for FooEntity {
    type Error = DaoError;
    fn try_from(db: &FooDb) -> Result<Self, Self::Error> {
        Ok(FooEntity {
            id: Uuid::from_slice(&db.id)?,
            name: Arc::from(db.name.as_str()),
            some_field: db.some_field,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}
```

**DAO impl struct:**
```rust
pub struct FooDaoImpl { pub pool: Arc<SqlitePool> }

#[async_trait]
impl FooDao for FooDaoImpl {
    type Transaction = TransactionImpl;
    // dump_all: sqlx::query_as::<_, FooDb>("SELECT ... FROM foo ORDER BY name")
    // create:   sqlx::query("INSERT INTO foo (...) VALUES (?, ?, ...)")
    // update:   WHERE id = ? AND version = ? AND deleted IS NULL -> ConflictError if 0 rows
}
```

**Key patterns in update():**
- Generate new version: `Uuid::new_v4().as_bytes().to_vec()`
- Check existence first, then update with version WHERE clause
- `rows_affected() == 0` -> `DaoError::ConflictError`

### Migration
```sql
CREATE TABLE IF NOT EXISTS foo (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    some_field INTEGER NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_foo_deleted ON foo(deleted);
```

---

## 2. Service Layer

### Service Entity (mirrors DAO entity)
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foo {
    pub id: Uuid,
    pub name: Arc<str>,
    pub some_field: i32,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

// Bidirectional From conversions between Foo <-> FooEntity
impl From<&FooEntity> for Foo { ... }
impl From<&Foo> for FooEntity { ... }
```

### Service Trait
```rust
#[automock(type Context=(); type Transaction = inventurly_dao::MockTransaction;)]
#[async_trait]
pub trait FooService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: inventurly_dao::Transaction;

    async fn get_all(&self, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<Arc<[Foo]>, ServiceError>;
    async fn get(&self, id: Uuid, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<Foo, ServiceError>;
    async fn create(&self, item: &Foo, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<Foo, ServiceError>;
    async fn update(&self, item: &Foo, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<Foo, ServiceError>;
    async fn delete(&self, id: Uuid, context: Authentication<Self::Context>, tx: Option<Self::Transaction>) -> Result<(), ServiceError>;
}
```

### Service Implementation (using macro)
```rust
gen_service_impl! {
    struct FooServiceImpl: FooService = FooServiceDeps {
        FooDao: FooDao<Transaction = Self::Transaction> = foo_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**The macro generates:**
1. `FooServiceDeps` trait (with associated types for each dependency)
2. `FooServiceImpl<Deps>` struct (with `Arc<Deps::FieldType>` fields)
3. `FooServiceImpl::new(...)` constructor

### Standard CRUD Implementation Pattern
```rust
#[async_trait]
impl<Deps: FooServiceDeps> FooService for FooServiceImpl<Deps> {
    // get_all: use_transaction -> check_permission -> dao.all() -> map to Foo -> commit
    // get:     use_transaction -> check_permission -> dao.find_by_id() -> commit
    // create:  use_transaction -> check_permission -> validate -> uuid_service.new_v4() -> dao.create() -> commit
    // update:  use_transaction -> check_permission -> validate -> dao.update() -> commit
    // delete:  use_transaction -> check_permission -> find_by_id -> set deleted timestamp -> dao.update() -> commit
}
```

**Validation pattern:**
```rust
let mut errors = Vec::new();
if item.name.is_empty() {
    errors.push(ValidationFailureItem { field: Arc::from("name"), message: Arc::from("Name cannot be empty") });
}
if !errors.is_empty() {
    return Err(ServiceError::ValidationError(errors));
}
```

**Soft delete pattern:**
```rust
async fn delete(&self, id: Uuid, ...) -> Result<(), ServiceError> {
    let existing = self.foo_dao.find_by_id(id, tx.clone()).await?;
    match existing {
        Some(mut entity) => {
            let now = time::OffsetDateTime::now_utc();
            entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
            self.foo_dao.update(&entity, PROCESS, tx.clone()).await?;
            self.transaction_dao.commit(tx).await?;
            Ok(())
        }
        None => Err(ServiceError::EntityNotFound(id)),
    }
}
```

---

## 3. REST Layer

### Transfer Object
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FooTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "Example Name")]
    pub name: String,
    #[schema(example = 42)]
    pub some_field: i32,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "iso8601_datetime::serialize", deserialize_with = "iso8601_datetime::deserialize", default)]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "iso8601_datetime::serialize", deserialize_with = "iso8601_datetime::deserialize", default)]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// Bidirectional From: &Foo <-> FooTO
```

### Route Registration
```rust
// In foo.rs:
pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all::<RestState>))
        .route("/{id}", get(get_one::<RestState>))
        .route("/", post(create::<RestState>))
        .route("/{id}", put(update::<RestState>))
        .route("/{id}", delete(delete_one::<RestState>))
}

// In lib.rs:
.nest("/foos", foo::generate_route())
// + add to OpenApi nest macro
```

### Handler Pattern
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(get, tag = "Foos", path = "", responses((status = 200, body = [FooTO])))]
pub async fn get_all<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler((async {
        let items: Arc<[FooTO]> = rest_state
            .foo_service()
            .get_all(crate::extract_auth_context(Some(context))?, None)
            .await?
            .iter()
            .map(FooTO::from)
            .collect();
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&items).unwrap()))
            .unwrap())
    }).await)
}
```

### RestStateDef Extension
For each new entity, add to the trait:
```rust
type FooService: FooService<Context = ContextType> + Send + Sync + 'static;
fn foo_service(&self) -> Arc<Self::FooService>;
```

### OpenAPI Doc
```rust
#[derive(OpenApi)]
#[openapi(
    paths(get_all, get_one, create, update, delete_one),
    components(schemas(FooTO)),
    tags((name = "Foos", description = "Foo management"))
)]
pub struct ApiDoc;
```

---

## 4. Binary Layer (Dependency Wiring)

```rust
// In inventurly_bin/src/lib.rs:
pub struct FooServiceDependencies;
unsafe impl Send for FooServiceDependencies {}
unsafe impl Sync for FooServiceDependencies {}

impl FooServiceDeps for FooServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type FooDao = FooDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type FooService = FooServiceImpl<FooServiceDependencies>;

// Then instantiate and pass to RestState
```

---

## 5. Frontend (Dioxus + Tailwind)

### API Client (`api.rs`)
```rust
pub async fn get_foos(config: &Config) -> Result<Vec<FooTO>, reqwest::Error> {
    let url = format!("{}/foos", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    response.json().await
}

pub async fn create_foo(config: &Config, foo: FooTO) -> Result<FooTO, reqwest::Error> {
    let url = format!("{}/foos", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&foo).send().await?;
    response.error_for_status_ref()?;
    response.json().await
}

pub async fn update_foo(config: &Config, foo: FooTO) -> Result<FooTO, reqwest::Error> {
    let url = format!("{}/foos/{}", config.backend, foo.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&foo).send().await?;
    response.error_for_status_ref()?;
    response.json().await
}

pub async fn delete_foo(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    let url = format!("{}/foos/{}", config.backend, id);
    reqwest::Client::new().delete(url).send().await?.error_for_status_ref()?;
    Ok(())
}
```

### Global State (`state/foo.rs`)
```rust
#[derive(Clone, Default)]
pub struct FooState {
    pub items: Vec<FooTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub filter_query: String,
}

pub static FOOS: GlobalSignal<FooState> = GlobalSignal::new(FooState::default);
```

### Service (`service/foo.rs`)
```rust
pub async fn refresh_foos() {
    let config = CONFIG.read().clone();
    FOOS.write().loading = true;
    match api::get_foos(&config).await {
        Ok(items) => { FOOS.write().items = items; FOOS.write().error = None; }
        Err(e) => { FOOS.write().error = Some(format!("{}", e)); }
    }
    FOOS.write().loading = false;
}
```

### Page Pattern (`page/foos.rs`)
```rust
#[component]
pub fn Foos() -> Element {
    let i18n = use_i18n();
    let nav = use_navigator();

    use_effect(move || { spawn(async move { refresh_foos().await; }); });

    rsx! {
        div { class: "flex flex-col min-h-screen",
            TopBar {}
            div { class: "flex-1 container mx-auto px-4 py-8",
                h1 { class: "text-3xl font-bold mb-6", {i18n.t(Key::Foos)} }
                // List, filters, create button...
            }
        }
    }
}
```

### Component Pattern (`component/foo_form.rs`)
```rust
#[component]
pub fn FooForm(foo_id: Option<Uuid>) -> Element {
    let i18n = use_i18n();
    let nav = use_navigator();
    let mut foo = use_signal(|| FooTO::default());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load existing data
    use_effect(move || {
        if let Some(id) = foo_id {
            spawn(async move {
                loading.set(true);
                let config = CONFIG.read().clone();
                match api::get_foo(&config, id).await {
                    Ok(data) => { *foo.write() = data; }
                    Err(e) => { error.set(Some(format!("{}", e))); }
                }
                loading.set(false);
            });
        }
    });

    // Save handler
    let save = move |_| {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            let data = foo.read().clone();
            let result = if data.id.is_some() {
                api::update_foo(&config, data).await
            } else {
                api::create_foo(&config, data).await
            };
            match result {
                Ok(_) => nav.push(Route::Foos {}),
                Err(e) => error.set(Some(format!("{}", e))),
            }
            loading.set(false);
        });
    };

    rsx! { /* form with inputs, save/cancel buttons */ }
}
```

### Routing
```rust
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/foos")]
    Foos {},
    #[route("/foos/:id")]
    FooDetails { id: String },
}
```

### i18n
- Add keys to `Key` enum
- Add translations in `en.rs`, `de.rs`
- Use: `i18n.t(Key::FooName)`

### Tailwind Common Classes
- Layout: `flex flex-col min-h-screen`, `container mx-auto px-4 py-8`
- Headings: `text-3xl font-bold mb-6`
- Buttons: `px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700`
- Inputs: `w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500`
- Tables: `w-full` with `border-b hover:bg-gray-50` rows
- Cards: `bg-white rounded-lg shadow`

---

## 6. Testing Patterns

### Unit Tests (Service Layer with Mocks)
- `#[automock]` on DAO/Service traits generates `MockFooDao`
- Set expectations: `mock_dao.expect_all().returning(|| Ok(Arc::from([...])))`
- Inject mocks into service via `FooServiceImpl::new(mock_dao, ...)`

### E2E Tests (Full HTTP Server)
- `test_server.rs`: binds to random port, starts Axum server
- Use `reqwest::Client` for real HTTP calls
- In-memory SQLite database per test for isolation

### Integration Tests (REST Layer)
- Mock services implementing the service trait
- Start test server with mock rest state
- Assert HTTP status codes and response bodies

---

## 7. Cross-Cutting Concerns

### Authentication Flow
```
Request -> CookieManager -> Sessions -> OIDC Auth -> RegisterSession -> ContextExtractor -> ForbidUnauthenticated -> Handler
```

### Feature Flags
- `mock_auth` (default): `MockContext`, no external auth
- `oidc`: Full OIDC flow with `AuthenticatedContext`

### Error Mapping
```
DaoError -> ServiceError -> RestError -> HTTP Status
NotFound    EntityNotFound  NotFound    404
Database    DataAccess      Internal    500
Conflict    -               -           409
-           ValidationError BadRequest  400/422
-           PermissionDenied Unauthorized 401
```

### Transaction Pattern
```rust
let tx = self.transaction_dao.use_transaction(tx).await?;
// ... do work ...
self.transaction_dao.commit(tx).await?;
```

`use_transaction(None)` creates a new tx, `use_transaction(Some(tx))` reuses existing.
