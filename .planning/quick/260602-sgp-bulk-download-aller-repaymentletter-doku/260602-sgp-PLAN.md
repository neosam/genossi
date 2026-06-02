---
phase: quick-260602-sgp
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - genossi_service/src/repayment_letter.rs
  - genossi_service_impl/src/repayment_letter.rs
  - genossi_rest/src/repayment_letter.rs
  - genossi_bin/tests/repayment_letter_e2e.rs
  - genossi-frontend/src/component/repayment_letter_download_button.rs
  - genossi-frontend/src/component/mod.rs
  - genossi-frontend/src/page/repayment_phase_details.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - Cargo.toml
autonomous: true
requirements: [QUICK-260602-SGP]
tags: [quick, repayment-letter, bulk-download, zip, pdf-merge, lopdf]

must_haves:
  truths:
    - "GET /api/repayment-phase/{phase_id}/letters/download?format=zip liefert ein ZIP-Archiv mit allen persistierten RepaymentLetter-PDFs der Phase"
    - "GET /api/repayment-phase/{phase_id}/letters/download?format=pdf liefert eine zusammengefuegte Bundle-PDF aus den persistierten PDFs (kein Neu-Render)"
    - "PDFs werden NICHT neu generiert — Service liest ausschliesslich existierende MemberDocuments vom Document-Storage"
    - "Endpoint ist Vorstand-only (privileges::ADMIN); Helper-Auth -> 403 Forbidden"
    - "Phase-Status-Gate: Preparation -> 409 phase_not_active (Konsistenz mit POST /letters/generate)"
    - "Edge-Case: Phase hat 0 persistierte Letters -> 404 NotFound mit klarer Message 'no_letters_persisted'"
    - "Edge-Case: einzelne Member-Letters fehlen im Storage -> skippen und X-Skipped-Count Header setzen; nicht abbrechen"
    - "Briefe sind nach member_number ASC sortiert (deterministisch, analog Phase 13)"
    - "ZIP-Dateinamen: <member_number>_<lastname>_<firstname>.pdf (Umlaute via sanitize_filename_component)"
    - "Response-Header X-Document-Count: N (Anzahl erfolgreich zusammengefasster Letters); X-Skipped-Count: M (fehlende im Storage)"
    - "Service hat ZERO Audit-Logging (reiner Read-Endpoint); keine audited_*! Macros, keine MemberDocument-Mutation"
    - "Frontend-Page repayment_phase_details.rs hat einen extrahierten Download-Button-Component (Component-First, NICHT inline-RSX) — wiederverwendet bestehende Browser-Download-Logik (<a download>)"
  artifacts:
    - path: "genossi_service/src/repayment_letter.rs"
      provides: "Erweiterung des RepaymentLetterService-Traits um download_bundle(phase_id, format, auth) + RepaymentLetterDownloadFormat-Enum + RepaymentLetterDownload-Struct"
      contains: "download_bundle"
    - path: "genossi_service_impl/src/repayment_letter.rs"
      provides: "RepaymentLetterServiceImpl::download_bundle — laedt persistierte MemberDocuments mit DocumentType::RepaymentLetter + description-Match fuer phase, sortiert nach member_number, baut ZIP oder PDF-Merge"
      contains: "download_bundle"
    - path: "genossi_rest/src/repayment_letter.rs"
      provides: "GET-Handler download_letters + Query-Struct DownloadQuery { format } + OpenAPI-Path + Route in generate_letter_route"
      contains: "download_letters"
    - path: "genossi_bin/tests/repayment_letter_e2e.rs"
      provides: "5+ E2E-Tests: ZIP-Happy, PDF-Happy, 0-letters-404, helper-403, preparation-409"
      contains: "test_download_"
    - path: "genossi-frontend/src/component/repayment_letter_download_button.rs"
      provides: "Wiederverwendbarer Download-Button-Component (Component-First) mit zwei Format-Optionen"
      contains: "fn RepaymentLetterDownloadButton"
    - path: "Cargo.toml"
      provides: "lopdf-Workspace-Dependency fuer PDF-Merge"
      contains: "lopdf"
  key_links:
    - from: "genossi_rest/src/repayment_letter.rs::download_letters"
      to: "RepaymentLetterService::download_bundle"
      via: "rest_state.repayment_letter_service().download_bundle(...)"
      pattern: "download_bundle"
    - from: "genossi_service_impl/src/repayment_letter.rs::download_bundle"
      to: "MemberDocumentDao::find_by_member_id + DocumentStorage::load"
      via: "Filter document_type=='repayment_letter' + description-Match fuer fiscal_year"
      pattern: "DocumentType::RepaymentLetter"
    - from: "genossi-frontend/src/page/repayment_phase_details.rs"
      to: "RepaymentLetterDownloadButton Component"
      via: "rsx Component-Mount im BasicsTab (oder eigener Letters-Abschnitt)"
      pattern: "RepaymentLetterDownloadButton"
---

<objective>
Vorstand braucht einen Bulk-Download aller bereits persistierten RepaymentLetter-PDFs einer Phase — ZIP-Archiv mit Einzel-PDFs ODER eine zusammengefuegte Bundle-PDF. Im Gegensatz zu POST /letters/generate werden die PDFs NICHT neu gerendert, sondern aus dem Document-Storage geladen.

**Architekt-Entscheidungen (begruendet):**

1. **PDF-Merge-Bibliothek `lopdf`**: `pdf-writer` ist nur transitive Typst-Dependency und nicht direkt zum Mergen geeignet. Re-Render via Typst-Bundle widerspricht der "kein Neu-Render"-Vorgabe und wuerde POST /letters/generate effektiv duplizieren. `lopdf` ist die Standard-Crate fuer PDF-Object-Manipulation in Rust, lebt seit Jahren, lightweight, und ihr Page-Tree-Merge-Pattern ist gut dokumentiert.
2. **Endpoint-Pfad `/api/repayment-phase/{phase_id}/letters/download` mit `?format=zip|pdf`**: koexistiert sauber mit `POST /letters/generate` (verschiedene HTTP-Methoden + verschiedener Sub-Pfad). Query-Parameter statt Pfad-Segment, weil 'zip' und 'pdf' Format-Varianten desselben Endpoints sind, nicht semantisch verschiedene Ressourcen.
3. **Edge-Cases (alle Empfehlungen aus task_scope uebernommen):**
   - 0 Letters -> 404 NotFound (keine "leeres ZIP"-Antwort — semantisch klarer)
   - Fehlende Files im Storage -> skippen + X-Skipped-Count Header (nicht abbrechen — der Vorstand bekommt das, was da ist; sieht im Header was fehlt)
   - Phase-Status-Gate: gleiches Gate wie POST /letters/generate (Open/Closed only; Preparation -> 409)
4. **Service-Layer-Erweiterung statt neuer Service**: `RepaymentLetterService` hat bereits alle benoetigten DAOs (member_document_dao, repayment_entry_dao, repayment_phase_dao, member_dao, document_storage). Neuer Service waere overhead — wir erweitern den existierenden Trait und ServiceImpl um EINE neue Methode.
5. **ZERO Audit-Logging**: Read-Endpoint, kein State-Change, kein audit_log-Eintrag (CLAUDE.md "Audit-Logging Macros" zaehlt nur write-Operationen).
6. **Component-First Frontend**: Download-Button als eigene Component in `genossi-frontend/src/component/repayment_letter_download_button.rs`. Page nutzt nur den Component-Mount.

