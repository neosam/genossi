# Architecture Research

**Domain:** GV-Anwesenheits-Tracking als neuer Bounded Context in Genossi
**Researched:** 2026-05-01
**Confidence:** HIGH (basiert auf gemappter Bestands-Architektur, nicht auf externer Recherche)

## Standard Architecture

### System Overview — Wo die GV-Funktionalität in der bestehenden Layered Architecture sitzt

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                         Frontend (Dioxus WASM)                           │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────────┐ │
│  │ page/assembly_   │  │ page/qr_redeem.rs│  │ page/attendance_       │ │
│  │ list.rs          │  │ (Helfer-Login    │  │ helper.rs              │ │
│  │ (Vorstand)       │  │  per QR-URL)     │  │ (gemeinsame View für   │ │
│  │                  │  │                  │  │  Helfer + Vorstand)    │ │
│  └────┬─────────────┘  └────┬─────────────┘  └────┬───────────────────┘ │
│       │                     │                     │                      │
│       └─────────┬───────────┴────────┬────────────┘                      │
│                 ▼                    ▼                                    │
│   ┌────────────────────────┐  ┌────────────────────────────────────┐    │
│   │ component/assembly_*   │  │ component/attendance_row.rs        │    │
│   │ component/qr_card.rs   │  │ component/attendance_search.rs     │    │
│   │ component/live_counter │  │ component/attendance_header.rs     │    │
│   └────────────────────────┘  └────────────────────────────────────┘    │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │ HTTP REST (Axum)
┌──────────────────────────────────▼─────────────────────────────────────┐
│                          REST Layer (genossi_rest)                      │
│  ┌─────────────────────┐  ┌─────────────────────┐                      │
│  │ assembly.rs         │  │ helper_session.rs   │                      │
│  │ POST /api/assembly  │  │ POST /api/helper-   │                      │
│  │ PUT  /api/assembly/ │  │   session/redeem    │                      │
│  │   :id (close)       │  │ (Pre-Token einlösen)│                      │
│  │ POST /api/assembly/ │  └─────────────────────┘                      │
│  │   :id/pre-token     │  ┌─────────────────────┐                      │
│  │ GET  /api/assembly/ │  │ attendance.rs       │                      │
│  │   :id/stats (Live)  │  │ GET /api/attendance/│                      │
│  └─────────────────────┘  │   :assembly_id/     │                      │
│                            │   members           │                      │
│  auth_middleware.rs        │ POST /api/         │                      │
│  ─ erweitert um            │   attendance       │                      │
│    Helper-Cookie-Pfad      │ DELETE /api/       │                      │
│                            │   attendance/:id   │                      │
│                            └─────────────────────┘                      │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼─────────────────────────────────────┐
│                       Service Layer (genossi_service_impl)              │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌────────────────┐ │
│  │ AssemblyService     │  │ HelperSessionService│  │ Attendance     │ │
│  │ ─ create_assembly   │  │ ─ create_pre_token  │  │ Service        │ │
│  │ ─ close_assembly    │  │ ─ redeem_pre_token  │  │ ─ list_members │ │
│  │   (triggert Helper- │  │   (One-Time-Use)    │  │   _for_helper  │ │
│  │    Session-Invalid.)│  │ ─ verify_helper_    │  │ ─ mark_present │ │
│  │ ─ get_stats         │  │   session(assembly) │  │ ─ unmark       │ │
│  └────────┬────────────┘  └────────┬────────────┘  └───────┬────────┘ │
│           │                        │                       │           │
│           └────────────────────────┴───────────────────────┘           │
│                                    │                                    │
│  Permission-Erweiterung:                                                │
│  ─ AuthContext::Helper { session_id, assembly_id }                      │
│  ─ neuer Privilege "attendance_helper" — beschränkt auf Helfer-View     │
│  ─ existing "admin" privilege gilt zusätzlich für Vorstand              │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼─────────────────────────────────────┐
│                        DAO Layer (genossi_dao + impl_sqlite)            │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌────────────────┐ │
│  │ AssemblyDao         │  │ HelperPreTokenDao   │  │ Attendance     │ │
│  │ ─ create / update   │  │ ─ create_pre_token  │  │ Dao            │ │
│  │ ─ find_by_id        │  │ ─ consume_pre_token │  │ ─ create       │ │
│  │ ─ dump_all          │  │   (atomic check+    │  │ ─ update       │ │
│  │ AssemblyEntity:     │  │    update)          │  │   (soft-       │ │
│  │ id, title, date,    │  │ HelperPreToken:     │  │    delete=     │ │
│  │ status (Open/Closed)│  │ id, assembly_id,    │  │    austragen)  │ │
│  │ + std (created/del/ │  │ memo_name, token_   │  │ ─ list_by_     │ │
│  │   version)          │  │   hash, consumed,   │  │   assembly     │ │
│  │ ─ KEIN Auditable    │  │ session_id (FK)     │  │ Attendance     │ │
│  │                     │  │ ─ KEIN Auditable    │  │ Entity:        │ │
│  │                     │  │                     │  │ id, assembly_  │ │
│  │                     │  │                     │  │   id, member_  │ │
│  │                     │  │                     │  │   id, marked_  │ │
│  │                     │  │                     │  │   by, marked_  │ │
│  │                     │  │                     │  │   at           │ │
│  │                     │  │                     │  │ + (deleted=    │ │
│  │                     │  │                     │  │   austragen)   │ │
│  └─────────────────────┘  └─────────────────────┘  └────────────────┘ │
│                                                                         │
│  user_session-Tabelle (existiert bereits, wird wiederverwendet):        │
│  ─ session-claims-Feld speichert {"assembly_id": "...", "kind":         │
│    "helper"}                                                            │
│  ─ expires_at = assembly.close_at (gesetzt beim assembly close)         │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼─────────────────────────────────────┐
│                              SQLite                                      │
│  ┌──────────┐  ┌────────────────┐  ┌────────────┐  ┌────────────────┐ │
│  │ assembly │  │ helper_pre_    │  │ attendance │  │ user_session   │ │
│  │          │  │ token          │  │            │  │ (existing)     │ │
│  └──────────┘  └────────────────┘  └────────────┘  └────────────────┘ │
│  Migration: 20260501000000_create_assembly_tables.sql                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Komponente | Verantwortung | Implementation |
|-----------|---------------|---------------|
| **AssemblyDao** | Lebenszyklus der GV-Entität (anlegen, schließen, abfragen) | Trait in `genossi_dao/src/assembly.rs`, Impl in `genossi_dao_impl_sqlite/src/assembly.rs` |
| **HelperPreTokenDao** | Speichert pre-tokens (Hash), führt One-Time-Use-Konsum atomar durch | `genossi_dao/src/helper_pre_token.rs` + sqlite impl |
| **AttendanceDao** | Join-Tabelle Assembly ↔ Member, soft-delete als „Austragen" | `genossi_dao/src/attendance.rs` + impl |
| **AssemblyService** | Permission-Check (admin), close_assembly invalidiert alle Helper-Sessions dieser GV via `SessionService::revoke_all_for_user` über zugehörige Token-IDs | `genossi_service_impl/src/assembly.rs` |
| **HelperSessionService** | Pre-Token erzeugen (für Vorstand), Pre-Token redeemen (für Helfer): erzeugt `UserSession` mit `claims = {"kind": "helper", "assembly_id": ...}` und `expires_at` ans GV-Ende gebunden | `genossi_service_impl/src/helper_session.rs` |
| **AttendanceService** | Reduced-Member-View (nur Mitgliedsnummer/Name/Titel/Anrede), idempotente Markierung, Permission-Check via `AuthContext::Helper { assembly_id == request_assembly_id }` ODER `admin` | `genossi_service_impl/src/attendance.rs` |
| **AuthContext-Erweiterung** | Neuer Variant `Helper { session_id, assembly_id }` neben `Mock` und `Oidc` | `genossi_service/src/auth_types.rs` (modifiziert) |
| **REST Handlers** | Routing, ISO8601-Serialisierung, Utoipa-Schemas, Helper-Cookie setzen nach Pre-Token-Redemption | `genossi_rest/src/{assembly,helper_session,attendance}.rs` |
| **Frontend Components** | `AttendanceRow`, `AttendanceSearch`, `AttendanceHeader`, `LiveCounter`, `QrCard` — alle in `genossi-frontend/src/component/` | Component-First-Prinzip |
| **Frontend Pages** | `assembly_list.rs`, `assembly_detail.rs` (Vorstand), `qr_redeem.rs` (Pre-Token-URL → Cookie), `attendance_helper.rs` (gemeinsam für Helfer + Vorstand) | `genossi-frontend/src/page/` |

