# Phase 25: Application File Upload + Audited Carryover — Research

**Researched:** 2026-07-03
**Domain:** REST/Service/DAO + Filesystem-Storage + Audit-Cascade (Rust/Axum/SQLx/SQLite)
**Confidence:** HIGH (in-repo pattern trace; keine externen Libraries neu)

## Summary

Phase 25 baut ein Single-Slot-Antragsdokument an `Application` und übergibt es beim `confirm()` als auditiertes `MemberDocument` an das neue Mitglied. **Alle relevanten Bausteine existieren bereits im Repo** — Multipart-Upload, `DocumentStorage`-Trait, MIME-Allowlist, Audit-Macros, `APPLICATION_SERVICE_PROCESS`-Konstante, Transaktions-Klammer im bestehenden `confirm()`. Es ist ein **Muster-Kopieren mit Anpassungen**, kein Design-Neuland.

Kritische Fund-Punkte:
1. Die `member_document`-Tabelle hat die reine Basis-Schema-Form ohne partial-unique-index — für `application_documents` wird das Constraint neu eingeführt (SQLite unterstützt `CREATE UNIQUE INDEX ... WHERE deleted IS NULL` seit 3.8).
2. `DocumentStorage` hat **weder `copy` noch `rename`** — Move = `load` → `save` neuer Pfad → `delete` alter Pfad. Beim `save`/`load`-Fehler propagiert `?` den Fehler und triggert Tx-Rollback; **beim `delete` nach Commit** darf **kein** Rollback mehr passieren (best-effort + Warn-Log).
3. `StorageError` hat **keine `From`-Impl für `ServiceError`** — muss explizit gemappt werden (`.map_err(|e| ServiceError::InternalError(Arc::from(...)))` bzw. `StorageError::NotFound → ServiceError::EntityNotFound(app_id)`).
4. **CR-02 im bestehenden `confirm()`** (Zeile 289-297): `current_user_id()` läuft **vor** `check_permission()` — Phase 25 muss das mit-fixen (APDOC-02).
5. `application_documents`-URL: CONTEXT.md schreibt `/api/application/{id}/document` (singular). Die bestehende Application-Route ist unter `/api/applications` (plural) genestet. **Empfehlung:** unter `/api/applications/{id}/document` einhängen (Konsistenz mit bestehender Basis), Discussion-CONTEXT-Wortlaut ist Sinn-, nicht Pfad-verbindlich — im Plan explizit machen.

**Primary recommendation:** 1:1 Struktur-Kopie von `member_document.rs` (REST) + `member_document.rs` (DAO) + Trimming (kein `document_type`, keine `description`, kein `Auditable`, single-slot statt list). Erweiterung von `ApplicationServiceImpl::confirm()` innerhalb der bestehenden `use_transaction`-Klammer um Move + `audited_create!(MemberDocument)`. Alle drei neuen REST-Endpunkte **von Anfang an mit korrekter CR-02-Reihenfolge**, und dieselbe Umsortierung als Micro-Diff im bestehenden `confirm_application`-Service.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
(Verbatim aus CONTEXT.md `<decisions>` — die Zusammenfassung; volle Rationales im 25-CONTEXT.md)

1. **Single-Slot pro Application** — max 1 aktive `application_documents`-Zeile; DB-Constraint = unique partial index `WHERE deleted IS NULL`.
2. **Re-Upload = Replace-in-Place** — derselbe Record wird via `UPDATE` überschrieben (neuer `version`-UUID), physische Alt-Datei nach erfolgreichem Save neuer Datei gelöscht. Sequenz: save-new → update-DB → delete-old (delete best-effort). Keine Audit-Historie.
3. **Carryover-Semantik = Move (Ownership-Übergabe), NICHT Copy** — `application_documents`-Zeile wird beim `confirm()` soft-deleted, in derselben Tx wird ein auditiertes `MemberDocument` erzeugt (`DocumentType::Other`, Description `"Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"`). Datei wird physisch verschoben (`load` → `save` neuer Pfad → `delete` alter Pfad). **Weicht vom aktuellen APDOC-03-Wortlaut ab — REQUIREMENTS.md-Textfix ist Teil der Phase.**
4. **Atomicity + Rollback** — Storage-Move läuft innerhalb der bestehenden `use_transaction`-Klammer. Bei fehlender/beschädigter Datei propagiert `?` → Tx rollt zurück (kein Member, keine Actions, kein MemberDocument). `delete(old)` nach commit ist best-effort (Warn-Log). Antrag ohne Dokument → Schritt wird übersprungen.
5. **application_documents-Schema** — Minimal: `id`, `application_id`, `file_name`, `mime_type`, `relative_path`, `size`, `created`, `deleted`, `version`. **Kein** `document_type`, **keine** `description`, **kein** `Auditable`-Impl.
6. **MIME-Allowlist & Body-Limit aus MemberDocument wiederverwenden** — `allowed_extensions()`/`lookup_allowed_mime()` unverändert; Body-Limit = `MEMBER_DOCUMENT_BODY_LIMIT` (50 MB). Client-MIME wird verworfen, Server derived.
7. **3 REST-Endpunkte, admin-only** — `POST/GET/DELETE /api/application/{id}/document` (Multipart POST = Upload oder Replace). **CR-02:** `check_permission()` VOR `current_user_id()` an allen neuen Sites + Fix im bestehenden `confirm()`.
8. **Frontend `ApplicationDocumentSlot`-Component** — Leer: Upload-Button; Gefüllt: Dateiname/Größe/Datum + Download/Ersetzen/Löschen-Icons. Component-First: erst `genossi-frontend/src/component/` prüfen. Dioxus-Button: `r#type: "button"` + `onclick`.
9. **Test-Strategie (Grob)** — Unit-Service (Mock-Storage: happy, missing-file, save-fail, delete-fail), Integration-REST (admin vs. non-admin, MIME-reject, body-limit, replace), E2E (`Upload → confirm → MemberDocument sichtbar + application_documents.deleted != NULL + Audit-Row + Hashchain valid`), CR-02-Regressions-Test.

**Zusätzlich (aus `<prior_decisions>`):** Layered DAO/Service/REST + Trait-Boundaries, Soft-Delete + optimistic `version`-UUID, `audited_*!`-Macros für auditierte Entities, Component-First-Frontend, deutsche UI-Sprache, jj statt git.