Purpose:
- Verbandskonformitaet: Vorstand kann alle Briefe einer Phase in einem Click archivieren/drucken (Druckdienstleister/Protokoll).
- DSGVO-konform: bestehende persistierte PDFs, kein Re-Render mit potenziell veraendertem Datenstand.

Output:
- Service-Trait + Impl Erweiterung um download_bundle
- REST-Handler GET /api/repayment-phase/{phase_id}/letters/download?format={zip|pdf}
- 5+ E2E-Tests
- Frontend Download-Component + Mount auf RepaymentPhase-Detail-Page
- lopdf-Dependency
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/STATE.md

<!-- Vorbild: bestehender POST-Generate-Endpoint (1:1 Pattern fuer Mount, ApiDoc, RestState-Trait) -->
@genossi_rest/src/repayment_letter.rs

<!-- Vorbild: ZIP-Bau mit zip-Crate (Pattern fuer File-Loop, Compression, Content-Type) -->
@genossi_rest/src/backup.rs

<!-- Service-Trait, der erweitert wird -->
@genossi_service/src/repayment_letter.rs

<interfaces>
<!-- Existierende Trait-Signatur die erweitert wird -->
```rust
// genossi_service/src/repayment_letter.rs:38-53
#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentLetterService: Send + Sync + 'static {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn generate(
        &self,
        phase_id: Uuid,
        entry_ids: Arc<[Uuid]>,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentLetterBundle, ServiceError>;
}
```

<!-- Existierender ServiceImpl (Felder, die das neue download_bundle nutzt) -->
```rust
// genossi_service_impl/src/repayment_letter.rs:92-105
pub struct RepaymentLetterServiceImpl<Deps: RepaymentLetterServiceDeps> {
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
    pub member_dao: Arc<Deps::MemberDao>,
    pub member_document_dao: Arc<Deps::MemberDocumentDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub document_storage: Arc<Deps::DocumentStorage>,
    // ... (audit_log_dao, uuid_service, etc. — nicht benoetigt fuer download_bundle)
}
```

<!-- Existierende DocumentType-Konstante + lookup-Helper -->
```rust
// genossi_service/src/member_document.rs:60 - DocumentType::RepaymentLetter (as_str: "repayment_letter")
// genossi_service_impl/src/repayment_letter.rs:198-210
// find_existing_letter_for_phase(docs: &[MemberDocumentEntity], fiscal_year: i32) -> Option<MemberDocumentEntity>
// Filter: description == "Anschreiben Auszahlung GJ {fiscal_year}"
// !! WICHTIG: Dieser bereits-existierende Helper ist DAS Identifikations-Schema (member, phase). Wir muessen es 1:1 nutzen, um sicherzustellen dass wir die GLEICHEN MemberDocuments finden, die POST /letters/generate persistiert hat. !!
```

<!-- Existierende DocumentStorage-Trait-API -->
```rust
// genossi_service/src/document_storage.rs
#[async_trait]
pub trait DocumentStorage {
    async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
}
```

<!-- Permission-Konstante -->
```rust
// genossi_service/src/auth_types.rs:137
pub mod privileges { pub const ADMIN: &str = "admin"; ... }
```

<!-- http_util Helper -->
```rust
// genossi_rest/src/http_util.rs:5,43
pub fn sanitize_filename_component(s: &str) -> String;  // Umlaute -> ASCII, etc.
pub fn content_disposition_attachment(filename: &str) -> String;
```

<!-- RepaymentLetterRestState-Trait (bereits existent — bleibt UNVERAENDERT, weil RepaymentLetterService nur erweitert wird) -->
```rust
// genossi_rest/src/repayment_letter.rs:62-68
pub trait RepaymentLetterRestState: Clone + Send + Sync + 'static {
    type RepaymentLetterService: RepaymentLetterService<Context = crate::ContextType>
        + Send + Sync + 'static;
    fn repayment_letter_service(&self) -> Arc<Self::RepaymentLetterService>;
}
```