## Recommended Project Structure

Folgt 1:1 der bestehenden Genossi-Konvention (siehe `.planning/codebase/STRUCTURE.md`):

```
genossi_dao/src/
├── assembly.rs                    # AssemblyEntity, AssemblyDao trait, AssemblyStatus enum (Open/Closed)
├── helper_pre_token.rs            # HelperPreTokenEntity, HelperPreTokenDao trait
└── attendance.rs                  # AttendanceEntity, AttendanceDao trait

genossi_dao_impl_sqlite/src/
├── assembly.rs                    # AssemblyDaoImpl (sqlx-Queries, BLOB-UUID, ISO8601)
├── helper_pre_token.rs            # HelperPreTokenDaoImpl mit atomic consume_pre_token()
└── attendance.rs                  # AttendanceDaoImpl

genossi_service/src/
├── assembly.rs                    # AssemblyService trait
├── helper_session.rs              # HelperSessionService trait
└── attendance.rs                  # AttendanceService trait

genossi_service_impl/src/
├── assembly.rs                    # AssemblyServiceImpl mit close_assembly() → Session-Invalidation
├── helper_session.rs              # HelperSessionServiceImpl (pre-token CRUD, redeem-Logik)
└── attendance.rs                  # AttendanceServiceImpl mit reduced-View + Permission-Check

genossi_rest/src/
├── assembly.rs                    # POST/PUT/GET /api/assembly + /api/assembly/:id/stats
├── helper_session.rs              # POST /api/helper-session/redeem (Pre-Token einlösen, Cookie setzen)
└── attendance.rs                  # GET /api/attendance/:assembly_id/members + POST/DELETE /api/attendance/:assembly_id/:member_id

genossi_rest_types/src/lib.rs       # ergänzen um AssemblyTO, HelperPreTokenTO,
                                    # AttendanceMemberTO (reduced view!), AttendanceStatsTO

genossi-frontend/src/component/
├── assembly_card.rs               # GV-Karte für Liste
├── assembly_form.rs               # GV-Anlegen-Formular
├── qr_card.rs                     # QR-Code-Anzeige + Memo-Name
├── live_counter.rs                # X von Y anwesend (refresh-only, kein SSE)
├── attendance_header.rs           # GV-Titel + Counter + Schließen-Button
├── attendance_row.rs              # Einzeiler: Mitgliedsnummer / Name / Anrede / Toggle
└── attendance_search.rs           # Suchfeld (filtert lokale Liste, kein API-Roundtrip)

genossi-frontend/src/page/
├── assembly_list.rs               # Vorstand: alle GVs sehen (Open + Closed)
├── assembly_detail.rs             # Vorstand: GV bearbeiten, QR-Codes erzeugen, Live-Counter
├── qr_redeem.rs                   # Helfer: nimmt URL-Token entgegen, erzwingt Redemption
└── attendance_helper.rs           # GEMEINSAME View für Helfer + Vorstand (nutzt selbe Components)

migrations/sqlite/
└── 20260501000000_create_assembly_tables.sql   # assembly + helper_pre_token + attendance
```