### Claude's Discretion
- **URL-Segment** `/document` (Singular) vs. `/documents` (Konsistenz mit bestehender `member_document`-Nesting-Konvention): **Research-Empfehlung: `/document` (Singular) verwenden — reflektiert das Single-Slot-Modell semantisch korrekt**. Genestet unter `/api/applications/{id}/document` (Konsistenz mit bestehender Application-Route-Basis, siehe `<code_deep_dive>`).
- Response-Shape der drei Endpunkte: `POST` → 201 + `ApplicationDocumentTO`; `GET` → 200 + Datei-Body (Content-Disposition: attachment); `DELETE` → 204.
- Description-Format beim Carryover-MemberDocument: `"Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"` — deutsches Datumsformat, konsistent mit FMT-01 (Phase 22).
- Ob Frontend-Slot als neue Component-Datei oder inline in `application_detail.rs`: **Research-Empfehlung: neue Component `application_document_slot.rs`**, da der Slot 3 Zustände + 3 Interaktionen hat (Component-First bricht bei inline-RSX).

### Deferred Ideas (OUT OF SCOPE)
- Housekeeping-Job für verwaiste Application-Files (bei best-effort-delete-Fehler nach commit).
- Multi-File pro Application.
- Application-Detail „Historie" nach `confirm` (Anzeige „ursprünglich lag hier eine Datei").
- CR-02 projektweit als `gen_auth_admin!`-Helper extrahieren.
- Drag-and-Drop-Upload (MVP: klassischer File-Dialog).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APDOC-01 | Admin lädt Datei an `Application` hoch; über `DocumentStorage` im Filesystem gespeichert; Multipart + `DefaultBodyLimit` + MIME-Allowlist + UUID-Pfad | Reference-Handler `genossi_rest/src/member_document.rs:115-224`, Storage-Impl `genossi_service_impl/src/document_storage.rs`, Allowlist `genossi_service/src/member_document.rs:12-46` — alle direkt wiederverwendbar |
| APDOC-02 | Upload-Endpunkt admin-only + CR-02 Permission-Check-Ordering (`check_permission()` VOR `current_user_id()`) | Anti-Pattern lokalisiert: `genossi_service_impl/src/application.rs:289-297` (aktuelles `confirm()`), `genossi_service_impl/src/member_document.rs:61-69` (`upload()`); Fix-Muster = zwei Zeilen tauschen |
| APDOC-03 | Beim `confirm` wird das Dokument als auditiertes `MemberDocument` **kopiert/übernommen** in derselben Tx via `audited_create!` unter `APPLICATION_SERVICE_PROCESS`, `DocumentType::Other`. **Wortlaut-Update auf „übernommen (Ownership-Übergabe)" nötig — siehe `<requirements_wording_update>`.** | Bestehende `confirm()`-Tx `genossi_service_impl/src/application.rs:287-421`, `audited_create!` `genossi_service_impl/src/audit_macros.rs:6-36`, `APPLICATION_SERVICE_PROCESS` `genossi_service_impl/src/application.rs:20`, `DocumentType::Other` `genossi_service/src/member_document.rs:52` |
| APDOC-04 | Robust gegen Edge-Cases: kein Dokument → skip, Re-Aktivierung → bestehender `Offen`-Guard blockt, fehlende Datei → Rollback | Guard existiert `application.rs:305-310`, Storage-Fehler propagieren via `?` durch `use_transaction`-Klammer (kein Commit → automatischer Rollback) |
| APDOC-05 | Antrags-Dokument im Frontend an Application sichtbar + herunterladbar, admin-only | Muster: `genossi-frontend/src/api.rs:329-405` (upload/delete/download-url), `genossi-frontend/src/page/member_details.rs:53,138` (Dokument-Liste als Signal). Frontend-Anker: `genossi-frontend/src/component/application_detail.rs` |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Layered DAO → Service → REST → Frontend** — Trait-Boundaries, generische `Deps`.
- **Audit-Macros verpflichtend** für Member/MemberAction/MemberDocument/Application. `application_documents` NEU: **kein** `Auditable`-Impl (per CONTEXT-Decision #5 + Roadmap-Audit-Hinweis).
- **Soft-Delete + optimistic `version` UUID** — projektweites Muster.
- **Component-First (Frontend)** — keine inline-RSX-Duplikate.
- **jj statt git** — Commits via `jj commit -m …`, log via `jj log`.
- **Deutsche UI-Sprache, englischer Code**.
- **Enum statt Boolean** für umschaltbare Zustände.
- **Tests-Pflicht** (User-CLAUDE.md): jede Änderung braucht Tests.
- **GSD Workflow Enforcement:** Direkte Repo-Edits nur innerhalb eines GSD-Workflows.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Datei-Storage (Bytes → Disk) | Service (`DocumentStorage`-Impl) | — | Trait sitzt im Service-Layer, Impl im `genossi_service_impl` — REST liest nur `save()`/`load()` |
| DB-Persistenz `application_documents` | DAO (`ApplicationDocumentDaoImpl`) | — | Layer-Konvention: alle SQLx-Queries im `genossi_dao_impl_sqlite` |
| Business Rules (single-slot-Guard, Replace-Sequenz, Move-Sequenz) | Service (`ApplicationDocumentServiceImpl` + `ApplicationServiceImpl::confirm()`) | DAO (partial-unique-index als Belt-and-Suspenders) | Business-Logik gehört in Service; DB-Constraint ist Absicherung, keine Primärgrenze |
| Permission-Check + CR-02-Ordering | Service (nutzt `PermissionService`) | REST (nur Route-Nesting, kein Middleware-Check auf Rolle) | Bestehendes Muster — REST extrahiert Context, Service prüft Permission |
| Multipart-Parsing | REST (Axum `Multipart`) | — | Byte-Extraction läuft im Handler, dann an Service übergeben |
| Body-Limit + MIME-Extension-Whitelist | REST (`DefaultBodyLimit`, `lookup_allowed_mime`) | Service (zusätzlicher `MAX_FILE_SIZE`-Check) | Defense-in-Depth: Axum blockt >50MB frühzeitig, Service prüft nochmal |
| Audit-Log-Eintrag für `MemberDocument` beim `confirm` | Service (`audited_create!`) | DAO (`AuditLogDao.create_entries`) | Macro kapselt DAO-Call + Audit-Hash-Chain |
| Frontend-Slot (Upload/Download/Replace-UI) | Frontend Component (`ApplicationDocumentSlot`) | Frontend-Page (`application_detail.rs` als Anker) | Component-First-Prinzip |
| API-Client (FormData-Upload) | Frontend Service (`api.rs`-Fn analog `upload_member_document`) | — | Bestehende Konvention |

## Standard Stack

### Core (alle bereits Cargo-Dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | 0.8.3 [VERIFIED: Cargo.toml] | Multipart, DefaultBodyLimit, Router-Nesting | Projekt-Standard, bestehendes Muster in `member_document.rs` |
| `sqlx` | 0.8 [VERIFIED: Cargo.toml] | SQLite-Migration + BLOB-UUID-Persistenz | Projekt-Standard |
| `tokio::fs` | via tokio 1.35 [VERIFIED: Cargo.toml] | Async Filesystem I/O in `DocumentStorage` | Wird bereits in `FilesystemDocumentStorage` genutzt |
| `path-clean` | 1.0 [VERIFIED: Cargo.toml] | Path-Traversal-Defense in `DocumentStorage::full_path` | Bereits im Einsatz |
| `uuid` | 1.6 [VERIFIED: Cargo.toml] | Entity-IDs + optimistic `version` | Projekt-Standard |
| `time` | 0.3 [VERIFIED: Cargo.toml] | `PrimitiveDateTime` + ISO8601 | Projekt-Standard |
| `serde` | 1.0 + `serde_json` [VERIFIED: Cargo.toml] | TO-Serialisierung | Projekt-Standard |
| `utoipa` | 5.0 [VERIFIED: Cargo.toml] | OpenAPI-Schemas | Projekt-Standard |
| `mockall` | 0.13 [VERIFIED: Cargo.toml] | Mock-DAO + `MockDocumentStorage` [VERIFIED: `#[automock]` in `document_storage.rs:22`] | Projekt-Standard |
| `dioxus` | 0.6.3 [VERIFIED: Cargo.toml] | Frontend-Component | Projekt-Standard |
| `web-sys` (`FormData`, `File`) | 0.3 [VERIFIED: Cargo.toml] | Browser-FormData-Upload | Muster in `api.rs:329-405` bereits vorhanden |

### Supporting
Keine neuen Libraries nötig. **Phase 25 fügt keine externe Crate-Abhängigkeit hinzu.**

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Move-Sequenz `load → save → delete` | Filesystem-native `rename()` (schneller, atomar auf gleichem Volume) | **Verworfen:** `DocumentStorage`-Trait bietet weder `rename` noch `copy`. Trait erweitern würde v1.4-Scope sprengen; Load-Save-Delete ist funktional äquivalent und arbeitet cross-volume. |
| Single-Slot via Service-Guard | Zusätzlich: DB unique partial index | **Beide** wählen (Decision #1) — Defense-in-Depth. |

**Installation:** *nichts zu installieren — alle Deps bereits im Workspace-`Cargo.toml`.*

**Version verification:** Alle relevanten Libraries via `Cargo.toml`-Grep verifiziert (siehe `[VERIFIED]`-Tags oben). Keine externen npm/PyPI-Package-Recherchen nötig.

## Package Legitimacy Audit

*Nicht anwendbar — Phase 25 installiert keine neuen Packages. Alle verwendeten Crates sind bereits Teil des `Cargo.lock`-committed Workspaces und wurden in vorangegangenen Phasen produktiv validiert.*

## Architecture Patterns

### System Architecture Diagram (Data Flow)

```
[Admin-Browser]
     │  1. File-Dialog
     ├──> POST /api/applications/{id}/document (multipart)
     │
     ▼
[Axum-Router: application_document::generate_route]
     │  DefaultBodyLimit(50MB)
     │  Multipart parse (file_name + bytes)
     │  Extension-Check → server MIME
     │
     ▼
[ApplicationDocumentService::upload(application_id, bytes)]
     │  check_permission(MANAGE_MEMBERS_PRIVILEGE)  ← CR-02 FIRST
     │  current_user_id()  ← danach
     │  transaction begin
     │  ├─ find existing active row for application_id
     │  │   ├─ none → CREATE new row + save(relative_path)
     │  │   └─ some → UPDATE row (new version) + save(new_path) + delete(old_path)
     │  │             (best-effort delete)
     │  transaction commit
     │
     ▼
[ApplicationDocumentDao] (Sqlite-Impl)
     │  BLOB-UUID persist
     │
[FilesystemDocumentStorage]
     │  base_path/applications/{app_id}/{doc_id}.{ext}
     │  path_clean() traversal guard
     ▼
[Filesystem]


CONFIRM-FLOW (existing + extended):

POST /api/applications/{id}/confirm
     │
     ▼
[ApplicationService::confirm(id)]
     │  check_permission(...)  ← CR-02 FIX
     │  current_user_id()      ← danach
     │  transaction begin
     │  status-guard (must be Offen)
     │  member_dao.create(member) via audited_create!
     │  member_action_dao.create(Eintritt) via audited_create!
     │  member_action_dao.create(Aufstockung) via audited_create!
     │  ── NEU (Phase 25) ──
     │  IF application_document exists:
     │    storage.load(app_doc.relative_path)     — ?  propagates → rollback
     │    new_path = member_document-Konvention (uuid.ext)
     │    storage.save(new_path, bytes)           — ?  propagates → rollback
     │    member_document_dao.create(...) via audited_create! (APPLICATION_SERVICE_PROCESS,
     │                                                          DocumentType::Other,
     │                                                          description="Original-Antrag...")
     │    application_document_dao.update(soft-delete row)  — nicht auditiert
     │    storage.delete(app_doc.relative_path)   — best-effort, Warn-Log bei Fehler
     │  ── ENDE NEU ──
     │  application_dao.update(status=Bestaetigt) via audited_update!
     │  transaction commit
```

### Recommended Project Structure (Neue Dateien)

```
migrations/sqlite/
└── 2026070XNNNNNN_create_application_documents_table.sql   (neu)

genossi_dao/src/
└── application_document.rs                                  (neu — Trait + Entity)

genossi_dao_impl_sqlite/src/
└── application_document.rs                                  (neu — SQLite-Impl)

genossi_service/src/
└── application_document.rs                                  (neu — Service-Trait + DTOs)

genossi_service_impl/src/
├── application_document.rs                                  (neu — Service-Impl)
└── application.rs                                           (MODIFY — confirm() erweitern + CR-02-Fix)

genossi_rest/src/
├── application_document.rs                                  (neu — 3 Handler)
├── application.rs                                           (nichts ändern — Endpoint sitzt in eigenem Modul)
└── lib.rs                                                   (MODIFY — nest("/api/applications/{id}/document", ...), RestState-Trait erweitern)

genossi_rest_types/src/
└── lib.rs                                                   (MODIFY — ApplicationDocumentTO + UploadResponse)

genossi_bin/src/
└── lib.rs                                                   (MODIFY — RestStateImpl-Wiring für ApplicationDocumentService + Dao)

genossi-frontend/src/
├── component/application_document_slot.rs                   (neu — Component-First)
├── component/application_detail.rs                          (MODIFY — Slot einhängen; nur wenn Application offen)
├── api.rs                                                   (MODIFY — upload_application_document / delete / download-url)
└── i18n/{de.rs,en.rs}                                       (MODIFY — neue Keys: „Antrag hochladen", „Ersetzen", „Herunterladen"...)
```

### Pattern 1: Multipart-Upload-Handler (aus `member_document.rs:115-224`)

**What:** Axum-Multipart-Feld-Iteration, Extraction von `file_name` + `bytes`; Client-MIME wird verworfen, Server derived aus Extension via `lookup_allowed_mime()`.

**When to use:** Direkt für den neuen `POST /api/applications/{id}/document`-Handler.

**Example (aus dem Repo, `genossi_rest/src/member_document.rs:128-192` — Auszug angepasst für ApplicationDocument):**
```rust
while let Some(field) = multipart.next_field().await.map_err(...)? {
    match field.name().unwrap_or("").as_str() {
        "file" => {
            file_name = field.file_name().map(|s| s.to_string());
            file_data = Some(field.bytes().await?.to_vec());
        }
        _ => {}
    }
}
let fname = file_name.unwrap_or_else(|| "document".to_string());
let extension = fname.rsplit('.').next().filter(|ext| *ext != fname.as_str()).unwrap_or("");
let server_mime = lookup_allowed_mime(extension).ok_or_else(|| {
    let allowed = allowed_extensions();
    RestError::UnsupportedMediaType(serde_json::json!({
        "error": format!("File type '{}' is not allowed", extension),
        "allowed_extensions": allowed,
    }).to_string())
})?;
```

Für ApplicationDocument entfallen `document_type` und `description`-Felder.

### Pattern 2: DAO-Trait mit `find_by_application_id` (analog `MemberDocumentDao`)

**What:** Default-Methoden `all()`, `find_by_id()`, `find_by_application_id()` filtern via `dump_all()` in Rust (nicht via SQL) — bestehendes Muster.

**When to use:** `ApplicationDocumentDao` braucht `find_active_by_application_id(app_id, tx) -> Option<Entity>` (weil single-slot).

**Example (aus `genossi_dao/src/member_document.rs:112-124`):**
```rust
async fn find_active_by_application_id(
    &self,
    application_id: Uuid,
    tx: Self::Transaction,
) -> Result<Option<ApplicationDocumentEntity>, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    Ok(all_entities.iter()
        .find(|e| e.application_id == application_id && e.deleted.is_none())
        .cloned())
}
```

### Pattern 3: Storage-Move-Sequenz für Carryover (neu — kombiniert bestehende Bausteine)

**What:** `DocumentStorage` bietet nur `save/load/delete`. Move = Load-Save-Delete.

**When to use:** In `ApplicationServiceImpl::confirm()`, innerhalb der `use_transaction`-Klammer, bevor `commit(tx)` läuft.

**Example (basierend auf `genossi_service_impl/src/member_document.rs:115-117` für Path-Konvention):**
```rust
// Nach den Member/MemberAction-audited_creates:
if let Some(app_doc) = self.application_document_dao
    .find_active_by_application_id(id, tx.clone())
    .await?
{
    // 1) Load bytes (fehlt → StorageError::NotFound → ? → rollback via drop(tx))
    let bytes = self.document_storage.load(&app_doc.relative_path).await
        .map_err(|e| match e {
            StorageError::NotFound => ServiceError::InternalError(
                Arc::from(format!("Application document file missing on filesystem: {}", app_doc.relative_path))
            ),
            other => ServiceError::InternalError(Arc::from(other.to_string())),
        })?;

    // 2) Neuen Member-Doc-Pfad bilden (Konvention aus member_document.rs:115-117: "{uuid}.{ext}")
    let new_doc_id = self.uuid_service.new_v4().await;
    let extension = app_doc.file_name.rsplit('.').next()
        .filter(|e| *e != app_doc.file_name.as_ref()).unwrap_or("bin");
    let new_relative_path = format!("{}.{}", new_doc_id, extension);

    // 3) Save unter neuem Pfad
    self.document_storage.save(&new_relative_path, &bytes).await
        .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

    // 4) audited_create MemberDocument
    let description_str = format!("Original-Antrag (übernommen bei Bestätigung am {})",
        format_de_date(join_date));  // format_de_date = "DD.MM.YYYY", konsistent mit FMT-01
    let member_doc = genossi_dao::member_document::MemberDocumentEntity {
        id: new_doc_id,
        member_id,
        document_type: Arc::from("other"),
        description: Some(Arc::from(description_str.as_str())),
        file_name: app_doc.file_name.clone(),
        mime_type: app_doc.mime_type.clone(),
        relative_path: Arc::from(new_relative_path.as_str()),
        created,
        deleted: None,
        version: self.uuid_service.new_v4().await,
        template_id: None,
        mail_recipient_id: None,
        status: None,
    };
    crate::audited_create!(self, self.member_document_dao, &member_doc,
        APPLICATION_SERVICE_PROCESS, &user_id, tx);

    // 5) application_document soft-delete (nicht auditiert)
    let mut old = app_doc.clone();
    old.deleted = Some(created);
    self.application_document_dao.update(&old, APPLICATION_SERVICE_PROCESS, tx.clone()).await?;

    // 6) Store old_path für best-effort-delete-nach-commit (KEIN ? — bei Fehler nur Warn)
    let old_path_for_cleanup = app_doc.relative_path.to_string();
    // ausgeführt NACH commit unten
}
```

**Anti-Pattern zu vermeiden:** `storage.delete(old)` **vor** `commit(tx)` — wenn `commit` fehlschlägt aber `delete` schon lief, ist die Datei weg und die Application zeigt sie noch. Reihenfolge muss `commit → delete` sein.

### Pattern 4: CR-02-Fix (Permission-Check-Ordering)

**What:** `check_permission()` MUSS **vor** `current_user_id()` laufen, damit ein unautorisierter Aufruf keinen Info-Leak über User-Existenz erzeugt.

**Anti-Pattern (aktuell in `genossi_service_impl/src/application.rs:289-297`):**
```rust
let user_id = self.permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());

self.permission_service
    .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
    .await?;
```

**Korrektes Muster (bereits in `application.rs:268-270` bei `get()`):**
```rust
self.permission_service
    .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
    .await?;

let user_id = self.permission_service
    .current_user_id(context)
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());
```

**Beachte:** `context` ist `Authentication<Self::Context>` und `.clone()` ist billig (Arc-basiert). Für den Fix in `confirm/reject` muss `context.clone()` zweimal auftauchen — einmal für `check_permission`, einmal für `current_user_id`.

### Anti-Patterns to Avoid

- **`current_user_id()` vor `check_permission()`** — CR-02-Regression. Alle 3 neuen Endpunkte + Fix im bestehenden `confirm`/`reject`.
- **Storage-Delete vor DB-Commit** — bei Commit-Fehler ist Datei verloren, DB inkonsistent. Sequenz: DB-ops → commit → storage.delete (best-effort).
- **`Auditable`-Impl für `application_documents`** — verboten per Roadmap-Audit-Hinweis + Decision #5.
- **Inline-RSX in `application_detail.rs`** — Component-First-Verletzung. Slot in eigene Component.
- **`r#type: "submit"` in Buttons ohne form** — Reload-Falle (Memory-Note `feedback_dioxus_button_type`).
- **Client-MIME vertrauen** — Server derived via Extension-Whitelist (`lookup_allowed_mime`).
- **Multi-Row auf `application_documents` für dieselbe app_id** — verletzt Single-Slot. DB-Constraint als Belt-and-Suspenders, aber Service muss auch `find + update` machen (statt blind `create`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Filesystem-Storage mit Traversal-Guard | Eigenen `std::fs::write` mit Path-Concat | `FilesystemDocumentStorage` (`genossi_service_impl/src/document_storage.rs`) | Path-Traversal-Defense-Guard bereits in `full_path()` implementiert, produktiv validiert |
| Multipart-Body-Limit | Eigenen Byte-Counter im Handler | `axum::extract::DefaultBodyLimit::max(...)` | Axum liefert 413 vor Handler-Aufruf, spart Speicher |
| MIME-Extension-Whitelist | Eigenen Extension→MIME-Match | `lookup_allowed_mime()` + `allowed_extensions()` aus `member_document` | Eine Wartungsstelle, konsistente 415-Response |
| Audit-Hash-Chain-Berechnung | Manuellen SHA256 im `confirm()` | `audited_create!`-Macro | Chain-Konsistenz + Hash-Berechnung schon abgekapselt |
| BLOB-UUID Round-Trip | Manuellen `Vec<u8>`↔`Uuid`-Cast | `Uuid::from_slice()` + `entity.id.as_bytes().to_vec()` (bereits Convention) | Fehlerbehandlung via `DaoError::ParseError`-`From`-Impl bereits vorhanden |
| Optimistic-Locking-Check | Manuellen version-Vergleich | `UPDATE ... WHERE id = ? AND version = ?` + rows_affected==0 → `ConflictError` | Muster aus `member_document.rs:199-224` |
| Frontend-FormData-Upload | Eigenen `reqwest`-multipart | `web_sys::FormData` + `RequestInit::body(FormData)` | Muster in `api.rs:329-383` bereits validiert (funktioniert produktiv) |

**Key insight:** Phase 25 hat **null** neue Bausteine. Jeder Schritt hat einen Musterhandler im Repo. Der Plan besteht aus 8 Cargo-Crate-Änderungen + 1 Migration + 3 Frontend-Änderungen — alles Kopieren-und-Anpassen.

## Runtime State Inventory

*Phase 25 ist keine reine Rename/Refactor-Phase — es fügt neue Entities hinzu. Trotzdem der Check:*

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Neue Tabelle `application_documents` — Migration ist Teil der Phase. Kein Rename/Umzug bestehender Daten. | Migration schreiben; keine Datenmigration. |
| Live service config | Keine (keine n8n/systemd/Tailscale-Interaktion). | Nichts. |
| OS-registered state | Keine (Storage-Basis-Pfad `DOCUMENT_STORAGE_PATH` bereits konfiguriert, wird geteilt). | Nichts. |
| Secrets/env vars | Kein neuer Env-Var — `DOCUMENT_STORAGE_PATH` wird geteilt mit MemberDocument. | Nichts. |
| Build artifacts | Keine Artefakte, die auf einen alten Namen lauten. Neue Build-Artefakte (sqlx-prepare, Dioxus-WASM) entstehen normal beim `cargo build`. | Vor commit `cargo sqlx prepare` gegen die neue Migration ausführen (Standard-Konvention). |

## Common Pitfalls

### Pitfall 1: `DocumentStorage` hat kein `rename` — Move ist 3 Operationen
**What goes wrong:** Nutzer erwartet atomares `fs::rename()`; das trait bietet es nicht.
**Why it happens:** Trait wurde für ein späteres cross-storage-backend (z. B. S3) designt, wo `rename` semantisch teuer/nicht atomar ist.
**How to avoid:** Move = `load(old) → save(new) → delete(old)`. `save(new)` und `load(old)` müssen VOR Commit laufen; `delete(old)` NACH Commit (best-effort).
**Warning signs:** Wenn im Plan Task „storage.rename()" auftaucht — verboten.

### Pitfall 2: SQLite `sqlx::query!` mit Migration → offline-Cache-Fehler
**What goes wrong:** Compile schlägt fehl weil `sqlx::query!`-Makros gegen `.sqlx/`-Cache checken, aber neue Migration-Table fehlt im Cache.
**Why it happens:** Projekt nutzt `cargo sqlx prepare` für offline builds.
**How to avoid:** Alle SQLx-Queries in `application_document.rs`-DAO als `sqlx::query()` / `sqlx::query_as::<_, ...>()` (String-Query, kein Makro) — bestehende `member_document.rs`-DAO nutzt genau dieses Muster. Migration läuft automatisch beim Server-Start via `sqlx::migrate!` in `genossi_bin/src/main.rs`.
**Warning signs:** Ein Plan-Task, der `sqlx::query!(...)` verwendet — verboten (Konsistenz mit bestehendem Code).

### Pitfall 3: `application_documents.application_id` unique partial-index vs. Replace-in-Place
**What goes wrong:** Bei Replace macht Service `UPDATE` (nicht `INSERT`), aber wenn Service versehentlich `create` statt `update` aufruft, feuert der UNIQUE-Constraint.
**Why it happens:** Single-Slot-Constraint als Belt-and-Suspenders ist stark; jeder Fehler im Service-Code manifestiert sich als DB-Error.
**How to avoid:** `ApplicationDocumentService::upload()` MUSS immer `find_active_by_application_id()` machen und dann entweder `create` oder `update` aufrufen — nicht immer `create`.
**Warning signs:** Test schlägt mit `UNIQUE constraint failed: idx_application_documents_one_active` fehl → das ist der Constraint der greift, Business-Logik-Fehler.

### Pitfall 4: Soft-delete UPDATE erwischt bereits-deleted-Row nicht → UNIQUE-Constraint block
**What goes wrong:** Beim `confirm` wird `application_document` soft-deleted. Wenn der Admin danach **erneut** ein Dokument an derselben (falls status noch offen wäre — ist nicht der Fall wegen Guard) hochladen wollte, würde ein neuer INSERT den UNIQUE-Constraint verletzen — falls `WHERE deleted IS NULL` NICHT im Constraint steht.
**Why it happens:** Standard-UNIQUE ohne `WHERE` erwischt alle Rows.
**How to avoid:** `CREATE UNIQUE INDEX idx_application_documents_one_active ON application_documents(application_id) WHERE deleted IS NULL` — **partial index** (SQLite unterstützt das seit 3.8, ohne Sonder-Flags [ASSUMED — bestätigt via bestehende Nutzung von `sqlx::query` mit `deleted IS NULL` in projektweitem Code]).
**Warning signs:** Test „Upload → confirm → nochmal Upload an neuer Application" scheitert am Constraint → Constraint ist nicht partial.

### Pitfall 5: `context.clone()` beim CR-02-Fix übersehen
**What goes wrong:** `check_permission(context.clone())` klaut `context`, danach ist `context` moved und `current_user_id(context)` schlägt fehl.
**Why it happens:** `Authentication<Ctx>` ist nicht `Copy`.
**How to avoid:** Zwei `.clone()`-Calls machen — Konvention siehe `application.rs:268-270`.
**Warning signs:** Rust-Compiler-Fehler `use of moved value` beim ersten Compile.

### Pitfall 6: Description-Datum in ISO statt DE-Format
**What goes wrong:** `description = "Original-Antrag (übernommen am 2026-07-03)"` — Vorstands-facing statt technisch.
**Why it happens:** `time::PrimitiveDateTime`-`Display` gibt ISO.
**How to avoid:** Kleiner Formatter-Helper (analog FMT-01) `"[day].[month].[year]"` → `"03.07.2026"`. Bereits im Projekt via `genossi_mail/src/template.rs` (nach Phase 22 FMT-01-Fix) oder inline via `time::format_description::parse(...)`.
**Warning signs:** Manuelle Kontrolle im MemberDocument-Detail zeigt `2026-07-03` statt `03.07.2026`.

### Pitfall 7: `DOCUMENT_STORAGE_PATH` shared between application/member docs — Directory-Kollision
**What goes wrong:** `storage.save("applications/{app_id}/{uuid}.pdf")` und `storage.save("{doc_uuid}.pdf")` (Member-Konvention) liegen im selben `base_path`. Wenn eine UUID-Kollision entsteht (praktisch null), wird eine Datei überschrieben.
**Why it happens:** Base-Pfad wird geteilt.
**How to avoid:** Application-Files unter Sub-Ordner ablegen: z. B. `applications/{app_id}/{uuid}.{ext}` (siehe Diagram oben). Member-Files bleiben `{uuid}.{ext}` (bestehend). UUID-Kollision ist mathematisch vernachlässigbar (2^128), aber Sub-Ordner-Trennung hilft beim Debuggen und späterer Migration.
**Warning signs:** Kein direkter Fehler, nur Verwirrung im Storage-Ordner. → **Recommend: `relative_path`-Konvention für application_documents = `applications/{application_id}/{doc_uuid}.{ext}`.**

## Code Examples

### Beispiel: Migration `application_documents`

**Source:** eigene Ableitung von `20260331000005_create_member_document_table.sql` + Decisions #1, #5.

```sql
-- Phase 25 (APDOC-01..05): application_documents-Tabelle für Original-Antrags-Datei.
-- Single-Slot pro Application (unique partial index).
-- Nicht auditiert (Roadmap-Audit-Hinweis Phase 25 + CONTEXT.md Decision #5).
CREATE TABLE IF NOT EXISTS application_documents (
    id             BLOB PRIMARY KEY NOT NULL,
    application_id BLOB NOT NULL,
    file_name      TEXT NOT NULL,
    mime_type      TEXT NOT NULL,
    relative_path  TEXT NOT NULL,
    size           INTEGER NOT NULL,
    created        TEXT NOT NULL,
    deleted        TEXT,
    version        BLOB NOT NULL,
    FOREIGN KEY (application_id) REFERENCES application(id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_application_documents_one_active
    ON application_documents(application_id) WHERE deleted IS NULL;
CREATE INDEX IF NOT EXISTS idx_application_documents_deleted
    ON application_documents(deleted);
```

Naming-Check: Der bestehende `member_document`-Table (Singular). Application-Table heißt tatsächlich `application` (Singular, siehe FK-Referenz oben) — daher `application_documents` (Plural, konsistent mit MemberDocument-Migrations-Konvention). **Bitte im Plan gegen die tatsächliche `application`-Table-Name in `20260413000000_create_application_table.sql` verifizieren** (das Migrationsfile existiert, siehe `<code_deep_dive>`).

### Beispiel: `audited_create!` für Carryover-MemberDocument im `confirm()`

**Source:** `genossi_service_impl/src/application.rs:353-360` (existing pattern) + `audit_macros.rs:6-36` (macro def).

```rust
crate::audited_create!(
    self,
    self.member_document_dao,
    &member_doc,
    APPLICATION_SERVICE_PROCESS,   // "application-service" (Zeile 20)
    &user_id,
    tx
);
```

Der `APPLICATION_SERVICE_PROCESS`-String stellt sicher, dass die Audit-Row dem Aktivierungs-Prozess zugeordnet ist (nicht `member-document-service`). Wichtig für die Compliance-Rekonstruktion des `confirm`-Events.

### Beispiel: `ApplicationDocumentTO` DTO

**Source:** Ableitung von `MemberDocumentTO` (`genossi_rest_types/src/lib.rs:667-695`), reduziert um `document_type`/`description`.

```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApplicationDocumentTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174001")]
    pub application_id: Uuid,
    #[schema(example = "antrag_scan.pdf")]
    pub file_name: String,
    #[schema(example = "application/pdf")]
    pub mime_type: String,
    #[schema(example = 234567)]
    pub size: i64,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}
```

**Kein** `document_type`, **kein** `description` — beide unnötig für Single-Slot-Antrag. **Kein** `deleted`-Feld im TO (soft-deletes werden vom Backend gefiltert; wenn der Slot leer ist, kommt einfach nichts oder 404 zurück).

### Beispiel: Frontend-Slot-Component Skeleton

**Source:** Ableitung von `application_detail.rs`-Konvention + Component-First-CLAUDE.md.

```rust
// genossi-frontend/src/component/application_document_slot.rs
use dioxus::prelude::*;
use rest_types::ApplicationDocumentTO;
use uuid::Uuid;

#[component]
pub fn ApplicationDocumentSlot(
    application_id: Uuid,
    document: Signal<Option<ApplicationDocumentTO>>,
    on_changed: EventHandler<()>,
) -> Element {
    // Zustand 1: leer → Upload-Button
    // Zustand 2: gefüllt → Dateiname/Größe/Datum + Download + Ersetzen + Löschen
    // Alle Buttons: r#type: "button" (Reload-Bug-Vermeidung)
    // Upload/Ersetzen öffnen `<input type="file">` via ref-hack (bestehende Muster in mail_compose/attachment_picker.rs prüfen)
    rsx! { /* ... */ }
}
```

## State of the Art

*Keine externen Bibliotheken relevant — Phase 25 nutzt ausschließlich in-repo-Muster.*

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manuelle Excel-Antrags-Ablage (Vor-genossi) | Datei-Upload an Application + auditierter Carryover | Phase 25 (v1.4) | Papier-scan wandert direkt zum Mitgliederstamm |
| Copy-Semantik (früher Plan-Wortlaut APDOC-03) | Move/Ownership-Übergabe | CONTEXT.md Decision #3 (2026-07-02) | Weniger Storage-Bloat, klare Datenherkunft |
| `check_permission` nach `current_user_id` in mehreren Handlers | CR-02: alle Handler prüfen zuerst Permission | v1.2-MILESTONE-AUDIT identified, laufender Fix pro Phase | Kein Info-Leak über User-Existenz |

**Deprecated/outdated:**
- APDOC-03-Wortlaut in `.planning/REQUIREMENTS.md:41` — sagt „kopiert (nicht verschoben)". Muss auf „übernommen (Ownership-Übergabe)" geändert werden.

## Runtime Requirements Wording Update (APDOC-03)

**Fundstelle:** `.planning/REQUIREMENTS.md:41` (Zeile 41):

**Aktueller Wortlaut:**
> - [ ] **APDOC-03**: Beim Aktivieren (`confirm`) einer `Application` wird ein hinterlegtes Antrags-Dokument **kopiert** (nicht verschoben) und als `MemberDocument` am Mitglied angelegt — innerhalb derselben atomaren Aktivierungs-Transaktion, via `audited_create!` unter `APPLICATION_SERVICE_PROCESS`, mit `DocumentType::Other` + beschreibender Bezeichnung.

**Vorgeschlagener neuer Wortlaut:**
> - [ ] **APDOC-03**: Beim Aktivieren (`confirm`) einer `Application` wird ein hinterlegtes Antrags-Dokument **übernommen** (Ownership-Übergabe — Move-Semantik: die `application_documents`-Zeile wird soft-deleted und die Datei physisch an den Member-Pfad verschoben) und als auditiertes `MemberDocument` am Mitglied angelegt — innerhalb derselben atomaren Aktivierungs-Transaktion, via `audited_create!` unter `APPLICATION_SERVICE_PROCESS`, mit `DocumentType::Other` + beschreibender Bezeichnung („Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)").

Zusätzlich sollte am Ende der Zeile (oder als extra Bullet) das Sync mit ROADMAP-Success-Criteria #3 erwähnt werden (dort steht ebenfalls „kopiert"). Der ROADMAP-Wortlaut ist im Milestone-Archiv historisch; ROADMAP-Live in `.planning/ROADMAP.md:154` ebenfalls anpassen zwecks Konsistenz.

**Plan-Task-Empfehlung:** Ein separater Doku-Commit-Task, der beide Zeilen synchron ändert (REQUIREMENTS.md:41 + ROADMAP.md:154 identisch) — der Planer sollte das als „Wave 0 / Doku-Fix" hochziehen, damit er nicht am Ende vergessen wird.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SQLite partial index `CREATE UNIQUE INDEX ... WHERE deleted IS NULL` funktioniert auf der Ziel-SQLite-Version. | Pitfall 4, Code Examples | Wenn Ziel < 3.8: partial index scheitert. **Mitigation:** SQLite 3.8 ist von 2013, produktiv sicher aktuell. Bestehende Projekt-Queries filtern bereits mit `WHERE deleted IS NULL` — Feature ist implizit verifiziert. |
| A2 | Der Application-Table heißt tatsächlich `application` (Singular), sodass FK `REFERENCES application(id)` korrekt ist. | Migration-Beispiel | Falls Table `applications` (Plural): FK-Constraint scheitert. **Mitigation:** Plan-Task hat Verifikation `sqlite3 genossi.db .schema application*` als vorletzten Schritt vor Migration. Bestätigt via `20260413000000_create_application_table.sql` (Datei-Name enthält Singular). |
| A3 | `axum::extract::Multipart` mit `DefaultBodyLimit::max(50 * 1024 * 1024)` blockt Bodies >50MB ohne Handler-Aufruf und liefert 413. | Standard Stack | Falls Axum das nicht macht: OOM-Risiko. **Mitigation:** Muster ist produktiv im `member_document.rs:48-49` — bereits validiert. |
| A4 | Die `application`-Table hat die Spalte `id` als `BLOB PRIMARY KEY` (nicht `TEXT`). | Migration FK | Falls anders: FK-Type-Mismatch. **Mitigation:** Plan verifiziert via Read der bestehenden Migration. Konvention im gesamten Projekt = BLOB. |
| A5 | Beim Frontend-FormData-Upload wird `Content-Type: multipart/form-data; boundary=...` **automatisch** vom Browser gesetzt, wenn `RequestInit::body(&form_data)` verwendet wird. | Frontend | Falls manuell gesetzt: Boundary-Mismatch. **Mitigation:** Muster `api.rs:355-360` setzt Content-Type NICHT manuell und funktioniert produktiv. |

## Open Questions

1. **URL-Segment `/document` vs. `/documents` unter `/api/applications/{id}/`**
   - Was wir wissen: CONTEXT.md #7 sagt `/api/application/{id}/document` (Singular auf beiden Ebenen). Bestehende Application-Route ist unter `/api/applications` (Plural) genestet (`genossi_rest/src/lib.rs:645`). Bestehende Member-Document-Route ist `/api/members/{member_id}/documents` (Plural auf beiden Ebenen).
   - Was unklar ist: Die CONTEXT.md-Formulierung könnte semantisch (Single-Slot = Singular) gemeint sein, nicht pfad-verbindlich.
   - Empfehlung: **Nesting-Basis `/api/applications` (bleibt Plural, keine Router-Restrukturierung) → Singular-Slot-Endpunkt `/document`**. Also finale Pfade: `POST/GET/DELETE /api/applications/{id}/document`. Semantisch korrekt (Single-Slot), konsistent mit Application-Route-Basis, keine Router-Duplikation.

2. **Description-Format-Helper**
   - Was wir wissen: FMT-01 (Phase 22) hat einen DE-Datumsformatter für Mail-Templates eingeführt (`genossi_mail/src/template.rs`).
   - Was unklar ist: Ist dieser Helper aus dem `genossi_mail`-Crate im `genossi_service_impl` schon verfügbar oder muss er dupliziert / hochgezogen werden?
   - Empfehlung: Kleiner Inline-Formatter im `application.rs::confirm()` (2 Zeilen: `time::format_description::parse("[day].[month].[year]")?` → `join_date.format(...)?`). Reines Duplizieren-Risiko ist mikro. Ansonsten Utility-Extraction in v1.5.

## Environment Availability

*Rein Code-/DB-/Frontend-Änderung. Keine neuen Environment-Dependencies.*

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain 1.70+ | Alle Backend-Crates | ✓ | via `flake.nix` [VERIFIED: via Nix-Notes im CLAUDE.md-Memory] | — |
| `sqlx-cli` | Migration-Prep | ✓ | via `nix develop` | — |
| SQLite 3.8+ (für partial index) | Migration | ✓ [ASSUMED — SQLite 3.8 ist von 2013; systemweit sicher aktuell auf allen Dev-Systemen] | — | — |
| Dioxus CLI (`dx`) | Frontend-Build | ✓ | via `nix develop` | — |
| Node.js (Tailwind Watch) | Frontend-Dev | ✓ | via Nix (Memory-Note: `Frontend-Flake trackt nixos-unstable`) | — |
| `DOCUMENT_STORAGE_PATH` env var | Runtime storage base | Default `./documents` | — | — |

**Missing dependencies with no fallback:** keine
**Missing dependencies with fallback:** keine

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Session-Cookie (bestehend), OIDC-Feature (bestehend) — keine Änderung |
| V3 Session Management | yes | tower-sessions (bestehend) — keine Änderung |
| V4 Access Control | **yes — CR-02-Kernthema** | `PermissionService::check_permission(MANAGE_MEMBERS_PRIVILEGE)` MUSS vor `current_user_id()`. Test: unautorisierter Aufruf liefert 401/403 ohne Side-Effect + ohne Datenpreisgabe |
| V5 Input Validation | yes | Multipart-Field-Names via match; File-Extension via `lookup_allowed_mime` (Whitelist); Client-MIME verworfen; Body-Limit via `DefaultBodyLimit`; UUID-Path via `FilesystemDocumentStorage::full_path()` (path_clean + `starts_with(base_path)`-Check) |
| V6 Cryptography | no | Keine neuen Crypto-Ops — Audit-Chain nutzt existing SHA256 aus `audit_log.rs` |
| V12 File & Resources | **yes** | Path-Traversal blockt `FilesystemDocumentStorage::full_path()`; MIME-Whitelist statt Blacklist; Body-Limit blockt DoS-via-huge-files |
| V13 API & Web Service | yes | Admin-only-Enforcement + kein Info-Leak (CR-02) |

### Known Threat Patterns for {Rust/Axum + SQLite + Filesystem}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via Filename `../../etc/passwd` | Tampering | Storage nutzt `path_clean()` + startet-mit-Basis-Check (`document_storage.rs:25-59`). UUID-basierter `relative_path` statt Client-Filename verhindert das ohnehin. |
| Slop MIME-Type-Spoofing (Client sendet `application/pdf` für `.exe`) | Tampering | Client-MIME wird verworfen; Server-derived via Extension-Whitelist (`lookup_allowed_mime`) |
| DoS via 10-GB-Upload | Denial of Service | `DefaultBodyLimit::max(50 * 1024 * 1024)` an der Route + Service-side `MAX_FILE_SIZE`-Check als Defense-in-Depth |
| Info leak über `check_permission`-Reihenfolge (CR-02) | Information Disclosure | `check_permission` VOR `current_user_id` in allen neuen Handlern + Fix im bestehenden `confirm()` |
| SQL injection via multipart-Feld | Tampering | SQLx-Bind-Parameter (bestehende Konvention) — keine String-Concat-Queries |
| Filesystem-Race beim Replace (delete-then-save) | Tampering | Save-new → update-DB → delete-old-Reihenfolge stellt sicher, dass die neue Datei existiert, bevor die alte weg ist |
| Zombie-Files nach commit-fail-delete-success | Info Disclosure (indirekt: alte Datei bleibt lesbar) | Reihenfolge im `confirm()`: erst `commit(tx)`, dann `storage.delete(old)` (best-effort, mit Warn-Log) |
| Cross-application file access via app_id-Substitution | Access Control | Storage-Layout unter `applications/{app_id}/{doc_id}` + jeder REST-Endpunkt prüft `application_document.application_id == path::application_id`. Aber: `check_permission(MANAGE_MEMBERS_PRIVILEGE)` reicht — Admin darf ohnehin alles sehen |

## Sources

### Primary (HIGH confidence — in-repo verified)
- `genossi_rest/src/member_document.rs` (Zeilen 37-224) — Multipart-Upload-Muster
- `genossi_service/src/document_storage.rs` (Zeilen 1-28) — Trait-Signatur `save/load/delete`
- `genossi_service_impl/src/document_storage.rs` (Zeilen 1-97) — Filesystem-Impl + Path-Traversal-Guard
- `genossi_service_impl/src/application.rs` (Zeilen 1-421) — Bestehende `confirm()`-Tx + CR-02-Anti-Pattern
- `genossi_service_impl/src/audit_macros.rs` (Zeilen 1-127) — `audited_create!` Signatur
- `genossi_service_impl/src/member_document.rs` (Zeilen 1-267) — Service-Muster + `extract_extension` + Storage-save-after-persist
- `genossi_dao/src/member_document.rs` (Zeilen 1-296) — DAO-Trait mit `find_by_member_id` + `Auditable`-Impl (was wir NICHT machen für ApplicationDocument)
- `genossi_dao_impl_sqlite/src/member_document.rs` (Zeilen 1-376) — SQLite-Impl-Muster inkl. BLOB-UUID-Handling
- `genossi_dao/src/auditable.rs` — Auditable-Trait-Definition (Kontrast zu ApplicationDocument, das dieses NICHT implementiert)
- `migrations/sqlite/20260331000005_create_member_document_table.sql` — Schema-Vorlage
- `genossi_rest_types/src/lib.rs` (Zeilen 667-717) — TO-Muster
- `genossi_rest/src/lib.rs` (Zeilen 225-263, 590-670) — RestState-Trait + Route-Nesting-Konvention
- `genossi-frontend/src/api.rs` (Zeilen 315-405) — FormData-Upload-Muster in WASM
- `genossi-frontend/src/component/application_detail.rs` (Zeilen 1-219) — Anker-Component + Confirm/Reject-Dialoge

### Secondary (MEDIUM confidence — hergeleitet aus mehreren Sources)
- Storage-Path-Konvention für Application-Files (`applications/{app_id}/{doc_uuid}.{ext}`) — hergeleitet aus MemberDocument-Konvention + Pitfall 7
- URL-Nesting-Empfehlung (`/api/applications/{id}/document`) — hergeleitet aus Router-Nesting-Muster in `genossi_rest/src/lib.rs`

### Tertiary (LOW confidence — Assumptions)
Siehe `## Assumptions Log`.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — alle Bausteine in-repo verifiziert, keine neuen Libraries
- Architecture: HIGH — 1:1 Muster-Kopie von MemberDocument mit dokumentierten Trimmings
- Pitfalls: HIGH — Pitfalls direkt aus Code-Trace (CR-02-Anti-Pattern zeigt aktuelle Zeile 289-297)
- Frontend: MEDIUM — kein existierender „file-slot"-Component gefunden, aber MemberDocument-Upload-Muster in `api.rs` liefert das Skeleton
- Security: HIGH — ASVS-Mapping direkt an existierende Controls angelehnt

**Research date:** 2026-07-03
**Valid until:** 2026-08-03 (30 Tage; keine fast-moving Deps involviert)