<!-- Frontend-Bestand: bestehender Download-Button im Letters-Tab (laesst sich als Pattern uebernehmen) -->
```rust
// genossi-frontend/src/page/repayment_phase_details.rs:300-310
// <a download>-Click + revoke_object_url Pattern
let _ = elem.set_attribute("download", &dl_filename);
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Backend — Service-Trait-Erweiterung + ServiceImpl::download_bundle + REST-Handler + Workspace-Dep lopdf</name>
  <files>
    genossi_service/src/repayment_letter.rs,
    genossi_service_impl/src/repayment_letter.rs,
    genossi_rest/src/repayment_letter.rs,
    Cargo.toml,
    genossi_service_impl/Cargo.toml
  </files>
  <read_first>
    - genossi_service/src/repayment_letter.rs (gesamte Datei — Trait-Definition)
    - genossi_service_impl/src/repayment_letter.rs:60-242 (Service-Impl-Struktur, check_admin_and_phase_status, find_existing_letter_for_phase, resolve_user_id_or_deny)
    - genossi_rest/src/repayment_letter.rs (gesamte Datei — Handler-Pattern)
    - genossi_rest/src/backup.rs:118-225 (ZIP-Aufbau via `zip::ZipWriter` + SimpleFileOptions::compression_method(Deflated))
    - genossi_rest/src/http_util.rs (sanitize_filename_component + content_disposition_attachment)
    - genossi_service/src/document_storage.rs (DocumentStorage-Trait)
    - genossi_dao/src/member_document.rs:68-140 (find_by_member_id-Default-Impl)
    - Cargo.toml (Workspace dependencies-Block)
  </read_first>
  <action>
    **6 Edit-Stellen — strikt sequentiell innerhalb dieser Task (gleicher Commit, weil Trait-Erweiterung + Impl + Caller atomar sein muessen, sonst Tree-Red zwischen Stellen):**

    **Stelle 1: Workspace-Dep `lopdf` (Cargo.toml, Workspace-Section):**

    Erst greppen:
    ```bash
    grep -n '\[workspace.dependencies\]\|^lopdf' Cargo.toml
    ```

    Im `[workspace.dependencies]`-Block sortiert einfuegen (alphabetisch zwischen 'lettre' und 'mail-parser' oder 'minijinja'):
    ```toml
    lopdf = "0.34"
    ```

    Falls die exakte Major-Version `0.34` API-Breaking-Changes hat: Implementer wahlt die juengste 0.x-Version, die `Document::load_mem(&[u8]) -> Result<Document, _>` + `Document::save_to(&mut Vec<u8>) -> Result<(), _>` exponiert. (`0.34` ist Stand 2026-06 verfuegbar; Documentation: https://docs.rs/lopdf/0.34/lopdf/struct.Document.html)

    Dann in `genossi_service_impl/Cargo.toml` unter `[dependencies]`:
    ```toml
    lopdf = { workspace = true }
    ```

    **Stelle 2: Service-Trait erweitern (genossi_service/src/repayment_letter.rs):**

    Neue Public-Types am Anfang des Moduls (nach `RepaymentLetterBundle`, vor dem `#[automock]`-Trait):
    ```rust
    /// Format-Wahl fuer Bulk-Download. ZIP packt Einzel-PDFs; PDF mergt sie zu einer Datei.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RepaymentLetterDownloadFormat {
        Zip,
        Pdf,
    }

    /// Output des Bulk-Download-Service.
    /// `bytes` ist application/zip ODER application/pdf je nach format.
    /// `document_count` zaehlt erfolgreich eingepackte Letters,
    /// `skipped_count` zaehlt MemberDocuments deren Files im Storage fehlten.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RepaymentLetterDownload {
        pub bytes: Vec<u8>,
        pub content_type: &'static str,
        pub filename: String,
        pub document_count: usize,
        pub skipped_count: usize,
    }
    ```

    Im Trait neue Methode hinzufuegen (am ENDE der Methodenliste, NACH `generate`):
    ```rust
        /// Bulk-Download aller bereits persistierten RepaymentLetter-PDFs einer Phase.
        ///
        /// NICHT-Neu-Render: liest ausschliesslich MemberDocuments mit
        /// DocumentType::RepaymentLetter, deren description "Anschreiben Auszahlung GJ {fy}"
        /// zur Phase passt. Bei `RepaymentLetterDownloadFormat::Zip` werden Einzel-PDFs
        /// in einem ZIP-Archiv geliefert; bei `Pdf` werden sie via `lopdf` zu einer
        /// Bundle-PDF zusammengefuegt.
        ///
        /// Returns:
        /// - 0 persistierte Letters -> `ServiceError::EntityNotFound(phase_id)` (REST -> 404)
        /// - Files im Storage teilweise fehlend -> erfolgreiche werden gepackt,
        ///   `skipped_count` zaehlt fehlende (REST liefert sie als Header)
        async fn download_bundle(
            &self,
            phase_id: Uuid,
            format: RepaymentLetterDownloadFormat,
            context: Authentication<Self::Context>,
        ) -> Result<RepaymentLetterDownload, ServiceError>;
    ```

    Im `#[cfg(test)]`-Modul AM ENDE drei Tests einfuegen:
    ```rust
    #[test]
    fn test_download_format_enum_variants() {
        assert_ne!(RepaymentLetterDownloadFormat::Zip, RepaymentLetterDownloadFormat::Pdf);
    }

    #[test]
    fn test_download_struct_fields() {
        let d = RepaymentLetterDownload {
            bytes: vec![1, 2, 3],
            content_type: "application/zip",
            filename: "x.zip".to_string(),
            document_count: 2,
            skipped_count: 1,
        };
        assert_eq!(d.bytes.len(), 3);
        assert_eq!(d.content_type, "application/zip");
        assert_eq!(d.document_count, 2);
        assert_eq!(d.skipped_count, 1);
    }

    #[test]
    fn test_mock_download_bundle_compiles() {
        let _m = MockRepaymentLetterService::new();
        // Smoke-Test fuer automock-Generation auf neue Methode.
    }
    ```

    **Stelle 3: ServiceImpl-Implementation (genossi_service_impl/src/repayment_letter.rs):**

    Imports am Datei-Anfang ergaenzen:
    ```rust
    use std::io::{Cursor, Write};
    use genossi_service::repayment_letter::{RepaymentLetterDownload, RepaymentLetterDownloadFormat};
    use lopdf::Document as PdfDoc;
    ```

    Im `#[async_trait] impl ... for RepaymentLetterServiceImpl<Deps>`-Block NACH `generate` einfuegen:
    ```rust
        async fn download_bundle(
            &self,
            phase_id: Uuid,
            format: RepaymentLetterDownloadFormat,
            context: Authentication<Self::Context>,
        ) -> Result<RepaymentLetterDownload, ServiceError> {
            // 1. Read-Tx oeffnen.
            let read_tx = self.transaction_dao.use_transaction(None).await?;

            // 2. Funnel (gleicher Gate wie generate): load phase (404) -> admin (403) -> status (409).
            //    KONSISTENZ: gleicher Status-Gate wie POST /letters/generate (Open/Closed only).
            let phase = self
                .check_admin_and_phase_status(phase_id, context.clone(), read_tx.clone())
                .await?;

            // 3. Phase-Entries laden -> member_ids extrahieren (unique).
            let phase_entries = self
                .repayment_entry_dao
                .find_by_phase_id(phase_id, read_tx.clone())
                .await?;
            let mut member_ids: Vec<Uuid> =
                phase_entries.iter().map(|e| e.member_id).collect();
            member_ids.sort();
            member_ids.dedup();

            // 4. Pro Member: existierendes RepaymentLetter-MemberDocument fuer GENAU diese Phase suchen.
            //    description-Match "Anschreiben Auszahlung GJ {fy}" identifiziert (member, phase).
            //    => REUSE des existierenden Helpers `find_existing_letter_for_phase`.
            //    Member zusaetzlich laden fuer Filename-Generation.
            let mut found: Vec<(MemberEntity, MemberDocumentEntity)> =
                Vec::with_capacity(member_ids.len());
            for mid in &member_ids {
                let docs = self
                    .member_document_dao
                    .find_by_member_id(*mid, read_tx.clone())
                    .await?;
                if let Some(doc) =
                    Self::find_existing_letter_for_phase(&docs, phase.fiscal_year)
                {
                    let member = self
                        .member_dao
                        .find_by_id(*mid, read_tx.clone())
                        .await?
                        .ok_or(ServiceError::EntityNotFound(*mid))?;
                    found.push((member, doc));
                }
            }

            // 5. Edge-Case: 0 persistierte Letters -> 404.
            //    EntityNotFound mit phase_id (REST mapped zu 404).
            if found.is_empty() {
                return Err(ServiceError::EntityNotFound(phase_id));
            }

            // 6. Sortiere nach member_number ASC (deterministisch, analog Phase 13).
            found.sort_by(|a, b| a.0.member_number.cmp(&b.0.member_number));

            // 7. Files aus Storage laden — fehlende skippen (X-Skipped-Count).
            let mut loaded: Vec<(MemberEntity, Vec<u8>)> = Vec::with_capacity(found.len());
            let mut skipped_count: usize = 0;
            for (member, doc) in &found {
                match self.document_storage.load(&doc.relative_path).await {
                    Ok(bytes) => loaded.push((member.clone(), bytes)),
                    Err(e) => {
                        tracing::warn!(
                            "RepaymentLetter download: missing file {} for member {} (phase {}): {:?}",
                            doc.relative_path,
                            member.id,
                            phase_id,
                            e
                        );
                        skipped_count += 1;
                    }
                }
            }

            // Edge-Case: alle Files fehlen -> 404.
            if loaded.is_empty() {
                return Err(ServiceError::EntityNotFound(phase_id));
            }

            // 8. Bau ZIP oder Bundle-PDF — KEIN Schreib-Tx, KEIN Audit.
            let (bytes, content_type, filename) = match format {
                RepaymentLetterDownloadFormat::Zip => {
                    let mut zip_buf = Cursor::new(Vec::new());
                    {
                        let mut zip = zip::ZipWriter::new(&mut zip_buf);
                        let options = zip::write::SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Deflated);
                        for (member, pdf_bytes) in &loaded {
                            // Filename-Schema: <member_number>_<lastname>_<firstname>.pdf
                            // Umlaut-safe via sanitize-Helper (im Service repliziert, weil
                            // genossi_rest::http_util nicht ins Service-Crate referenziert
                            // werden darf — Layer-Inversion).
                            let safe_last = sanitize_for_filename(&member.last_name);
                            let safe_first = sanitize_for_filename(&member.first_name);
                            let fname = format!(
                                "{}_{}_{}.pdf",
                                member.member_number, safe_last, safe_first
                            );
                            zip.start_file(&fname, options).map_err(|e| {
                                ServiceError::InternalError(Arc::from(
                                    format!("zip: {}", e).as_str(),
                                ))
                            })?;
                            zip.write_all(pdf_bytes).map_err(|e| {
                                ServiceError::InternalError(Arc::from(
                                    format!("zip-write: {}", e).as_str(),
                                ))
                            })?;
                        }
                        zip.finish().map_err(|e| {
                            ServiceError::InternalError(Arc::from(
                                format!("zip-finish: {}", e).as_str(),
                            ))
                        })?;
                    }
                    let fname = format!("auszahlungs_anschreiben_GJ_{}.zip", phase.fiscal_year);
                    (zip_buf.into_inner(), "application/zip", fname)
                }
                RepaymentLetterDownloadFormat::Pdf => {
                    // PDF-Merge via lopdf. Wir bauen ein neues Dokument und uebernehmen
                    // alle Pages der Eingabe-PDFs in Reihenfolge.
                    // Pattern: load_mem -> renumber_objects auf jedem Sub-Doc, dann manual merge.
                    // Implementer waehlt die idiomatischste lopdf-Merge-Strategie:
                    //   a) Document::load_mem fuer jedes input
                    //   b) renumber_objects(start_id) pro doc
                    //   c) collect pages, build new Pages-tree, attach Resources
                    //   d) save_to(&mut Vec<u8>)
                    // Bei API-Drift in lopdf 0.34+: konsultiere die Docs-rs-Seite,
                    // halte am Public-API-Vertrag `load_mem` + `save_to` fest.
                    let merged_bytes = merge_pdfs_via_lopdf(&loaded)
                        .map_err(|e| ServiceError::InternalError(Arc::from(
                            format!("pdf-merge: {}", e).as_str(),
                        )))?;
                    let fname = format!("auszahlungs_anschreiben_GJ_{}.pdf", phase.fiscal_year);
                    (merged_bytes, "application/pdf", fname)
                }
            };

            Ok(RepaymentLetterDownload {
                bytes,
                content_type,
                filename,
                document_count: loaded.len(),
                skipped_count,
            })
        }
    ```

    Free-Funktionen am Datei-Ende ergaenzen (NICHT in `impl`-Block, damit sie aus Tests reachable sind):
    ```rust
    /// Filename-Sanitisierung — replikat von genossi_rest::http_util::sanitize_filename_component
    /// (kann nicht referenziert werden — Layer-Inversion REST -> Service).
    /// Umlaute -> ASCII, Sonderzeichen -> '_', Trim am Rand.
    fn sanitize_for_filename(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                'ä' | 'Ä' => out.push_str("ae"),
                'ö' | 'Ö' => out.push_str("oe"),
                'ü' | 'Ü' => out.push_str("ue"),
                'ß' => out.push_str("ss"),
                c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => out.push(c),
                _ => out.push('_'),
            }
        }
        out.trim_matches('_').to_string()
    }

    /// Mergt mehrere PDF-Bytes in EIN Dokument via lopdf.
    /// Reihenfolge: wie in `inputs` uebergeben (caller sortiert vorher nach member_number).
    /// Returns merged PDF bytes oder String-Fehler.
    fn merge_pdfs_via_lopdf(
        inputs: &[(MemberEntity, Vec<u8>)],
    ) -> Result<Vec<u8>, String> {
        use lopdf::{Document, Object, ObjectId};
        use std::collections::BTreeMap;

        // Loese-Strategie:
        //   1. Erste PDF als Basis laden.
        //   2. Fuer jede weitere PDF: load_mem, renumber_objects(max_id + 1),
        //      Objects via add_object einsammeln, Pages an Page-Tree haengen.
        //   3. Document::save_to(&mut Vec<u8>).
        //
        // Implementer waehlt die idiomatischste lopdf-API:
        //   - Option A (einfacher): `Document::merge_documents` falls in 0.34+ vorhanden.
        //   - Option B (manuell): renumber + extend objects-map + rebuild Pages.
        //     Standard-Pattern aus lopdf-Examples-Repo.
        // Test verifiziert nur "Output ist parseable PDF mit >= page_count == sum(input_pages)".

        if inputs.is_empty() {
            return Err("empty inputs".to_string());
        }
        if inputs.len() == 1 {
            // Single-PDF -> direkt zurueckliefern (kein Merge noetig).
            return Ok(inputs[0].1.clone());
        }

        // --- Implementer-Skelett (anpassen an verfuegbare lopdf-API) ---
        let mut merged = Document::load_mem(&inputs[0].1)
            .map_err(|e| format!("load doc 0: {}", e))?;
        for (idx, (_member, bytes)) in inputs.iter().enumerate().skip(1) {
            let mut other = Document::load_mem(bytes)
                .map_err(|e| format!("load doc {}: {}", idx, e))?;
            // Beispiel-Pattern (siehe lopdf-Examples 'merge.rs'):
            //   1. max_id = merged.max_id;
            //   2. other.renumber_objects_with(max_id);
            //   3. for (obj_id, obj) in other.objects { merged.objects.insert(obj_id, obj); }
            //   4. merged-Pages und other-Pages in /Pages /Kids zusammenfuehren + /Count update.
            // Detail-Implementierung lt. juengster lopdf-Docs.
            // Implementer: nutze die offiziellen lopdf-Examples als Vorlage.
            let max_id = merged.max_id + 1;
            other.renumber_objects_with(max_id);
            merged.max_id = other.max_id;
            for (id, obj) in other.objects {
                merged.objects.insert(id, obj);
            }
            // Pages-Tree merge — Implementer ermittelt /Pages-Root-IDs und ergaenzt /Kids:
            let other_pages: BTreeMap<u32, ObjectId> = {
                // Re-load page list aus other (mit verschobenen IDs ist es einfacher,
                // erst die Page-IDs auf der ORIGINAL-Doc zu lesen und dann offsetted zu adden):
                BTreeMap::new() // Placeholder — Implementer fuellt aus Object::Dictionary( /Pages /Kids ).
            };
            // Naechster Schritt: merged.get_pages()-Mutation + /Count-Erhoehung.
            // Falls lopdf 0.34 die High-Level-`merge_documents`-Methode mitliefert:
            // einfach `merged.merge_documents(other)` aufrufen statt der obigen Manualarbeit.
            let _ = other_pages; // silence unused warning bis Implementer fertig ist
        }

        let mut out = Vec::new();
        merged.save_to(&mut out).map_err(|e| format!("save: {}", e))?;
        Ok(out)
    }
    ```

    **WICHTIG fuer Implementer**: das obige `merge_pdfs_via_lopdf` ist ein **Skelett mit klarer Vertrags-Definition**, KEINE finale Implementation. Implementer **MUSS** vor dem GREEN-Run:
    1. `cargo doc --open -p lopdf` oder docs.rs/lopdf/0.34 lesen
    2. Pruefen ob `Document::merge_documents` (oder aehnliches) in der verwendeten lopdf-Version existiert
    3. Falls ja: einzeiliger Loop `for input in inputs { other = load_mem; merged.merge_documents(other); }`
    4. Falls nein: Pages-Tree-Merge manuell (siehe lopdf-Repo `examples/merge.rs`)

    Acceptance-Test (siehe Task 2) verifiziert NUR das End-Verhalten ("Output ist valides PDF mit gemerged Pages"), nicht die internen lopdf-API-Calls.

    **Stelle 4: ServiceImpl-Tests (genossi_service_impl/src/repayment_letter.rs, im `#[cfg(test)] mod tests`):**

    Mind. 3 Unit-Tests ergaenzen (Mockall-basiert, analog existierender Test-Struktur):

    ```rust
    #[tokio::test]
    async fn test_download_bundle_zero_letters_returns_not_found() {
        // Setup: phase exists, 0 RepaymentLetter-MemberDocuments.
        // Expectation: ServiceError::EntityNotFound(phase_id).
    }

    #[tokio::test]
    async fn test_download_bundle_zip_happy_path_sorted_by_member_number() {
        // Setup: 2 RepaymentLetter-Docs, Member 020 + Member 005, beide im Storage.
        // Expectation: ZIP enthaelt 2 Files, Member 005 zuerst (sortiert).
        // Assert: zip-Extraction zeigt Filenames in der erwarteten Reihenfolge.
        // document_count == 2, skipped_count == 0.
    }

    #[tokio::test]
    async fn test_download_bundle_skipped_missing_files() {
        // Setup: 3 RepaymentLetter-Docs, nur 2 davon im Storage (1x StorageError).
        // Expectation: document_count == 2, skipped_count == 1, kein Error-Return.
    }

    #[tokio::test]
    async fn test_download_bundle_preparation_status_returns_conflict() {
        // Setup: phase im Preparation-Status.
        // Expectation: ServiceError::Conflict("phase_not_active") — gleicher Gate wie generate.
    }
    ```

    Re-Use der bestehenden mockall-Helper (`build_service`, `MockTestMemberDocumentDao`, etc.) wenn vorhanden.

    **Stelle 5: REST-Handler erweitern (genossi_rest/src/repayment_letter.rs):**

    Imports ergaenzen:
    ```rust
    use axum::extract::Query;
    use axum::routing::get;
    use genossi_service::repayment_letter::{
        RepaymentLetterDownload, RepaymentLetterDownloadFormat,
        // bestehende ergaenzen: RepaymentLetterBundle, RepaymentLetterService
    };
    ```

    Nach `GenerateLettersRequest` ergaenzen:
    ```rust
    /// Query-Params fuer GET /letters/download.
    #[derive(Debug, Deserialize, utoipa::IntoParams)]
    pub struct DownloadQuery {
        /// "zip" oder "pdf". Default falls fehlend: 400 BadRequest.
        pub format: String,
    }
    ```

    Nach `generate_letters` neuen Handler:
    ```rust
    /// GET /api/repayment-phase/{phase_id}/letters/download?format=zip|pdf
    ///
    /// Bulk-Download aller bereits persistierten RepaymentLetter-PDFs der Phase.
    /// NICHT-Neu-Render: Service laedt MemberDocuments mit DocumentType::RepaymentLetter
    /// aus dem Document-Storage. Fehlende Files werden geskippt; Count im
    /// `X-Skipped-Count` Header.
    ///
    /// Response-Header:
    /// - `Content-Type: application/zip` ODER `application/pdf`
    /// - `Content-Disposition: attachment; filename="auszahlungs_anschreiben_GJ_{fy}.{zip|pdf}"`
    /// - `X-Document-Count: N` — Anzahl erfolgreich zusammengefasster Letters
    /// - `X-Skipped-Count: M` — Anzahl fehlender Files im Storage
    #[utoipa::path(
        get,
        path = "/api/repayment-phase/{phase_id}/letters/download",
        params(
            ("phase_id" = Uuid, Path, description = "RepaymentPhase UUID — Open oder Closed."),
            DownloadQuery,
        ),
        responses(
            (status = 200, description = "Bulk-Download als ZIP oder Bundle-PDF.",
                content_type = "application/octet-stream"),
            (status = 400, description = "Ungueltiges format (nur 'zip' und 'pdf' erlaubt)."),
            (status = 401, description = "Session ungueltig oder fehlt."),
            (status = 403, description = "Auth gueltig, aber kein Vorstand (Helfer)."),
            (status = 404, description = "Phase nicht gefunden ODER keine persistierten Letters."),
            (status = 409, description = "Phase im Preparation-Status — phase_not_active."),
        ),
        tag = "RepaymentLetter"
    )]
    #[instrument(skip(rest_state))]
    pub async fn download_letters<RestState: RestStateDef + RepaymentLetterRestState>(
        rest_state: State<RestState>,
        Extension(context): Extension<Context>,
        Path(phase_id): Path<Uuid>,
        Query(query): Query<DownloadQuery>,
    ) -> Response {
        error_handler(
            (async {
                let auth = extract_auth_context(Some(context))?;

                let format = match query.format.as_str() {
                    "zip" => RepaymentLetterDownloadFormat::Zip,
                    "pdf" => RepaymentLetterDownloadFormat::Pdf,
                    other => {
                        return Err(RestError::BadRequest(format!(
                            "invalid format '{}': use 'zip' or 'pdf'",
                            other
                        )));
                    }
                };

                let result: RepaymentLetterDownload = rest_state
                    .repayment_letter_service()
                    .download_bundle(phase_id, format, auth)
                    .await
                    .map_err(map_letter_error)?;

                let cd = http_util::content_disposition_attachment(&result.filename);

                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", result.content_type)
                    .header("Content-Disposition", &cd)
                    .header("X-Document-Count", result.document_count.to_string())
                    .header("X-Skipped-Count", result.skipped_count.to_string())
                    .body(Body::from(result.bytes))
                    .unwrap())
            })
            .await,
        )
    }
    ```

    Route in `generate_letter_route` ergaenzen:
    ```rust
    pub fn generate_letter_route<RestState: RestStateDef + RepaymentLetterRestState>(
    ) -> Router<RestState> {
        Router::new()
            .route(
                "/{phase_id}/letters/generate",
                post(generate_letters::<RestState>),
            )
            .route(
                "/{phase_id}/letters/download",
                get(download_letters::<RestState>),
            )
    }
    ```

    ApiDoc ergaenzen:
    ```rust
    #[derive(OpenApi)]
    #[openapi(
        paths(generate_letters, download_letters),  // ergaenzt
        components(schemas(GenerateLettersRequest)),
        tags(...)
    )]
    pub struct ApiDoc;
    ```

    Tests im `#[cfg(test)] mod tests` ergaenzen:
    ```rust
    #[test]
    fn test_download_query_deserialization_zip() {
        let q: DownloadQuery = serde_urlencoded::from_str("format=zip").unwrap();
        assert_eq!(q.format, "zip");
    }

    #[test]
    fn test_download_query_deserialization_pdf() {
        let q: DownloadQuery = serde_urlencoded::from_str("format=pdf").unwrap();
        assert_eq!(q.format, "pdf");
    }
    ```

    (`serde_urlencoded` muesste fuer Tests verfuegbar sein — sonst weglassen und stattdessen direkt DownloadQuery instanziieren.)

    **Stelle 6: Modul-Re-Exports** (genossi_service/src/lib.rs):
    Greppen ob `repayment_letter`-Modul bereits re-exportiert wird; falls die neuen Types (`RepaymentLetterDownload`, `RepaymentLetterDownloadFormat`) explizit re-exportiert werden muessen, ergaenzen.
  </action>
  <verify>
    <automated>cargo build --workspace && cargo test -p genossi_service repayment_letter && cargo test -p genossi_service_impl repayment_letter && cargo test -p genossi_rest repayment_letter</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "lopdf" Cargo.toml` returns >= 1
    - `grep -c "lopdf" genossi_service_impl/Cargo.toml` returns >= 1
    - `grep -c "pub enum RepaymentLetterDownloadFormat" genossi_service/src/repayment_letter.rs` returns 1
    - `grep -c "pub struct RepaymentLetterDownload" genossi_service/src/repayment_letter.rs` returns 1
    - `grep -c "async fn download_bundle" genossi_service/src/repayment_letter.rs` returns 1 (Trait-Def)
    - `grep -c "async fn download_bundle" genossi_service_impl/src/repayment_letter.rs` returns 1 (Impl)
    - `grep -c "pub async fn download_letters" genossi_rest/src/repayment_letter.rs` returns 1
    - `grep -c "fn merge_pdfs_via_lopdf" genossi_service_impl/src/repayment_letter.rs` returns 1
    - `grep -c "fn sanitize_for_filename" genossi_service_impl/src/repayment_letter.rs` returns 1
    - `grep -c "X-Skipped-Count" genossi_rest/src/repayment_letter.rs` returns 1
    - `grep -c "X-Document-Count" genossi_rest/src/repayment_letter.rs` returns >= 2 (bestehender generate + neuer download)
    - `grep -c '\.route("/{phase_id}/letters/download"' genossi_rest/src/repayment_letter.rs` returns 1
    - `grep -c '"application/zip"' genossi_service_impl/src/repayment_letter.rs` returns >= 1
    - **KEIN Audit-Logging im neuen Pfad:**
      - In der Funktion `download_bundle` MUSS gelten: `grep -v '^[[:space:]]*//' genossi_service_impl/src/repayment_letter.rs | awk '/async fn download_bundle/,/^    }$/' | grep -c 'audited_' == 0`
    - cargo build --workspace exit 0
    - cargo test -p genossi_service repayment_letter (Trait-Tests grün, incl. neue 3 Tests)
    - cargo test -p genossi_service_impl repayment_letter (Impl-Tests grün, incl. neue 4 Tests)
    - cargo test -p genossi_rest repayment_letter (REST-Tests grün, incl. neue DownloadQuery-Tests)
    - cargo fmt --check exit 0
    - cargo clippy --workspace --all-targets exit 0
  </acceptance_criteria>
  <done>
    Backend-Stack vollstaendig: Trait + Impl + REST-Handler kompilieren; 7+ neue Unit-Tests grün; lopdf-Workspace-Dep eingebaut; KEIN Audit-Logging im Lese-Pfad; cargo workspace-Build + fmt + clippy alle clean.
  </done>
</task>

<task type="auto">
  <name>Task 2: E2E-Tests + Frontend-Component + Page-Mount</name>
  <files>
    genossi_bin/tests/repayment_letter_e2e.rs,
    genossi-frontend/src/component/repayment_letter_download_button.rs,
    genossi-frontend/src/component/mod.rs,
    genossi-frontend/src/page/repayment_phase_details.rs,
    genossi-frontend/src/api.rs,
    genossi-frontend/src/i18n/mod.rs,
    genossi-frontend/src/i18n/de.rs,
    genossi-frontend/src/i18n/en.rs
  </files>
  <read_first>
    - genossi_bin/tests/repayment_letter_e2e.rs:30-340 (existierende Helpers + Happy-Path-Pattern fuer Phase 13)
    - genossi-frontend/src/page/repayment_phase_details.rs:255-340 (existierende Browser-Download-Logik via <a download>)
    - genossi-frontend/src/api.rs:1940-2000 (bestehende generate_letters API-Funktion — Pattern fuer neue download_letters_zip / download_letters_pdf)
    - genossi-frontend/src/component/mod.rs (Component-Registry)
    - genossi-frontend/src/i18n/mod.rs (Key-Enum fuer i18n-Erweiterung)
    - genossi-frontend/src/i18n/de.rs + en.rs (Translations-Pattern)
  </read_first>
  <action>
    **Stelle 1: E2E-Tests (genossi_bin/tests/repayment_letter_e2e.rs):**

    Am DateiEnde 5 neue Tests ergaenzen, die das existierende Setup-Helpers-Repertoire wiederverwenden (`setup_with_templates`, `create_admin_session`, `seed_open_phase_with_entries`, etc. — Implementer prueft die existierenden Helper-Namen).

    ```rust
    // ─── quick-260602-sgp: Bulk-Download GET /letters/download ──────────

    /// Helper: triggere POST /letters/generate, damit MemberDocuments im Storage liegen.
    /// Nachgeschalteter download_letters-Call erwartet diese persistierten Files.
    async fn seed_persisted_letters(
        client: &reqwest::Client,
        base: &str,
        session: &str,
        phase_id: Uuid,
        entry_ids: &[Uuid],
    ) {
        // Re-use bestehender Helper-Aufruf POST .../letters/generate.
        // (Implementer ermittelt den korrekten existierenden Helper-Namen.)
    }

    #[tokio::test]
    async fn test_download_letters_zip_happy_path() {
        // Setup: Phase mit 2 Members + 2 Entries; POST /letters/generate liefert 2 persistierte Letters.
        // Action: GET .../letters/download?format=zip
        // Assert:
        //   - status 200
        //   - Content-Type "application/zip"
        //   - Content-Disposition enthaelt "auszahlungs_anschreiben_GJ_{fy}.zip"
        //   - X-Document-Count == "2"
        //   - X-Skipped-Count == "0"
        //   - Body-Bytes beginnen mit ZIP-Magic 0x504B0304 (PK\x03\x04)
        //   - ZIP-Inhalt: 2 Files, sortiert nach member_number
    }

    #[tokio::test]
    async fn test_download_letters_pdf_happy_path() {
        // Setup wie oben.
        // Action: GET .../letters/download?format=pdf
        // Assert:
        //   - status 200
        //   - Content-Type "application/pdf"
        //   - X-Document-Count == "2"
        //   - Body-Bytes beginnen mit %PDF
        //   - Body laesst sich mit lopdf::Document::load_mem parsen (smoke-Test)
    }

    #[tokio::test]
    async fn test_download_letters_zero_persisted_returns_404() {
        // Setup: Phase OHNE persistierte Letters (POST /letters/generate NICHT aufgerufen).
        // Action: GET .../letters/download?format=zip
        // Assert:
        //   - status 404
    }

    #[tokio::test]
    async fn test_download_letters_helper_auth_returns_403() {
        // Setup: Helper-Session.
        // Action: GET .../letters/download?format=zip mit Helper-Token.
        // Assert:
        //   - status 403
    }

    #[tokio::test]
    async fn test_download_letters_preparation_phase_returns_409() {
        // Setup: Phase im Preparation-Status (vor open).
        // Action: GET .../letters/download?format=zip
        // Assert:
        //   - status 409
        //   - Body enthaelt "phase_not_active"
    }

    #[tokio::test]
    async fn test_download_letters_invalid_format_returns_400() {
        // Action: GET .../letters/download?format=docx
        // Assert: status 400, Body enthaelt "use 'zip' or 'pdf'".
    }
    ```

    **Stelle 2: Frontend API-Funktionen (genossi-frontend/src/api.rs):**

    Nach den bestehenden Phase-13-Funktionen einfuegen:

    ```rust
    /// GET /api/repayment-phase/{phase_id}/letters/download?format=zip|pdf
    /// quick-260602-sgp: Bulk-Download bereits persistierter Letters.
    ///
    /// Returns (bytes, document_count, skipped_count, content_type, filename).
    pub async fn download_repayment_letters(
        backend: &str,
        phase_id: Uuid,
        format: &str, // "zip" oder "pdf"
    ) -> Result<(Vec<u8>, usize, usize, String, String), AppError> {
        let url = format!(
            "{}/api/repayment-phase/{}/letters/download?format={}",
            backend, phase_id, format
        );
        let resp = http_get_with_session(&url).await?;
        // Status-Handling wie bei den anderen API-Funktionen.
        // Extract X-Document-Count, X-Skipped-Count Header.
        // Extract Filename aus Content-Disposition.
        // Body bytes.
        // ...
    }
    ```

    Implementer ermittelt den existierenden HTTP-Pattern (Header-Extraction, Cookie-Session, Error-Mapping) aus `download_repayment_export` oder `generate_repayment_letters`.

    **Stelle 3: Component (genossi-frontend/src/component/repayment_letter_download_button.rs — NEUE DATEI):**

    Component-First: KEIN inline-RSX in der Page. Eigene Datei:

    ```rust
    //! Quick-260602-sgp: Bulk-Download bereits persistierter RepaymentLetter-PDFs.
    //!
    //! Wiederverwendbar fuer RepaymentPhase-Detail-Page. Zwei Format-Optionen
    //! (ZIP / Bundle-PDF) als zwei Buttons im Component, damit die Page
    //! KEIN inline-RSX-Markup fuer Download-Logik braucht (Component-First-Prinzip
    //! aus CLAUDE.md + Memory feedback_component_first.md).

    use dioxus::prelude::*;
    use uuid::Uuid;

    use crate::api::download_repayment_letters;
    use crate::i18n::{Key, use_i18n};

    #[derive(Props, PartialEq, Clone)]
    pub struct RepaymentLetterDownloadButtonProps {
        pub phase_id: Uuid,
        pub backend: Signal<String>,
        pub on_toast: EventHandler<String>,
    }

    #[component]
    pub fn RepaymentLetterDownloadButton(props: RepaymentLetterDownloadButtonProps) -> Element {
        let i18n = use_i18n();
        let phase_id = props.phase_id;
        let backend = props.backend;
        let on_toast = props.on_toast;

        let trigger_download = move |format: &'static str| {
            let backend_str = backend.read().clone();
            let on_toast = on_toast.clone();
            spawn(async move {
                match download_repayment_letters(&backend_str, phase_id, format).await {
                    Ok((bytes, doc_count, skipped_count, content_type, filename)) => {
                        // Browser-Save via <a download> — gleiches Pattern wie
                        // repayment_phase_details.rs:300-310 (existierender Download
                        // im Phase-13 generate-Flow).
                        // Implementer extrahiert die wiederverwendbaren Schritte
                        // (Blob, URL.create_object_url, a-Element, click, revoke).
                        // ...
                        let msg = if skipped_count > 0 {
                            format!("{} files downloaded ({} missing in storage)", doc_count, skipped_count)
                        } else {
                            format!("{} files downloaded", doc_count)
                        };
                        on_toast.call(msg);
                    }
                    Err(e) => {
                        on_toast.call(format!("Download failed: {}", e));
                    }
                }
            });
        };

        rsx! {
            div { class: "flex gap-2",
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| trigger_download("zip"),
                    {i18n.t(Key::RepaymentLetterDownloadZipButton)}
                }
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| trigger_download("pdf"),
                    {i18n.t(Key::RepaymentLetterDownloadPdfButton)}
                }
            }
        }
    }
    ```

    **WICHTIG (Memory feedback_dioxus_button_type.md)**: `r#type: "button"` ist PFLICHT, sonst Page-Reload-Bug.

    **Stelle 4: Component-Registry (genossi-frontend/src/component/mod.rs):**

    ```rust
    pub mod repayment_letter_download_button;
    pub use repayment_letter_download_button::RepaymentLetterDownloadButton;
    ```

    **Stelle 5: i18n-Keys (mod.rs, de.rs, en.rs):**

    Neue Keys:
    ```rust
    // i18n/mod.rs
    RepaymentLetterDownloadZipButton,
    RepaymentLetterDownloadPdfButton,
    ```

    Deutsche Texte (`i18n/de.rs`):
    ```rust
    Key::RepaymentLetterDownloadZipButton => "Alle Briefe als ZIP herunterladen".into(),
    Key::RepaymentLetterDownloadPdfButton => "Alle Briefe als Bundle-PDF herunterladen".into(),
    ```

    Englische Texte (`i18n/en.rs`):
    ```rust
    Key::RepaymentLetterDownloadZipButton => "Download all letters as ZIP".into(),
    Key::RepaymentLetterDownloadPdfButton => "Download all letters as Bundle-PDF".into(),
    ```

    **Stelle 6: Page-Mount (genossi-frontend/src/page/repayment_phase_details.rs):**

    Neue Component an einer geeigneten Stelle (innerhalb BasicsTab oder im Letters-Bereich der Page — Implementer waehlt die natuerlichste Stelle und dokumentiert sie im SUMMARY) einbauen. KEINE inline-RSX-Duplikation — NUR `RepaymentLetterDownloadButton`-Tag mit Props.

    Beispiel:
    ```rust
    RepaymentLetterDownloadButton {
        phase_id: phase.id,
        backend: backend.clone(),
        on_toast: move |msg: String| toast.set(Some(msg)),
    }
    ```

    Verwende den existierenden `toast`-Signal-Wert der Page (Implementer ermittelt den exakten Namen).
  </action>
  <verify>
    <automated>cargo test --test repayment_letter_e2e -- test_download_letters && cd genossi-frontend && cargo check --target wasm32-unknown-unknown && cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - **E2E-Tests**:
      - `grep -c "fn test_download_letters_" genossi_bin/tests/repayment_letter_e2e.rs` returns >= 6 (zip-happy, pdf-happy, zero-404, helper-403, preparation-409, invalid-format-400)
      - `cargo test --test repayment_letter_e2e test_download_letters` alle grün
    - **Frontend Component-File existiert**:
      - `test -f genossi-frontend/src/component/repayment_letter_download_button.rs`
      - `grep -c "pub fn RepaymentLetterDownloadButton" genossi-frontend/src/component/repayment_letter_download_button.rs` returns 1
      - `grep -c 'r#type: "button"' genossi-frontend/src/component/repayment_letter_download_button.rs` returns >= 2 (beide Buttons; memory feedback_dioxus_button_type.md)
    - **Component-Registry**:
      - `grep -c "pub mod repayment_letter_download_button" genossi-frontend/src/component/mod.rs` returns 1
      - `grep -c "pub use repayment_letter_download_button::RepaymentLetterDownloadButton" genossi-frontend/src/component/mod.rs` returns 1
    - **Page nutzt Component (KEIN inline-RSX-Duplikat)**:
      - `grep -c "RepaymentLetterDownloadButton" genossi-frontend/src/page/repayment_phase_details.rs` returns >= 1
      - **Anti-Duplication-Gate**: in `genossi-frontend/src/page/repayment_phase_details.rs` DARF download_letters NICHT direkt inline-aufgerufen werden (nur via Component); grep `'download_repayment_letters'` in der page-Datei returns 0 (nur in Component erlaubt)
    - **API-Funktion**:
      - `grep -c "pub async fn download_repayment_letters" genossi-frontend/src/api.rs` returns 1
    - **i18n-Keys**:
      - `grep -c "RepaymentLetterDownloadZipButton" genossi-frontend/src/i18n/mod.rs` returns 1
      - `grep -c "RepaymentLetterDownloadZipButton" genossi-frontend/src/i18n/de.rs` returns 1
      - `grep -c "RepaymentLetterDownloadZipButton" genossi-frontend/src/i18n/en.rs` returns 1
      - `grep -c "RepaymentLetterDownloadPdfButton" genossi-frontend/src/i18n/mod.rs` returns 1
      - `grep -c "RepaymentLetterDownloadPdfButton" genossi-frontend/src/i18n/de.rs` returns 1
      - `grep -c "RepaymentLetterDownloadPdfButton" genossi-frontend/src/i18n/en.rs` returns 1
    - cargo build --workspace exit 0
    - cargo test --workspace exit 0 (alle bestehenden Tests + neue grün)
    - cargo fmt --check exit 0
    - cargo clippy --workspace --all-targets exit 0
  </acceptance_criteria>
  <done>
    6+ neue E2E-Tests grün; Component-First eingehalten (eigener Component-File, Page nur Mount); i18n-Texte fuer DE + EN; Workspace-Build + alle Tests + fmt + clippy clean.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Browser -> REST | Vorstand-Session (Cookie-basiert); Helper-Auth muss 403 produzieren |