### Structure Rationale

- **Drei separate Entitäten statt einer monolithischen `gv.rs`:** Folgt der Genossi-Praxis (Member, MemberAction, MemberDocument sind ebenfalls als drei Files getrennt). Ermöglicht unabhängige Tests pro Aggregat-Wurzel und macht den Service-Schnitt klarer.
- **Wiederverwendung der `user_session`-Tabelle statt eigener helper_session-Tabelle:** Das bestehende `SessionService`-Trait (`genossi_service/src/session.rs`) hat bereits ein `claims`-Feld als JSON-String mit dem Kommentar „Used for inventur token login auto-registration flow" — exakt das benötigte Muster. Die Helper-Session ist eine `UserSession` mit `claims = {"kind":"helper","assembly_id":"<uuid>"}`. Damit muss **kein neues Cookie-/Token-System** gebaut werden; das bestehende Auth-Middleware funktioniert weiter.
- **`helper_pre_token` ist eine eigene Tabelle (NICHT user_session):** Pre-Token und aktive Session sind unterschiedliche Lebensdauer-Stufen. Pre-Token = einmalig einlösbar, Session = aktiv bis GV-Ende. Trennung erlaubt es, gebrauchte Pre-Token zu Audit-Zwecken (Memo-Name) zu erhalten, ohne das Session-Schema zu erweitern.
- **`attendance.rs` als REST-Datei (nicht in `member.rs`):** Anwesenheit ist nicht Teil des Member-Aggregats — ein Member kann an N GVs teilnehmen. Der REST-Pfad `/api/attendance/:assembly_id/...` macht den Aggregat-Bezug explizit.
- **`attendance_helper.rs` als gemeinsame Page (statt zwei separater Pages für Helfer und Vorstand):** Bewusste Entscheidung des Projekts (PROJECT.md: „Vorstand kann die Helfer-Ansicht ohne QR-Code direkt aus seiner regulären Anmeldung heraus öffnen"). Eine Page, zwei Auth-Pfade — kein UI-Duplikat. Authorization passiert im Service: sowohl `AuthContext::Helper { assembly_id == X }` als auch `admin`-Privilege akzeptiert.

## Architectural Patterns

### Pattern 1: Aggregate Boundary für Assembly

**Was:** Assembly ist eigene Aggregat-Wurzel mit Lifecycle (Open → Closed). Member bleibt unabhängiges Aggregat. Die Verbindung läuft ausschließlich über die `attendance`-Join-Tabelle.

**Wann nutzen:** Hier verpflichtend — User-Entscheidung in PROJECT.md (Key Decision: „GV als eigene Entität (`Assembly`) statt globalem Zustand"; „Anwesenheit als Join-Tabelle (`AssemblyAttendance`) statt Member-Flag").

**Trade-offs:**
- ➕ Mehrere GVs parallel/historisch möglich; saubere Protokoll-Historie
- ➕ Member-Schema bleibt unverändert — kein Risiko für bestehende auditierte Operationen
- ➖ Eine Anwesenheits-Markierung kostet einen JOIN beim Lesen (akzeptabel für GV-Skala: O(100–1000) Mitglieder)

**Beispiel — DAO-Trait:**
```rust
// genossi_dao/src/assembly.rs
pub struct AssemblyEntity {
    pub id: Uuid,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
    // domain fields:
    pub title: Arc<str>,
    pub date: PrimitiveDateTime,
    pub status: AssemblyStatus,        // Open | Closed
    pub closed_at: Option<PrimitiveDateTime>,
}

#[async_trait]
pub trait AssemblyDao {
    type Transaction;
    async fn create(&self, entity: &AssemblyEntity, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn update(&self, entity: &AssemblyEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<AssemblyEntity>, DaoError>;
}
// KEIN impl Auditable — Anwesenheits-Aggregat ist explizit aus Audit-Scope ausgeschlossen.
```

### Pattern 2: One-Time-Use Pre-Token mit atomarem Konsum

**Was:** Vorstand erzeugt einen pre-token (UUID v4, hashed gespeichert). QR-Code-URL trägt das *Klartext*-Token als Pfad-Parameter. Beim Redeem-Endpoint wird in einer Transaktion: (1) Token-Hash gesucht, (2) `consumed_at IS NULL` geprüft, (3) `consumed_at = now()` gesetzt, (4) `UserSession` mit `claims` erzeugt, (5) `helper_pre_token.session_id` gesetzt. Schlägt einer der Schritte fehl → ROLLBACK. Antwort enthält Set-Cookie mit `session_id`.

**Wann nutzen:** Wenn ein einmaliger, weitergebbarer aber pro Token nur einmal nutzbarer Login benötigt wird. Standard für Magic-Link, Invite-Token, Inventur-Token (siehe Kommentar `ensure_user_and_create_session_with_claims`).

**Trade-offs:**
- ➕ Token-Hash nur in DB → bei DB-Leak kein Replay
- ➕ Atomic Konsum verhindert Race (zwei Helfer scannen denselben QR gleichzeitig)
- ➕ Session erbt `expires_at` = Assembly-Schließzeitpunkt → automatisches Ablaufen
- ➖ Erfordert Transaktions-Konsistenz zwischen pre_token-Tabelle und user_session-Tabelle (gleiche SQLite-DB → kein Issue)

**Beispiel — Service:**
```rust
// genossi_service_impl/src/helper_session.rs
async fn redeem_pre_token(&self, plain_token: &str) -> Result<RedemptionResultTO, ServiceError> {
    let tx = self.transaction_dao.transaction().await?;
    let token_hash = sha256(plain_token);

    let pre_token = self.pre_token_dao
        .find_by_hash(token_hash, tx.clone()).await?
        .ok_or(ServiceError::Unauthorized)?;

    if pre_token.consumed_at.is_some() {
        return Err(ServiceError::Unauthorized); // already used
    }

    let assembly = self.assembly_dao
        .find_by_id(pre_token.assembly_id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(pre_token.assembly_id))?;
    if assembly.status != AssemblyStatus::Open {
        return Err(ServiceError::Unauthorized);
    }

    // Synthetic user_id = pre_token.id (deterministic, isolated from real users)
    let synthetic_user = format!("helper:{}", pre_token.id);
    let claims = serde_json::json!({"kind":"helper","assembly_id":assembly.id}).to_string();
    let expires_at_secs = (assembly.date + Duration::hours(24)).unix_timestamp(); // hard upper bound

    let session = self.session_service
        .create_session_with_claims(&synthetic_user, expires_at_secs - now(), Some(claims)).await?;

    let mut consumed = pre_token.clone();
    consumed.consumed_at = Some(now());
    consumed.session_id = Some(session.session_id.clone());
    self.pre_token_dao.update(&consumed, "helper-session-redeem", tx.clone()).await?;

    self.transaction_dao.commit(tx).await?;
    Ok(RedemptionResultTO { session_id: session.session_id, assembly_id: assembly.id })
}
```

### Pattern 3: Polymorphes AuthContext für „eine View, zwei Auth-Backends"

**Was:** Bestehender `AuthContext`-Enum (`genossi_service/src/auth_types.rs`) wird um Variante `Helper { session_id, assembly_id }` erweitert. Die `extract_auth_context`-Logik im SessionService liest `claims.kind` und gibt entweder `AuthContext::Oidc(...)` (Vorstand) oder `AuthContext::Helper{...}` (Helfer) zurück. AttendanceService akzeptiert beide.

**Wann nutzen:** Wenn dasselbe UI/Feature von verschiedenen Authentifizierungs-Pfaden aufgerufen wird, aber unterschiedliche Privilege-Sätze hat.

**Trade-offs:**
- ➕ Frontend muss nicht zwei API-Pfade unterhalten; eine `/api/attendance/...`-Route
- ➕ Authorization bleibt zentralisiert in `AttendanceServiceImpl::check_can_mark_attendance(ctx, assembly_id)`
- ➖ `AuthContext`-Enum wird größer; alle bestehenden `match`-Stellen müssen erweitert werden (clippy fängt das auf)

**Beispiel — Permission-Check:**
```rust
// genossi_service_impl/src/attendance.rs
fn assert_can_access_assembly(&self, ctx: &AuthContext, assembly_id: Uuid) -> Result<(), ServiceError> {
    match ctx {
        AuthContext::Helper { assembly_id: helper_assembly, .. } if *helper_assembly == assembly_id => Ok(()),
        AuthContext::Helper { .. } => Err(ServiceError::PermissionDenied), // Helfer einer ANDEREN GV
        AuthContext::Oidc(_) | AuthContext::Mock(_) => {
            // Vorstand braucht admin-privilege; check_permission delegiert
            // an PermissionService → existing pattern
            Ok(())
        }
    }
}
```

### Pattern 4: Refresh-Only Live-Counter (kein SSE/WebSocket)

**Was:** Frontend ruft im Polling-Intervall (z. B. 5s, oder bei jedem Suchvorgang) `GET /api/assembly/:id/stats` auf. Backend gibt `{present: 47, total: 152}` zurück. Vorstand sieht Live-Counter, Helfer-Aktionen werden bei nächstem Refresh sichtbar.

**Wann nutzen:** Wenn Echtzeit-Anforderungen lasch sind (PROJECT.md: „Sync zwischen Helfern nur bei Refresh, kein Live-Push") — vermeidet komplette SSE/WebSocket-Infrastruktur.

**Trade-offs:**
- ➕ Keine neue Infrastruktur; passt in bestehendes REST-Pattern
- ➕ Doppel-Abhaken ist akzeptabel (Anwesend-Markierung ist idempotent: `UNIQUE(assembly_id, member_id) WHERE deleted IS NULL`)
- ➖ N Helfer × Polling = N Requests/Sekunde — bei einer GV mit max ~10 Helfern unproblematisch

**Beispiel:**
```rust
// genossi_rest/src/assembly.rs
async fn get_assembly_stats(/* ... */) -> Result<AttendanceStatsTO, RestError> {
    let stats = service.get_stats(assembly_id, ctx).await?;
    // counters, no member list
    Ok(AttendanceStatsTO { present: stats.present, total: stats.total })
}
```

## Data Flow

### (a) GV Anlegen (Vorstand)

```
Browser (Vorstand)
   │ POST /api/assembly  Body: {title, date}  Cookie: oidc_session
   ▼
auth_middleware → AuthContext::Oidc(...)
   ▼
assembly.rs::create_assembly()
   ▼
AssemblyServiceImpl::create()
   ├─ permission.check_permission("admin", ctx)
   ├─ uuid_service.new_uuid() → assembly_id
   ├─ tx = transaction_dao.transaction()
   ├─ assembly_dao.create(entity, tx)
   └─ tx.commit()
   ▼
201 Created  Body: AssemblyTO
```

### (b) QR-Code für Helfer erzeugen (Vorstand)

```
Browser (Vorstand, assembly_detail.rs)
   │ POST /api/assembly/:id/pre-token  Body: {memo_name: "Anna"}
   ▼
HelperSessionServiceImpl::create_pre_token(assembly_id, "Anna", ctx)
   ├─ permission.check_permission("admin", ctx)
   ├─ check assembly.status == Open
   ├─ plain_token = uuid_service.new_uuid().to_string()  // 36-char base
   ├─ token_hash = sha256(plain_token)
   ├─ pre_token_dao.create({id, assembly_id, memo_name, token_hash, ...}, tx)
   └─ commit
   ▼
201 Created  Body: {pre_token_id, redeem_url: "https://genossi/qr/<plain_token>", memo_name}
   ▼
Frontend rendert QrCard-Component mit redeem_url als QR
```

### (c) Helfer-Login per QR-Scan

```
Helfer öffnet QR-URL https://genossi/qr/<plain_token>
   │
   ▼
Frontend Page qr_redeem.rs lädt
   │ POST /api/helper-session/redeem  Body: {token: "<plain_token>"}
   ▼
helper_session.rs::redeem()
   ▼
HelperSessionServiceImpl::redeem_pre_token(plain_token)
   ├─ tx = transaction.begin()
   ├─ pre_token_dao.find_by_hash(sha256(token))
   │    └─ NOT FOUND → 401
   ├─ if pre_token.consumed_at.is_some() → 401  (already used)
   ├─ assembly_dao.find_by_id(pre_token.assembly_id)
   │    └─ status != Open → 401
   ├─ session_id = uuid()
   ├─ session_service.create_session_with_claims(
   │     user_id = "helper:<pre_token.id>",
   │     expires_in = (assembly.date_end_of_day - now),
   │     claims = '{"kind":"helper","assembly_id":"..."}'
   │   )
   ├─ pre_token.consumed_at = now()
   ├─ pre_token.session_id = session_id
   ├─ pre_token_dao.update(pre_token, ...)
   └─ tx.commit()
   ▼
200 OK  Body: {assembly_id, assembly_title}
       Set-Cookie: session_id=<uuid>; HttpOnly; Secure; SameSite=Strict
   ▼
Frontend redirect → attendance_helper.rs?assembly_id=...
```

### (d) Anwesenheit markieren (Helfer ODER Vorstand)

```
Browser (Helfer ODER Vorstand)
   │ POST /api/attendance/:assembly_id/:member_id  Cookie: session_id=<...>
   ▼
auth_middleware
   ├─ liest cookie, extract_context_from_headers()
   ├─ session_service.extract_auth_context(session_id)
   │    └─ liest UserSession.claims, parse JSON
   │    └─ wenn claims.kind == "helper" → AuthContext::Helper{...}
   │    └─ sonst                          → AuthContext::Oidc(...)
   └─ Request.extensions_mut().insert(auth_context)
   ▼
attendance.rs::mark_present(assembly_id, member_id)
   ▼
AttendanceServiceImpl::mark_present()
   ├─ assert_can_access_assembly(ctx, assembly_id)  ← Pattern 3
   ├─ check assembly.status == Open
   ├─ existing = attendance_dao.find_by_assembly_member(assembly_id, member_id, tx)
   ├─ if existing.is_some() && existing.deleted.is_none() → 200 (idempotent)
   ├─ if existing.is_some() && existing.deleted.is_some() → update: deleted=None
   ├─ else → attendance_dao.create({id, assembly_id, member_id, marked_by: ctx.actor_id(), marked_at: now})
   └─ commit
   ▼
200 OK  (KEIN Audit-Log-Eintrag — explizit aus Scope; PROJECT.md Out-of-Scope)
```

### (e) GV schließen → Helper-Sessions invalidieren

```
Browser (Vorstand)
   │ PUT /api/assembly/:id  Body: {status: "Closed"}
   ▼
AssemblyServiceImpl::close_assembly(id, ctx)
   ├─ permission.check_permission("admin", ctx)
   ├─ tx = transaction.begin()
   ├─ assembly = assembly_dao.find_by_id(id, tx)
   ├─ assembly.status = Closed
   ├─ assembly.closed_at = Some(now())
   ├─ assembly_dao.update(assembly, "assembly-close", tx)
   ├─ // Cascade: alle helper_pre_tokens dieser Assembly mit gesetzter session_id invalidieren
   ├─ tokens = pre_token_dao.list_by_assembly(id, tx)
   ├─ for token in tokens.filter(t => t.session_id.is_some()):
   │     session_service.invalidate_session(&token.session_id)
   └─ tx.commit()
   ▼
200 OK
```

**Wichtig:** Auch ohne expliziten Cascade laufen Helper-Sessions automatisch ab, weil `expires_at` beim Anlegen der Session ans GV-Datums-Ende gebunden wird. Der Cascade in `close_assembly()` ist ein **Safety-Net** für vorzeitiges Schließen vor dem Tagesende.

## Build-Reihenfolge (Phase-Empfehlung für den Roadmap-Schritt)

Die folgende Reihenfolge minimiert Lock-In und macht jeden Schritt für sich testbar:

1. **Phase A — Assembly-Aggregat (DAO → Service → REST)**
   Abhängigkeit: keine (außer existing TransactionDao, UuidService, PermissionService).
   - DAO: `genossi_dao/src/assembly.rs` + `genossi_dao_impl_sqlite/src/assembly.rs`
   - Migration: `assembly`-Tabelle
   - Service: `AssemblyServiceImpl::create/update/get_all/find_by_id/close_assembly` (close ohne Cascade noch — Cascade kommt in Phase C)
   - REST: `POST/PUT/GET /api/assembly`
   - DI-Wiring in `genossi_bin/src/lib.rs`
   - Tests: Unit-Tests Service mit MockDaos, e2e POST/GET über Test-Server.
   - Liefert: Vorstand kann GVs anlegen und schließen — Frontend noch nicht erforderlich.

2. **Phase B — Pre-Token + HelperSession (DAO → Service → REST)**
   Abhängigkeit: Assembly-Aggregat (für FK), bestehender SessionService.
   - DAO: `helper_pre_token` mit `find_by_hash`, atomic `consume_pre_token`
   - Migration: `helper_pre_token`-Tabelle
   - Service: `HelperSessionServiceImpl` mit `create_pre_token`, `redeem_pre_token`
   - **AuthContext-Erweiterung:** Variante `Helper { session_id, assembly_id }` in `auth_types.rs`; `SessionService::extract_auth_context` parst `claims.kind`
   - REST: `POST /api/assembly/:id/pre-token`, `POST /api/helper-session/redeem` (setzt Cookie)
   - Tests: Pre-Token Race-Test (zwei parallele redeems → genau einer erfolgreich); abgelaufene Sessions; Helper-AuthContext-Extraction.
   - Liefert: Helfer können sich per QR-URL einloggen — eine Cookie-Session existiert, kann aber noch keine Anwesenheit markieren.

3. **Phase C — Attendance-Aggregat + Cascade-Invalidation**
   Abhängigkeit: Assembly + HelperSession.
   - DAO: `attendance` als Join (UNIQUE(assembly_id, member_id) WHERE deleted IS NULL)
   - Migration: `attendance`-Tabelle
   - Service: `AttendanceServiceImpl::list_members_for_helper` (reduced view via SQL-Projection auf `member.member_number/name/title/salutation`), `mark_present`, `unmark`, `get_stats`
   - Permission-Logik mit polymorphem `AuthContext` (Pattern 3)
   - **Erweiterung von Phase A:** `AssemblyServiceImpl::close_assembly` cascade-invalidiert Helper-Sessions
   - REST: `GET /api/attendance/:assembly_id/members`, `POST/DELETE /api/attendance/:assembly_id/:member_id`, `GET /api/assembly/:id/stats`
   - Tests: Vorstand kann ohne Helper-Token markieren; Helper kann nur in eigener Assembly markieren; idempotent (zweimal POST → derselbe Zustand); Stats-Counter.
   - Liefert: Backend-API ist vollständig, alle 9 Active-Requirements aus PROJECT.md sind funktional bedient (Frontend-Bonus folgt).

4. **Phase D — Frontend-Components (Component-First)**
   Abhängigkeit: Backend-Phasen A–C abgeschlossen.
   - Reihenfolge: erst Components (`attendance_row`, `attendance_search`, `attendance_header`, `qr_card`, `live_counter`), dann Pages (`assembly_list`, `assembly_detail`, `qr_redeem`, `attendance_helper`).
   - Eine UI-Bibliothek-Erweiterung: `attendance_helper.rs` muss prüfen, ob aktueller AuthContext `Helper` oder `Oidc` ist (sichtbar im API-Response — `/api/session` liefert Mode), und dann die richtige Top-Bar zeigen (Vorstand: full nav; Helfer: nur „Logout" + Assembly-Title).
   - Live-Counter via `use_resource` mit Polling-Intervall (5s), Suchfeld filtert lokal (kein API-Roundtrip).

5. **Phase E (optional) — Hardening**
   - Rate-Limit auf `/api/helper-session/redeem` (z. B. tower-governor) gegen Token-Bruteforce
   - Audit-Timestamp für `assembly.closed_at` via bestehende `audit_timestamp`-Infrastruktur (siehe `genossi_service_impl/src/timestamp_worker.rs`) — gibt RFC 3161 Beweis für Protokoll
   - Excel-Export der Anwesenheitsliste über bestehende `MemberImportService`-Pattern (read-side)

**Kritische Build-Reihenfolge-Regeln:**
- **B vor C** ist zwingend, weil C die `AuthContext::Helper`-Variante nutzt
- **Cascade-Invalidation in close_assembly** kommt erst in Phase C (nicht A), weil der Cascade die HelperPreTokenDao kennt — vermeidet zirkuläre Service-Deps in Phase A
- **Frontend (D) erst nach Backend (A–C)** — die existierende Genossi-Konvention ist Backend-First (siehe `.planning/codebase/STRUCTURE.md`); Frontend nutzt API-Schemas via `genossi_rest_types`

## State Management

```
Backend
  ├─ user_session table (existing) — single source of truth für aktive Sessions (oidc + helper)
  ├─ assembly table — Aggregat-Wurzel, status (Open/Closed) treibt Cascade-Invalidation
  ├─ helper_pre_token table — One-Time-Use-Marker; consumed_at + session_id verbinden Pre-Token mit aktiver Session
  └─ attendance table — Join, soft-delete = ausgetragen

Frontend
  ├─ State hooks pro Page (Dioxus use_resource für Server-State)
  ├─ Helper-Session-State: Cookie-basiert (transparent), kein client-seitiges State-Management nötig
  └─ Live-Counter: use_resource mit Polling-Intervall, manueller Refresh bei Search-Submit
```

## Anti-Patterns

### Anti-Pattern 1: Helper-Session als komplett neues Cookie/Token-System bauen

**Was Leute tun:** Eigene `helper_session`-Tabelle, eigener Header `X-Helper-Token`, eigene Middleware-Pipeline neben der OIDC-Middleware.

**Warum falsch:** Verdoppelt die Auth-Codebase. Auth-Middleware in `genossi_rest/src/auth_middleware.rs` müsste an mehreren Stellen branchen. Bug-Fixes (Cookie-Flags, Timing-Attacks) müssten zweimal gemacht werden.

**Stattdessen:** Bestehendes `user_session`-Schema mit `claims`-JSON-Feld nutzen (existiert für genau diesen Zweck — siehe Kommentar im Code: „Used for inventur token login auto-registration flow"). Eine Variante in `AuthContext`-Enum ergänzen.

### Anti-Pattern 2: Plain-Token in DB speichern

**Was Leute tun:** `helper_pre_token.token` als plain UUID-String in der Tabelle speichern.

**Warum falsch:** DB-Backup-Leak = sofortiger Replay aller noch nicht eingelösten QR-Codes. Auch wenn das Window bis zur GV kurz ist — die Genossi-Codebase pflegt eine Hash-Chain für Audits, mit Plain-Token im DB ist diese Konsistenz-Story brüchig.

**Stattdessen:** SHA256-Hash speichern, Plain-Token nur in der QR-URL. `find_by_hash`-Lookup beim Redeem. Bei Memo-Anzeige im Vorstand wird **nicht** der Token gezeigt, sondern die Pre-Token-ID + Memo-Name + Status (consumed/active).

### Anti-Pattern 3: Helfer-View als zwei separate Pages bauen

**Was Leute tun:** `helper_attendance.rs` und `admin_attendance.rs` als zwei Pages mit ähnlichem RSX.

**Warum falsch:** Verstößt direkt gegen Component-First-Prinzip aus `genossi-frontend/CLAUDE.md` und das Memory-Item „Always use reusable components, never duplicate UI logic across pages". User hat dies in PROJECT.md explizit als Key Decision festgehalten („Helfer-View auch für Vorstand zugänglich (ohne QR), vermeidet UI-Duplikat").

**Stattdessen:** Eine Page `attendance_helper.rs`, zwei Auth-Pfade. Components sind oblivious zur Auth-Quelle. Auth-spezifische Differenzen (Top-Bar, Logout-Verhalten) liegen in der Page selbst, nicht in den Components.

### Anti-Pattern 4: Anwesenheit ans Member-Aggregat hängen

**Was Leute tun:** `member.attended_assembly_id` oder Liste `member.attended_assemblies` als Felder am Member.

**Warum falsch:** PROJECT.md Key Decision: „Anwesenheit als Join-Tabelle (`AssemblyAttendance`) statt Member-Flag" — Mehrfach-GV-Historie geht sonst verloren. Member-Schema-Migration triggert zudem Audit-Macros (Member ist auditiert!), während Attendance explizit aus Audit-Scope ausgeschlossen ist.

**Stattdessen:** `attendance`-Join-Tabelle. Member bleibt unverändert.

### Anti-Pattern 5: SSE/WebSocket „weil Live-Counter klingt nach Echtzeit"

**Was Leute tun:** axum-streams, tokio_stream, Broadcast-Channel für Anwesenheits-Events.

**Warum falsch:** PROJECT.md Out-of-Scope: „Live-Push zwischen Helfern (SSE/WebSocket) — Synchronisation nur bei Refresh/Suche". User hat das explizit ausgeschlossen, um Komplexität zu vermeiden.

**Stattdessen:** Polling im Frontend (`use_resource` mit Refresh-Intervall). Backend-Endpoint `GET /api/assembly/:id/stats` ist eine simple SQL-Aggregation.

## Integration Points

### Externe Abhängigkeiten

| Service | Integration-Pattern | Hinweise |
|---------|-------------------|---------|
| **Nextcloud OIDC** (existing) | unverändert; Vorstand authentifiziert sich gleich wie heute | Nichts neu — `AuthContext::Oidc` bleibt für Vorstand der einzige Pfad |
| **QR-Code-Rendering** | Frontend-only via `qrcode` crate (WASM-kompatibel: `qrcode = { version, default-features = false, features = ["svg"] }`) | Backend liefert nur die `redeem_url`-String, Frontend rendert SVG |
| **Tower-Sessions / tower-cookies** (existing) | unverändert; bestehender Cookie-Mechanismus für `session_id` setzt das Helper-Cookie genauso | Set-Cookie-Header in REST-Layer reicht aus |

### Interne Boundaries

| Boundary | Kommunikation | Hinweise |
|----------|--------------|---------|
| **Assembly ↔ HelperPreToken** | DAO-Layer FK-Constraint + Service-Layer `assembly_id`-Check | HelperPreToken kann nicht ohne existierende Assembly erzeugt werden; bei Assembly-Soft-Delete laufen pre_tokens ins Leere — akzeptabel, weil `redeem` zusätzlich `assembly.status == Open` prüft |
| **HelperSession ↔ user_session** | Service-Layer; HelperSessionService nutzt `SessionService::create_session_with_claims` | KEINE direkte DB-Interaktion mit user_session aus HelperSessionService — alles über das Trait, damit Mocking + Tests stabil bleiben |
| **AttendanceService ↔ MemberDao** | Read-only Projection beim `list_members_for_helper` | NICHT die volle MemberEntity zurückgeben — eigenes `AttendanceMemberTO` mit nur 4 Feldern, um DSGVO-Datenminimierung im Code sichtbar zu machen (siehe PROJECT.md Constraint „Datenschutz") |
| **AssemblyService ↔ HelperSessionService** | Cascade-Invalidation via Trait-Aufruf in `close_assembly` | Beide Services bekommen einander als Dep injiziert in `genossi_bin/src/lib.rs::RestStateImpl::new()`; Reihenfolge: HelperSessionService zuerst konstruieren, dann AssemblyService mit Reference |
| **AttendanceService ↔ AssemblyService** | Read-only `find_by_id` für Status-Check (Open/Closed) | Markierungen werden auf geschlossener GV abgelehnt → 409 Conflict |

## Sources

- `.planning/PROJECT.md` (Project context, Key Decisions, Out-of-Scope)
- `.planning/codebase/ARCHITECTURE.md` (Layered architecture, Audit-Pattern, Anti-Patterns)
- `.planning/codebase/STRUCTURE.md` (Directory layout, „Where to Add New Code"-Recipe)
- `.planning/codebase/CONVENTIONS.md` (Naming-Patterns, Error-Handling-Patterns, Audit-Macros-Vorgaben)
- `genossi_service/src/auth_types.rs` (AuthContext-Enum, UserSession.claims-Feld)
- `genossi_service/src/session.rs` (SessionService-Trait mit `create_session_with_claims` und Kommentar zur Inventur-Token-Pattern-Wiederverwendung)
- `genossi_rest/src/auth_middleware.rs` (extract_context_from_headers — Cookie + Bearer-Pfad ist bereits vorhanden)
- `/home/neosam/.claude/projects/-home-neosam-programming-rust-projects-genossi3/memory/MEMORY.md` (Component-First, OIDC = Nextcloud)

---
*Architecture research for: GV-Anwesenheits-Tracking als neuer Genossi-Domain-Bereich*
*Researched: 2026-05-01*
