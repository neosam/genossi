# Phase 25 Context: Application File Upload + Audited Carryover

**Created:** 2026-07-02
**Phase Goal (ROADMAP):** Admin lädt originalen Mitgliedsantrag als Datei an eine `Application`; beim `confirm` wird die Datei automatisch als auditiertes `MemberDocument` ans Mitglied übernommen. (Unabhängig — parallelisierbar zu 22→23→24.)
**Requirements:** APDOC-01, APDOC-02, APDOC-03, APDOC-04, APDOC-05

<domain>
Ein Admin hängt genau **einen** eingescannten Original-Antrag (PDF/Bild) an eine offene `Application`. Beim `confirm()` wird die Datei atomar mit der Member-Aktivierung als **auditiertes** `MemberDocument` ans neue Mitglied übertragen. Die `Application`-Seite gibt die Datei dabei ab (Ownership-Übergabe), sodass nach dem `confirm` das Mitglied der eindeutige Besitzer des Original-Antrags ist.
</domain>

<canonical_refs>
- **`.planning/ROADMAP.md`** — Phase 25 Success Criteria (APDOC-01..05); enthält den **Audit-Hinweis** (`application_documents` NICHT auditiert, Carryover-`MemberDocument` IST auditiert)
- **`.planning/REQUIREMENTS.md`** — APDOC-01..05 Wortlaut. **⚠ Sync-Pflicht:** APDOC-03 formuliert derzeit „kopiert (nicht verschoben)". Diese Discussion entscheidet **Move-Semantik statt Copy** (siehe `<decisions>` #3). Requirements-Update auf „übernommen (Ownership-Übergabe)" im Rahmen der Phase-25-Implementierung mit erledigen — reine Textkorrektur, semantisches Ergebnis (auditierter Carryover) bleibt.
- **`genossi_rest/src/member_document.rs`** (Zeilen 43–224) — **Referenz-Upload-Handler**: Multipart-Parsing, Server-derived MIME (Client-MIME wird verworfen), `DefaultBodyLimit`, `allowed_extensions()`/`lookup_allowed_mime()`, `document_storage().save()`-nach-Service-Persist
- **`genossi_service/src/document_storage.rs`** (Zeilen 24–28) — Storage-Trait `DocumentStorage`: `save/load/delete`. **Kein `copy`** und **kein `rename`** — Move = `load` → `save` unter neuem Pfad → `delete` unter altem Pfad, alles in derselben Service-Transaktion um die File-Operationen; bei DB-Rollback muss der Storage-Move ebenfalls zurückrollen (siehe `<decisions>` #4)
- **`genossi_service_impl/src/application.rs`** (Zeilen 280–419) — bestehender `confirm()`-Ablauf (Member-Entity + Eintritt + Aufstockung + Application-Update in einer Tx via `audited_*!`). **CR-02-Anti-Pattern hier präsent** (`current_user_id()` vor `check_permission()`); an dieser Stelle mit dem Phase-25-Diff **fixen** (per APDOC-02 verpflichtend)
- **`genossi_dao/src/auditable.rs`** — `Auditable`-Trait (nur für Entitäten, die auditiert werden). `application_documents` bekommt **keine** `Auditable`-Impl
- **`genossi_service_impl/src/audit_macros.rs`** — `audited_create!`-Signatur inkl. `APPLICATION_SERVICE_PROCESS`-Konvention
- **`genossi_dao_impl_sqlite/src/member_document*.rs`** — SQLite-DAO-Muster (BLOB-UUID, `deleted`, `version`) als Vorlage für `application_documents`-DAO
- **`.planning/codebase/ARCHITECTURE.md`**, **`.planning/codebase/CONVENTIONS.md`** — Layer-Grenzen (DAO→Service→REST→Frontend), Naming, Fehler-Enums (`DaoError`, `ServiceError`, `RestError`)
</canonical_refs>

<prior_decisions>
**Aus v1.1/v1.2/v1.3 übernommen (nicht mehr fragen):**
- **Layered Architektur** DAO → Service → REST — Trait-Boundaries + generische `Deps`
- **Soft-Delete + optimistic `version` UUID** — projektweites Entity-Muster
- **Audit-Macros `audited_create!/update!/delete!`** — für auditierte Entitäten Pflicht; MemberDocument gehört dazu
- **CR-02 Permission-Check-Ordering** — projektweiter BLOCKER-Techdebt: `check_permission()` MUSS **vor** `current_user_id()` laufen (Info-Leak-Vermeidung). Neuer Code darf nicht regressiv sein
- **Component-First (Frontend)** — keine inline-RSX-Duplikate; wiederkehrende UI → `genossi-frontend/src/component/`
- **Enum statt Boolean** — für umschaltbare Zustände (falls hier relevant, z. B. Upload-Status)
- **jj statt git** — Repo ist Jujutsu; Commits via `jj commit -m …`
- **Deutsche UI-Sprache** — Frontend-Texte deutsch, Code englisch

**Aus dieser Discussion:** siehe `<decisions>`.
</prior_decisions>

<code_context>
**Wiederzuverwendende Assets (nicht neu schreiben):**

| Asset | Datei | Zweck in Phase 25 |
|---|---|---|
| `DocumentStorage`-Trait | `genossi_service/src/document_storage.rs` | Filesystem-Storage für den Application-Upload und beim Move zum Member-Pfad |
| `allowed_extensions()`, `lookup_allowed_mime()` | `genossi_service*/member_document*` (verwendet in `genossi_rest/src/member_document.rs:184,192`) | MIME-Allowlist wiederverwenden — **eine** Wartungsstelle |
| `MEMBER_DOCUMENT_BODY_LIMIT` | `genossi_rest/src/member_document.rs:37` | Body-Limit teilen (kein separates Application-Limit) |
| Multipart-Upload-Handler-Muster | `genossi_rest/src/member_document.rs:115-224` | Struktur 1:1 spiegeln (parse Fields → validate extension → service call → storage save) |
| `audited_create!` | `genossi_service_impl/src/audit_macros.rs` | Für die MemberDocument-Zeile beim `confirm` |
| `APPLICATION_SERVICE_PROCESS`-Konstante | `genossi_service_impl/src/application.rs` | Audit-Process-String beim Carryover — im selben Prozess wie Member/Actions |
| bestehende `confirm()`-Tx | `genossi_service_impl/src/application.rs:280-419` | Ergänzt um Move + `audited_create!(MemberDocument)`, alles in der bestehenden `use_transaction` |
| MemberDocument-DAO/Service | `genossi_dao/src/member_document.rs`, `genossi_service*/member_document*.rs` | Wird beim Carryover aufgerufen — **nicht** duplizieren |

**Frontend-Kontext:**
- Bestehende Application-Detail-Page als Anker für Upload-Button + Download-Link
- Component-Kandidat: `ApplicationDocumentSlot` (Upload/Replace/Download-Anzeige in einer Komponente) — nur extrahieren, falls das Muster mehrfach vorkommt; sonst inline auf der Application-Detail-Page, aber sauber gegliedert
</code_context>

<decisions>

### 1. Slot-Modell: Single-Slot (genau eine Datei pro Application)
Pro `Application` existiert höchstens **eine** aktive `application_document`-Zeile. Keine Liste, kein Auswahl-UI beim `confirm`. Semantisch: „der Original-Antrag" — Einzahl.

- **DB-Constraint:** unique Index auf `application_id WHERE deleted IS NULL` (oder single-record-Guard im Service — bevorzugt DB-Constraint, hält Invariante bei allen Pfaden).
- **UI-Konsequenz:** Application-Detail-Page hat einen Slot, der entweder leer ist (Upload-Button) oder gefüllt (Datei-Name + Download + Ersetzen).

### 2. Re-Upload-Verhalten: Ersetzen (Replace-in-Place)
Zweiter Upload überschreibt die bestehende Datei:
- **DB:** derselbe `application_document`-Record wird via `UPDATE` mit neuem `file_name`, `mime_type`, `relative_path`, `size` + neuem `version`-UUID versorgt.
- **Storage:** die alte Datei wird **physisch gelöscht** (nach erfolgreichem Save der neuen Datei), um verwaiste Blobs zu vermeiden. Reihenfolge in einer Service-Methode `replace_document`: save-new → update-DB → delete-old (falls delete fehlschlägt, nur warnen, DB ist korrekt).
- Keine Historie / kein Audit auf `application_documents` (nicht auditiert per Roadmap-Audit-Hinweis).

### 3. Carryover-Semantik: **Move (Ownership-Übergabe), nicht Copy**
**Abweichung vom aktuellen APDOC-03-Wortlaut.** Rationale (User-Entscheidung): Nach `confirm()` gehört der Antrag konzeptuell dem Mitglied. Die `Application` ist danach ohnehin `Bestaetigt` (read-only, historischer Datensatz) und braucht die Datei nicht mehr — das MemberDocument **ist** das Antrags-Dokument in seiner endgültigen Form.

- **Fachlich:** Beim `confirm()` wird die `application_document`-Zeile **soft-deleted** (`deleted = now()`), und in derselben Transaktion wird ein neues, **auditiertes** `MemberDocument` erzeugt (`DocumentType::Other`, Description z. B. `"Original-Antrag (übernommen bei Bestätigung am {date})"`, deutsches Format `DD.MM.YYYY`).
- **Storage:** Die Datei wird an den Member-Pfad **verschoben** (Trait bietet kein `rename` — Service macht `load(old_path)` → `save(new_path)` → `delete(old_path)`; alles innerhalb der Service-`confirm`-Methode).
- **REQUIREMENTS.md-Sync:** APDOC-03-Text umformulieren auf „übernommen (Ownership-Übergabe)" — als kleiner Doku-Fix in der Phase-25-Implementierung; das übergeordnete Ziel (auditierter Carryover) bleibt.

### 4. Atomicity + Rollback bei fehlender/beschädigter Datei
Kritischer Edge-Case (APDOC-04): Datei fehlt/ist beschädigt auf dem Filesystem → **komplette `confirm`-Transaktion rollt zurück** (kein neuer Member, keine Actions, kein MemberDocument).

- **Umsetzung:** Storage-Move-Schritte laufen **innerhalb** der bestehenden `use_transaction`-Klammer. Sequenz beim `confirm()`:
  1. Permission + Status-Guard (`Offen`)
  2. Prüfen ob `application_document` existiert (Optional — kein Fehler wenn nicht)
  3. Member/Actions/Application-Status wie gehabt
  4. **Falls Dokument vorhanden:**
     a. `storage.load(old_relative_path)` — schlägt fehl → `?` propagiert → Tx-Rollback
     b. Neue relative Path für Member erzeugen (UUID + Extension)
     c. `storage.save(new_relative_path, bytes)`
     d. `member_document_dao` create via `audited_create!` (`APPLICATION_SERVICE_PROCESS`, `DocumentType::Other`, Description)
     e. `application_document` soft-delete (DAO-Update, nicht auditiert)
     f. `storage.delete(old_relative_path)` — best-effort (Warn-Log bei Fehler, kein Rollback), analog zum Member-Document-Muster
  5. `commit(tx)`
- **Best-Effort-Kaveat:** Wenn `commit(tx)` erfolgreich, `storage.delete(old)` aber fehlschlägt, bleibt eine Waisen-Datei am Application-Pfad. Kein Rollback (Member ist bereits aktiviert). **Nur** loggen — Bereinigung erfolgt manuell oder via zukünftigem Housekeeping-Job (deferred, siehe unten).
- **Antrag ohne Dokument:** Schritt 4 wird übersprungen; `confirm` läuft wie bisher (kein Fehler).
- **Re-Aktivierung:** bestehender Status-Guard (`entity.status != Offen → Conflict`) verhindert Doppel-Carryover — keine zusätzliche Logik nötig.

### 5. `application_documents`-Schema (Minimal)

```
CREATE TABLE application_documents (
  id            BLOB PRIMARY KEY,          -- UUID
  application_id BLOB NOT NULL,             -- FK Application.id
  file_name     TEXT NOT NULL,              -- ursprünglicher Client-Name
  mime_type     TEXT NOT NULL,              -- server-derived
  relative_path TEXT NOT NULL,              -- z. B. applications/{app_id}/{uuid}.pdf
  size          INTEGER NOT NULL,           -- bytes
  created       TIMESTAMP NOT NULL,
  deleted       TIMESTAMP NULL,             -- Soft-Delete (Carryover setzt diesen)
  version       BLOB NOT NULL,              -- UUID optimistic locking
  FOREIGN KEY (application_id) REFERENCES applications(id)
);
CREATE UNIQUE INDEX idx_application_documents_one_active
  ON application_documents(application_id) WHERE deleted IS NULL;
```

- **Kein `document_type`** — Typ ist implizit „Antrag" (Single-Slot).
- **Keine `description`** — auf Application-Seite ohnehin nur ein Slot; die Description beim Carryover-MemberDocument wird beim `confirm` in Rust erzeugt.
- **Kein Auditable-Impl.**

### 6. MIME-Allowlist & Body-Limit: aus MemberDocument wiederverwenden
- `allowed_extensions()` und `lookup_allowed_mime()` unverändert übernehmen (eine Wartungsstelle).
- Body-Limit = `MEMBER_DOCUMENT_BODY_LIMIT` (kein separates `APPLICATION_DOCUMENT_BODY_LIMIT`).
- Client-MIME wird verworfen, Server leitet MIME aus der Extension ab — Muster 1:1 aus `member_document.rs:177-192`.

### 7. Endpoint-Design (REST)
- `POST   /api/application/{id}/document`  — Multipart-Upload (admin-only). Wenn Slot leer: neu. Wenn Slot gefüllt: **Ersetzen** (siehe #2).
- `GET    /api/application/{id}/document`  — Download (admin-only). 404 wenn nicht vorhanden.
- `DELETE /api/application/{id}/document`  — Soft-Delete (admin-only). Storage-Datei physisch löschen. Nutzt Admin, wenn falsche Datei hochgeladen und noch nicht bestätigt.

Alle drei Endpunkte: **`check_permission()` VOR `current_user_id()`** (CR-02-Fix). Zusätzlich im bestehenden `confirm()`-Handler denselben Fix mit-anwenden (per APDOC-02 verpflichtend).

### 8. Frontend (APDOC-05)
- Auf der Application-Detail-Page ein `ApplicationDocumentSlot`-Bereich:
  - **Leerer Zustand:** primärer Upload-Button („Antrag hochladen") + drag-and-drop optional (Nice-to-have; MVP: Button reicht).
  - **Gefüllter Zustand:** Datei-Name, Größe, Upload-Datum, Download-Icon, Ersetzen-Icon (öffnet Datei-Dialog → Replace-Endpoint), Löschen-Icon.
- Nach erfolgreichem `confirm()`: der Slot verschwindet automatisch (Application wechselt zu `Bestaetigt` → Application-Detail-Ansicht zeigt keinen Upload mehr, aber Verweis auf das MemberDocument am neuen Member-Detail).
- **Component-First:** Prüfen, ob wiederverwendbarer Datei-Slot bereits existiert (`genossi-frontend/src/component/` durchsuchen). Sonst neu anlegen: `ApplicationDocumentSlot` in `genossi-frontend/src/component/`, verwendet in Application-Detail-Page.
- **Dioxus-Button-Reload-Fix:** Alle interaktiven Buttons `r#type: "button"` + `onclick`, keine form-Submits (bekannte Falle aus Phase 17-Hotfix e245013).

### 9. Test-Strategie (grob — Details im Plan)
- **Unit Service:** Move-Sequenz mit Mock-Storage: Happy Path, fehlende Datei → Rollback, save-Fehler → Rollback, delete-Fehler nach commit → nur Warn.
- **Integration REST:** `POST/GET/DELETE /api/application/{id}/document` (admin vs. non-admin, MIME-Allowlist-Reject 415, Body-Limit-Reject, Replace-Verhalten).
- **E2E genossi_bin:** Full flow „Upload → confirm → MemberDocument sichtbar am Member + `application_document.deleted != NULL` + Audit-Row exists".
- **Audit-Hashchain:** unverändert valid nach `confirm` (bestehender `/api/audit/verify` grün).
- **CR-02-Regression:** Test dass unautorisierter Aufruf **weder** Info-Leak noch Side-Effect erzeugt (permission check happens first).

</decisions>

<deferred>
- **Housekeeping-Job für verwaiste Application-Files** (Best-Effort-Delete kann bei Storage-Ausfall Waisen hinterlassen). Deferred → separate Quick oder Phase in v1.5+.
- **Multi-File pro Application** (z. B. Vorstands-Notizen, Rückfragen-Dokumente) — bewusst außerhalb Scope, Single-Slot reicht für Original-Antrag.
- **Application-Detail „Historie" nach `confirm`** (Anzeige „ursprünglich lag hier eine Datei, jetzt beim Mitglied"). Nice-to-have, nicht Phase-25.
- **CR-02 projektweit als `gen_auth_admin!`-Helper extrahieren** — bleibt Carry-Forward-Techdebt (v1.2-MILESTONE-AUDIT), separater Refactor-Phase-Kandidat.
- **Drag-and-Drop-Upload** auf der Application-Detail-Page — MVP: klassischer File-Dialog reicht.
</deferred>

<open_questions>
- **REQUIREMENTS.md-Wortlaut-Update APDOC-03** — Wer/wann? Vorschlag: im Rahmen der Phase-25-Execution als Doku-Commit mit-erledigen (nicht in eigenem Ticket versauern lassen). Planer erhält den Auftrag über CONTEXT.md; wenn du das lieber vorher machst, sag Bescheid.
</open_questions>

<next_steps>
`/gsd-plan-phase 25`

Der Planer soll:
1. Migrations-Plan für `application_documents`-Tabelle (SQLx-Migration in `migrations/sqlite/`)
2. DAO-Layer (`ApplicationDocumentDao` Trait + SQLite-Impl, **kein `Auditable`**)
3. Service-Layer (`ApplicationDocumentService` Trait + Impl mit `upload/replace/get/delete`, **CR-02-Ordering**)
4. Service-Layer-Erweiterung: `ApplicationServiceImpl::confirm()` um Storage-Move + `audited_create!(MemberDocument)` erweitern
5. REST-Layer (3 Endpoints, admin-only, Multipart-Muster aus `member_document.rs` wiederverwenden)
6. Frontend: `ApplicationDocumentSlot`-Component + Integration in Application-Detail-Page
7. Tests: Unit (Service Move-Sequenz), Integration (REST), E2E (bin) — inkl. CR-02-Regressions-Test
8. **Doku-Fix APDOC-03** in REQUIREMENTS.md („übernommen / Ownership-Übergabe")
</next_steps>
</content>
</invoke>