| REST -> Service | `Authentication<Context>`-Token; PermissionService.check_permission(ADMIN) |
| Service -> DAO | Read-only Transaction; KEIN audited_*! |
| Service -> DocumentStorage | Filesystem-Read auf `relative_path` aus MemberDocumentEntity (vertraute, intern erzeugte Pfade) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-sgp-01 | S | REST download_letters | mitigate | `extract_auth_context(Some(context))?` erzwingt Session-Validierung; Helper-Token-Cookies werden vom Permission-Service als nicht-admin erkannt -> 403 via map_letter_error |
| T-sgp-02 | T | Body-Bytes auf Transport-Layer | accept | TLS terminiert am Reverse-Proxy (Genossi-Standard); kein Application-Level-Signing fuer Downloads |
| T-sgp-03 | R | Read-Endpoint, kein audited_log | accept | Bewusst kein Audit (CLAUDE.md: Audit-Pflicht nur fuer Write-Ops auf Member/MemberAction/MemberDocument/Application). Der reine Lese-Download eines existierenden, bereits auditierten MemberDocuments ist nicht repudiation-relevant |
| T-sgp-04 | I | DSGVO-Daten in Bulk-PDF/ZIP | mitigate | Permission-Gate ADMIN; X-Document-Count + X-Skipped-Count erlauben Frontend, dem Nutzer Transparency zu schaffen; KEIN Member-Daten-Leak ueber Filename hinaus (member_number/lastname/firstname == Standard-Verbandskonformitaet) |
| T-sgp-05 | D | Riesen-Phase mit 1000+ Letters | mitigate | Genossi-Realbetrieb hat <100 Members pro Phase; lopdf/zip in-memory ist akzeptabel. Falls jemals >1000 Letters: Streaming-Variante als Tech-Debt-Item |
| T-sgp-06 | E | Path-Traversal via doc.relative_path | mitigate | `relative_path` wird im Service nicht aus User-Input gebaut, sondern aus MemberDocumentEntity (intern erzeugt via `format!("{}.pdf", doc_id)` in `RepaymentLetterServiceImpl::generate`). `DocumentStorage::load` hat zudem eigene Path-Cleaning-Garantien. Defense-in-Depth: doc_id ist UUID -> kein "../"-Vektor moeglich |
| T-sgp-07 | I | ZIP-Slip via Filename | mitigate | Filenames werden aus `member.member_number` (i64) + `sanitize_for_filename(last_name/first_name)` gebaut, NICHT aus User-Input. sanitize_for_filename ersetzt Sonderzeichen -> '_', sodass keine "/" oder ".." in ZIP-Eintraegen landen koennen |
</threat_model>

<verification>
1. `cargo build --workspace` exit 0
2. `cargo test --workspace` exit 0 (alle bestehenden + neue Tests grün)
3. `cargo fmt --check` exit 0
4. `cargo clippy --workspace --all-targets` exit 0
5. `cd genossi-frontend && cargo check --target wasm32-unknown-unknown` exit 0
6. Audit-Disziplin-Grep:
   `grep -v '^[[:space:]]*//' genossi_service_impl/src/repayment_letter.rs | awk '/async fn download_bundle/,/^    }$/' | grep -c 'audited_'` returns 0
7. Single-Source-of-Truth fuer Letter-Identifikation:
   `grep -c "find_existing_letter_for_phase" genossi_service_impl/src/repayment_letter.rs` returns >= 3 (Definition + generate-Call + download_bundle-Call)
8. Component-First-Gate (Anti-Inline-RSX):
   `grep -c "download_repayment_letters" genossi-frontend/src/page/repayment_phase_details.rs` returns 0
   `grep -c "download_repayment_letters" genossi-frontend/src/component/repayment_letter_download_button.rs` returns >= 1
</verification>

<success_criteria>
1. **Funktional**: Vorstand sieht zwei Buttons auf RepaymentPhase-Detail-Page; Klick auf "Als ZIP" liefert dl mit allen Briefen; Klick auf "Als Bundle-PDF" liefert eine zusammengefuegte PDF.
2. **NICHT-Neu-Render bewiesen**: Service-Code-Pfad enthaelt KEINE `pdf_generator.render_*`-Aufrufe in `download_bundle`; nur `document_storage.load(...)` + ZIP-/lopdf-Merge.
3. **Edge-Cases**: 0 Letters -> 404; teilweise fehlende Files -> X-Skipped-Count Header; invalid format -> 400; Helper-Auth -> 403; Preparation-Status -> 409.
4. **Audit-Konsistenz**: keine neuen audit_log-Eintraege durch den Download-Endpoint; bestehende Hash-Chain bleibt valid (`GET /api/audit/verify` muss vor + nach Download den gleichen `valid: true` liefern; E2E-Tests verifizieren das implizit via cargo test --workspace).
5. **Component-First eingehalten**: Download-Button-Logik vollstaendig in eigener Component-Datei; Page nutzt NUR die Component (Anti-Duplikation-Gate grep == 0).
</success_criteria>

<output>
After completion, create `.planning/quick/260602-sgp-bulk-download-aller-repaymentletter-doku/260602-sgp-SUMMARY.md` documenting:
- Welche lopdf-API-Variante wurde fuer Merge verwendet (Document::merge_documents falls vorhanden, sonst manuelles Pages-Tree-Merge)
- Welche existierenden Test-Helper im Phase-13-E2E-Setup wiederverwendet wurden
- Wo auf RepaymentPhase-Detail-Page der Download-Button gemounted wurde (welcher Tab / welcher Bereich)
- Skipped-Count-Behavior: ob es im Real-Lauf je >0 wurde (sollte normalerweise 0 sein, Storage-Drift-Indikator)
- Cargo-fmt + clippy-Status
</output>
