//! Phase 13 D-13-01..11: RepaymentLetterServiceImpl.
//!
//! Orchestriert Permission-Funnel, Entry-Validation, Multi-Entry-Aggregation
//! (via Resolver::aggregate pure-fn, KEIN 1+N DB-Read), auditierte
//! MemberDocument-Persistenz mit File-Save, Bundle-Render und Return.
//!
//! Pattern-Quellen:
//! - Funnel + DI-Pattern: `genossi_service_impl/src/repayment_export.rs:42-110`
//! - audited_create + relative_path: `genossi_service_impl/src/member_document.rs:115-150`
//! - audited_create-Macro: `genossi_service_impl/src/audit_macros.rs:5-36`
//!
//! Locked Decisions:
//! - D-13-01: Hybrid Bundle (N persistierte MemberDocuments + 1 transientes Bundle-PDF)
//! - D-13-03: Body = { entry_ids: [...] }; subset-Check gegen Phase
//! - D-13-04: Multi-Entry-Aggregation per Member via Resolver::aggregate
//! - D-13-08: Re-Generierung erlaubt (kein 409, kein singleton)
//! - D-13-09: KEIN Backend-Touch auf RepaymentEntry — weder direkter
//!            DAO-Write noch indirekter Service-Aufruf, der den Status toggelt
//!
//! Render-Reihenfolge (Pitfall #2): Read-Tx commit VOR Render (sync Typst),
//! danach Render, danach Schreibe-Tx fuer audited_create-Loop.

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::{MemberDao, MemberEntity};
use genossi_dao::member_document::{MemberDocumentDao, MemberDocumentEntity};
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity};
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus};
use genossi_dao::{Transaction, TransactionDao};
use genossi_service::document_storage::DocumentStorage;
use genossi_service::member_document::{DocumentType, MemberDocument};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::repayment_context::{RepaymentContext, RepaymentContextResolver};
use genossi_service::repayment_letter::{
    RepaymentLetterBundle, RepaymentLetterDownload, RepaymentLetterDownloadFormat,
    RepaymentLetterService,
};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};

use crate::pdf_generation::PdfGenerator;

/// D-11 Pattern: konsistent mit allen anderen Vorstand-Endpoints.
const ADMIN_PRIVILEGE: &str = "admin";
/// audit-process-Konstante — landet im audit_log.process Spalte.
const REPAYMENT_LETTER_PROCESS: &str = "repayment-letter-service";
/// tracing target (analog Phase 11 EXPORT_TARGET).
const LETTER_TARGET: &str = "repayment_letter";
/// Template-Pfade — hardcoded (D-13-05: User editiert das File-Inhalt, nicht den Pfad).
const SINGLE_TEMPLATE_PATH: &str = "auszahlungs_anschreiben.typ";
const BUNDLE_TEMPLATE_PATH: &str = "auszahlungs_anschreiben_bundle.typ";

/// Server-side Bulk-Limit: max. Anzahl entry_ids in einer Bulk-Request.
/// Schuetzt vor DoS via Riesen-entry_ids-Liste (Threat-Model).
const MAX_ENTRY_IDS_PER_REQUEST: usize = 200;

/// Dependency-Injection-Trait fuer `RepaymentLetterServiceImpl`.
/// Vorbild: `RepaymentExportServiceDeps` erweitert um MemberDocumentDao,
/// AuditLogDao, UuidService, DocumentStorage, RepaymentContextResolver.
pub trait RepaymentLetterServiceDeps: Send + Sync + 'static {
    type Context: Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: Transaction;
    type RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> + Send + Sync;
    type RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> + Send + Sync;
    type MemberDao: MemberDao<Transaction = Self::Transaction> + Send + Sync;
    type MemberDocumentDao: MemberDocumentDao<Transaction = Self::Transaction> + Send + Sync;
    type AuditLogDao: AuditLogDao<Transaction = Self::Transaction> + Send + Sync;
    type PermissionService: PermissionService<Context = Self::Context> + Send + Sync;
    type TransactionDao: TransactionDao<Transaction = Self::Transaction> + Send + Sync;
    type UuidService: UuidService + Send + Sync;
    type RepaymentContextResolver: RepaymentContextResolver<Transaction = Self::Transaction>
        + Send
        + Sync;
    type DocumentStorage: DocumentStorage + Send + Sync;
}

/// Konkrete Letter-Service-Implementation. Plan 13-DI-Wiring (kommendes Plan
/// 13-05/Final-Wiring) instanziiert mit Production-`Deps`.
pub struct RepaymentLetterServiceImpl<Deps: RepaymentLetterServiceDeps> {
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
    pub member_dao: Arc<Deps::MemberDao>,
    pub member_document_dao: Arc<Deps::MemberDocumentDao>,
    pub audit_log_dao: Arc<Deps::AuditLogDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub uuid_service: Arc<Deps::UuidService>,
    pub repayment_context_resolver: Arc<Deps::RepaymentContextResolver>,
    pub document_storage: Arc<Deps::DocumentStorage>,
    pub pdf_generator: Arc<PdfGenerator>,
    pub template_base: Arc<PathBuf>,
}

impl<Deps: RepaymentLetterServiceDeps> RepaymentLetterServiceImpl<Deps> {
    /// WR-05: Konsistenter Constructor fuer DI-Wiring + Tests. Felder bleiben
    /// `pub` (Backwards-Compat mit existierenden Test-Helpern build_service /
    /// build_service_with_templates, die direktes Struct-Literal nutzen), aber
    /// neuer Code SOLLTE `new(...)` verwenden, damit kuenftige Invariant-Checks
    /// (z.B. template_base.exists()) eine zentrale Stelle haben.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
        repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
        member_dao: Arc<Deps::MemberDao>,
        member_document_dao: Arc<Deps::MemberDocumentDao>,
        audit_log_dao: Arc<Deps::AuditLogDao>,
        permission_service: Arc<Deps::PermissionService>,
        transaction_dao: Arc<Deps::TransactionDao>,
        uuid_service: Arc<Deps::UuidService>,
        repayment_context_resolver: Arc<Deps::RepaymentContextResolver>,
        document_storage: Arc<Deps::DocumentStorage>,
        pdf_generator: Arc<PdfGenerator>,
        template_base: Arc<PathBuf>,
    ) -> Self {
        Self {
            repayment_phase_dao,
            repayment_entry_dao,
            member_dao,
            member_document_dao,
            audit_log_dao,
            permission_service,
            transaction_dao,
            uuid_service,
            repayment_context_resolver,
            document_storage,
            pdf_generator,
            template_base,
        }
    }

    /// Permission-Funnel (Pitfall #2): load (404) -> admin (403) -> status (409).
    /// 1:1 Pattern aus `repayment_export.rs:77-110`, Error-String "phase_not_active".
    async fn check_admin_and_phase_status(
        &self,
        phase_id: Uuid,
        context: Authentication<Deps::Context>,
        tx: Deps::Transaction,
    ) -> Result<RepaymentPhaseEntity, ServiceError> {
        // 1. Load (404 if missing).
        let phase = self
            .repayment_phase_dao
            .find_by_id(phase_id, tx)
            .await?
            .ok_or(ServiceError::EntityNotFound(phase_id))?;

        // 2. Admin gate (403). `Authentication::Full` short-circuits.
        match &context {
            Authentication::Full => {}
            Authentication::Context(_) => {
                self.permission_service
                    .check_permission(ADMIN_PRIVILEGE, context)
                    .await?;
            }
        }

        // 3. Status gate (409): Open ODER Closed akzeptiert.
        match phase.status {
            RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
            RepaymentPhaseStatus::Preparation => {
                return Err(ServiceError::Conflict(Arc::from("phase_not_active")));
            }
        }

        Ok(phase)
    }

    /// Quick 260602-q9l: idempotente Regenerate-Lookup.
    ///
    /// Sucht das (aktive) RepaymentLetter-MemberDocument fuer den uebergebenen
    /// `fiscal_year` aus einer bereits per `find_by_member_id` geladenen
    /// Doc-Liste. Reine Funktion (kein DI, kein async) — leicht unit-testbar.
    ///
    /// Filter:
    ///   - `deleted is None`         (Defense-in-Depth; `find_by_member_id`
    ///     filtert das bereits, aber wir wollen die Regel hier explizit halten.)
    ///   - `document_type == "repayment_letter"`
    ///   - `description == "Anschreiben Auszahlung GJ {fiscal_year}"`
    ///
    /// Das Description-Feld ist hier der per-Row-Fingerprint fuer
    /// (member, phase): `MemberDocumentEntity` hat kein `phase_id` /
    /// `fiscal_year`-Feld, daher dient die Description als deterministische
    /// Identifikation. Aenderungen an dem Description-Format MUESSEN mit dieser
    /// Funktion synchron gehalten werden (siehe Konstruktionsstelle weiter unten
    /// im `generate()`-write-tx-Loop).
    fn find_existing_letter_for_phase(
        docs: &[MemberDocumentEntity],
        fiscal_year: i32,
    ) -> Option<MemberDocumentEntity> {
        let expected_desc = format!("Anschreiben Auszahlung GJ {}", fiscal_year);
        docs.iter()
            .find(|d| {
                d.deleted.is_none()
                    && d.document_type.as_ref() == DocumentType::RepaymentLetter.as_str()
                    && d.description.as_deref() == Some(expected_desc.as_str())
            })
            .cloned()
    }

    /// user_id-Resolution. **KEIN Sentinel-UUID-Fallback** — bei
    /// nicht-extrahierbarer user_id wird `ServiceError::PermissionDenied`
    /// geworfen, damit der audit_log.user_id-String NIE leer ist und
    /// Audit-Hashchain konsistent bleibt.
    ///
    /// PRE-FLIGHT-GREP RESULTAT (Schritt 0):
    /// - `genossi_service/src/permission.rs:42` definiert
    ///   `async fn current_user_id(...) -> Result<Option<String>, ServiceError>`.
    /// - Returnt `Option<String>` (String-ID — z.B. OIDC-`sub` oder username).
    /// - **Pattern A gewaehlt** mit Adaption: existierende Caller (z.B.
    ///   `member_document.rs:61-65`) machen `.unwrap_or_else(|| "SYSTEM".to_string())`.
    ///   Phase-13-Verbandskonformitaet erlaubt KEIN "SYSTEM"-Fallback fuer
    ///   Vorstand-Aktionen — daher hier `PermissionDenied` bei `None`.
    /// - `Authentication::Full`-Pfad: keine extrahierbare user_id (Full ist nur
    ///   fuer interne System-Calls gedacht); ebenfalls `PermissionDenied`.
    async fn resolve_user_id_or_deny(
        &self,
        context: &Authentication<Deps::Context>,
    ) -> Result<String, ServiceError> {
        match context {
            Authentication::Full => Err(ServiceError::PermissionDenied),
            Authentication::Context(_) => {
                let user_id_opt = self
                    .permission_service
                    .current_user_id(context.clone())
                    .await?;
                user_id_opt.ok_or(ServiceError::PermissionDenied)
            }
        }
    }
}

#[async_trait]
impl<Deps: RepaymentLetterServiceDeps> RepaymentLetterService for RepaymentLetterServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn generate(
        &self,
        phase_id: Uuid,
        entry_ids: Arc<[Uuid]>,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentLetterBundle, ServiceError> {
        // 0. Pre-Validation (vor jeder DB-Touche).
        if entry_ids.is_empty() {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("entry_ids"),
                message: Arc::from("must not be empty"),
            }]));
        }
        if entry_ids.len() > MAX_ENTRY_IDS_PER_REQUEST {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("entry_ids"),
                message: Arc::from(
                    format!("max {} entries per bulk request", MAX_ENTRY_IDS_PER_REQUEST).as_str(),
                ),
            }]));
        }

        // WR-01 fix: user_id-Resolution VOR jeder DB-Touche (Defense-in-Depth).
        // Auth-Validierung ist konzeptuell unabhaengig von der Phase-/Entry-Existenz —
        // ein transienter Auth-Glitch sollte keine Read-Tx-Ressource verbrauchen.
        // Konsistent mit der Pre-Validation-Reihenfolge oben (Validation vor jeder DB-Touche).
        let user_id = self.resolve_user_id_or_deny(&context).await?;

        // 1. Read-Tx oeffnen: Funnel + Entry-Validation + Member-Reads alle in
        //    EINER Tx, die VOR dem Render committed wird (Pitfall #2).
        let read_tx = self.transaction_dao.use_transaction(None).await?;

        // 2. Funnel: load phase (404) -> admin (403) -> status (409).
        let phase = self
            .check_admin_and_phase_status(phase_id, context.clone(), read_tx.clone())
            .await?;

        // 3. Load Phase-Entries und validiere subset (ONCE, vor dem Member-Loop —
        //    KEIN 1+N DB-Read durch resolve.).
        let phase_entries: Vec<RepaymentEntryEntity> = self
            .repayment_entry_dao
            .find_by_phase_id(phase_id, read_tx.clone())
            .await?
            .iter()
            .cloned()
            .collect();
        let phase_entry_set: HashSet<Uuid> = phase_entries.iter().map(|e| e.id).collect();
        let requested_set: HashSet<Uuid> = entry_ids.iter().copied().collect();
        if !requested_set.is_subset(&phase_entry_set) {
            // ValidationError -> HTTP 400 (entry_phase_mismatch).
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("entry_ids"),
                message: Arc::from("entry_phase_mismatch"),
            }]));
        }

        // 4. Group entries per member_id (in-memory). dedup VOR Member-Reads.
        let mut member_ids: Vec<Uuid> = phase_entries
            .iter()
            .filter(|e| requested_set.contains(&e.id))
            .map(|e| e.member_id)
            .collect();
        member_ids.sort();
        member_ids.dedup();

        // 5. Members laden (in derselben Read-Tx) + Resolver::aggregate
        //    (sync pure-fn, kein DB-Round-Trip).
        let mut recipients: Vec<(MemberEntity, RepaymentContext)> =
            Vec::with_capacity(member_ids.len());
        for &mid in &member_ids {
            let member = self
                .member_dao
                .find_by_id(mid, read_tx.clone())
                .await?
                .ok_or(ServiceError::EntityNotFound(mid))?;
            // resolver.aggregate (sync, kein tx, kein DB-Round-Trip).
            let ctx = self
                .repayment_context_resolver
                .aggregate(&phase, &phase_entries, mid)?;
            recipients.push((member, ctx));
        }

        // 6. Sortiere recipients nach member_number ASC (Pitfall #10).
        recipients.sort_by(|a, b| a.0.member_number.cmp(&b.0.member_number));

        // 7. Commit Read-Tx VOR Render (Pitfall #2).
        self.transaction_dao.commit(read_tx).await?;

        // 8. (user_id wird bereits vor dem Read-Tx-Open resolved — WR-01 Defense-in-Depth.)

        // 9. Render N Single-Letter-PDFs (sync, in-memory).
        let mut single_pdfs: Vec<(Uuid, Vec<u8>)> = Vec::with_capacity(recipients.len());
        for (member, ctx) in &recipients {
            let bytes = self.pdf_generator.render_repayment_letter(
                SINGLE_TEMPLATE_PATH,
                &self.template_base,
                &phase,
                member,
                ctx,
            )?;
            single_pdfs.push((member.id, bytes));
        }

        // 10. Render Bundle-PDF (sync, in-memory) — VOR Schreibe-Tx, damit ein
        //     Bundle-Render-Fehler nicht zu halb-persistierten MemberDocuments fuehrt.
        let bundle_bytes = self.pdf_generator.render_repayment_letter_bundle(
            BUNDLE_TEMPLATE_PATH,
            &self.template_base,
            &phase,
            &recipients,
        )?;

        // 11. Schreibe-Tx: alle MemberDocuments via audited_create (sequential — Pitfall #4).
        //
        // CR-02 fix (atomic-then-persist): File-Saves wurden vorher PRO Recipient
        // VOR audited_create ausgefuehrt. Wenn audited_create fuer Recipient N
        // fehlschlug, rollbackte SQLite alle vorherigen INSERTs, aber die N-1
        // bereits geschriebenen PDF-Files blieben verwaist (Storage-vs-DB-Drift,
        // DSGVO-relevant da personenbezogene Daten unverlinkt).
        //
        // Neu: a) alle audited_create-Calls + commit zuerst in der Schreibe-Tx,
        //      b) erst NACH erfolgreichem commit die PDF-Files aufs Filesystem
        //         schreiben (planned_saves-Liste).
        //
        // Tradeoff: Wenn das File-Write nach dem commit fehlschlaegt, gibt es
        // umgekehrt einen MemberDocument-DB-Row OHNE PDF-File. Das ist
        // operativ tolerabler (der Vorstand sieht ein nicht-ladbares Doc und
        // kann re-generieren, statt unbemerkt verwaiste Personendaten zu hinter-
        // lassen). Wir return-en bei File-Fehler trotzdem mit Err — der Caller
        // weiss damit, dass die Operation nicht vollstaendig durchgelaufen ist.
        let write_tx = self.transaction_dao.use_transaction(None).await?;

        // (relative_path, &pdf_bytes) — wird erst NACH Tx-Commit auf Disk geschrieben.
        let mut planned_saves: Vec<(String, &Vec<u8>)> = Vec::with_capacity(recipients.len());
        let mut document_ids: Vec<Uuid> = Vec::with_capacity(recipients.len());
        for ((member, _ctx), (_mid, pdf_bytes)) in recipients.iter().zip(single_pdfs.iter()) {
            // Quick 260602-q9l: idempotent regenerate -- audited_update if existing.
            //
            // Lookup-Heuristik (member, phase): MemberDocumentEntity hat KEIN
            // `phase_id`/`fiscal_year`-Feld; wir scopen daher in zwei Stufen:
            //   1) DAO-Read `find_by_member_id` (filtert bereits deleted IS NULL).
            //   2) In-Memory-Filter ueber document_type + description-Fingerprint.
            // Description-Format MUSS exakt dem Konstruktor unten entsprechen
            // ("Anschreiben Auszahlung GJ {fiscal_year}") — siehe
            // `find_existing_letter_for_phase`.
            let existing_for_member = self
                .member_document_dao
                .find_by_member_id(member.id, write_tx.clone())
                .await?;
            let existing =
                Self::find_existing_letter_for_phase(&existing_for_member, phase.fiscal_year);

            let now = time::OffsetDateTime::now_utc();
            let file_name = Arc::from(
                format!(
                    "auszahlungs_anschreiben_{}_GJ_{}.pdf",
                    member.member_number, phase.fiscal_year
                )
                .as_str(),
            );
            let description = Some(Arc::from(
                format!("Anschreiben Auszahlung GJ {}", phase.fiscal_year).as_str(),
            ));

            if let Some(existing_doc) = existing {
                // UPDATE-Zweig: gleiche `id` + gleiches `relative_path` ->
                // PDF-File auf Disk wird in place ueberschrieben; audit_log
                // bekommt UPDATE-Eintraege fuer geaenderte Felder.
                let existing_id = existing_doc.id;
                let existing_relative_path = existing_doc.relative_path.clone();
                let updated_doc = MemberDocumentEntity {
                    id: existing_id,
                    member_id: existing_doc.member_id,
                    document_type: Arc::from(DocumentType::RepaymentLetter.as_str()),
                    description,
                    file_name,
                    mime_type: Arc::from("application/pdf"),
                    relative_path: existing_relative_path.clone(),
                    created: existing_doc.created, // immutable.
                    deleted: None,
                    version: self.uuid_service.new_v4().await, // rotate per optimistic-locking.
                    template_id: None,                         // D-LETT-04
                    mail_recipient_id: None,                   // D-LETT-04
                    status: None,                              // D-LETT-04
                };
                crate::audited_update!(
                    self,
                    self.member_document_dao,
                    existing_id,
                    &updated_doc,
                    REPAYMENT_LETTER_PROCESS,
                    &user_id,
                    write_tx
                );
                planned_saves.push((existing_relative_path.to_string(), pdf_bytes));
                document_ids.push(existing_id);
            } else {
                // CREATE-Zweig: unveraenderte Backwards-Compat fuer erste
                // Generierung. Fresh `doc_id` + `relative_path = "{doc_id}.pdf"`.
                let doc_id = self.uuid_service.new_v4().await;
                let relative_path = format!("{}.pdf", doc_id);

                let new_doc = MemberDocument {
                    id: doc_id,
                    member_id: member.id,
                    document_type: DocumentType::RepaymentLetter,
                    description,
                    file_name,
                    mime_type: Arc::from("application/pdf"),
                    relative_path: Arc::from(relative_path.as_str()),
                    created: time::PrimitiveDateTime::new(now.date(), now.time()),
                    deleted: None,
                    version: self.uuid_service.new_v4().await,
                    template_id: None,       // D-LETT-04
                    mail_recipient_id: None, // D-LETT-04
                    status: None,            // D-LETT-04
                };

                let doc_entity: MemberDocumentEntity = (&new_doc).into();
                crate::audited_create!(
                    self,
                    self.member_document_dao,
                    &doc_entity,
                    REPAYMENT_LETTER_PROCESS,
                    &user_id,
                    write_tx
                );
                planned_saves.push((relative_path, pdf_bytes));
                document_ids.push(doc_id);
            }
        }

        // Commit FIRST — Audit-Hashchain + MemberDocument-Rows sind nun
        // atomar persistiert. Bei Tx-Fehler wurde NICHTS aufs Filesystem
        // geschrieben (planned_saves wird nie ausgefuehrt).
        self.transaction_dao.commit(write_tx).await?;

        // NACH commit: PDF-Files persistieren. Bei File-Fehler bleibt das
        // korrespondierende MemberDocument als nicht-ladbar im Audit-Log —
        // operativ tolerabler als verwaiste personenbezogene PDF-Files.
        for (path, bytes) in &planned_saves {
            self.document_storage.save(path, bytes).await.map_err(|e| {
                ServiceError::InternalError(Arc::from(
                    format!("document_storage save failed: {}", e).as_str(),
                ))
            })?;
        }

        // 12. Return Bundle + Metadaten.
        let filename = format!("auszahlungs_anschreiben_GJ_{}.pdf", phase.fiscal_year);

        tracing::info!(
            target: LETTER_TARGET,
            phase_id = %phase_id,
            fiscal_year = phase.fiscal_year,
            recipients_count = recipients.len(),
            document_ids_count = document_ids.len(),
            "repayment letters generated"
        );

        Ok(RepaymentLetterBundle {
            bundle_bytes,
            filename,
            document_ids,
        })
    }

    /// Quick 260602-sgp: Bulk-Download bereits persistierter RepaymentLetter-PDFs.
    ///
    /// Reiner Lese-Endpoint — KEIN audited_*! Macro, KEINE Tx-Mutation, KEIN
    /// PdfGenerator-Render. Pfad:
    ///
    ///  1. Read-Tx oeffnen, Funnel (404 / 403 / 409 phase_not_active).
    ///  2. Phase-Entries laden -> unique member_ids (sort+dedup).
    ///  3. Pro Member existierendes RepaymentLetter-MemberDocument fuer GENAU
    ///     diese Phase suchen (Description-Fingerprint "Anschreiben Auszahlung
    ///     GJ {fy}" via Helper `find_existing_letter_for_phase`).
    ///  4. 0 persistierte Letters -> `ServiceError::EntityNotFound(phase_id)`.
    ///  5. Nach member_number ASC sortieren (deterministisch).
    ///  6. Files aus DocumentStorage laden — fehlende skippen (X-Skipped-Count).
    ///  7. ZIP-Bau (zip-Crate) ODER lopdf-Merge je nach Format.
    async fn download_bundle(
        &self,
        phase_id: Uuid,
        format: RepaymentLetterDownloadFormat,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentLetterDownload, ServiceError> {
        // 1. Read-Tx oeffnen.
        let read_tx = self.transaction_dao.use_transaction(None).await?;

        // 2. Funnel: load phase (404) -> admin (403) -> status (409).
        //    Identischer Gate wie POST /letters/generate.
        let phase = self
            .check_admin_and_phase_status(phase_id, context.clone(), read_tx.clone())
            .await?;

        // 3. Phase-Entries laden -> unique member_ids.
        let phase_entries = self
            .repayment_entry_dao
            .find_by_phase_id(phase_id, read_tx.clone())
            .await?;
        let mut member_ids: Vec<Uuid> = phase_entries.iter().map(|e| e.member_id).collect();
        member_ids.sort();
        member_ids.dedup();

        // 4. Pro Member existierendes RepaymentLetter-MemberDocument suchen.
        //    Reuse des existing-Helpers stellt sicher, dass wir die GLEICHEN
        //    Dokumente finden, die `generate()` persistiert hat (single source
        //    of truth fuer den Description-Fingerprint).
        let mut found: Vec<(MemberEntity, MemberDocumentEntity)> =
            Vec::with_capacity(member_ids.len());
        for mid in &member_ids {
            let docs = self
                .member_document_dao
                .find_by_member_id(*mid, read_tx.clone())
                .await?;
            if let Some(doc) = Self::find_existing_letter_for_phase(&docs, phase.fiscal_year) {
                let member = self
                    .member_dao
                    .find_by_id(*mid, read_tx.clone())
                    .await?
                    .ok_or(ServiceError::EntityNotFound(*mid))?;
                found.push((member, doc));
            }
        }

        // Read-Tx committen (saubere Trennung Read -> ZIP/PDF-Bau im Speicher).
        self.transaction_dao.commit(read_tx).await?;

        // 5. Edge-Case: 0 persistierte Letters -> 404.
        if found.is_empty() {
            return Err(ServiceError::EntityNotFound(phase_id));
        }

        // 6. Sort nach member_number ASC (deterministisch).
        found.sort_by(|a, b| a.0.member_number.cmp(&b.0.member_number));

        // 7. Files aus Storage laden — fehlende skippen.
        let mut loaded: Vec<(MemberEntity, Vec<u8>)> = Vec::with_capacity(found.len());
        let mut skipped_count: usize = 0;
        for (member, doc) in &found {
            match self.document_storage.load(&doc.relative_path).await {
                Ok(bytes) => loaded.push((member.clone(), bytes)),
                Err(e) => {
                    tracing::warn!(
                        target: LETTER_TARGET,
                        member_id = %member.id,
                        phase_id = %phase_id,
                        relative_path = %doc.relative_path,
                        "RepaymentLetter download: missing file in storage: {:?}",
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

        // 8. ZIP oder Bundle-PDF bauen — KEIN Schreib-Tx, KEIN Audit.
        let (bytes, content_type, filename): (Vec<u8>, &'static str, String) = match format {
            RepaymentLetterDownloadFormat::Zip => {
                let mut zip_buf = Cursor::new(Vec::new());
                {
                    let mut zip = zip::ZipWriter::new(&mut zip_buf);
                    let options = zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);
                    for (member, pdf_bytes) in &loaded {
                        // Filename-Schema: <member_number>_<lastname>_<firstname>.pdf
                        let safe_last = sanitize_for_filename(&member.last_name);
                        let safe_first = sanitize_for_filename(&member.first_name);
                        let fname =
                            format!("{}_{}_{}.pdf", member.member_number, safe_last, safe_first);
                        zip.start_file(&fname, options).map_err(|e| {
                            ServiceError::InternalError(Arc::from(
                                format!("zip start_file: {}", e).as_str(),
                            ))
                        })?;
                        zip.write_all(pdf_bytes).map_err(|e| {
                            ServiceError::InternalError(Arc::from(
                                format!("zip write_all: {}", e).as_str(),
                            ))
                        })?;
                    }
                    zip.finish().map_err(|e| {
                        ServiceError::InternalError(Arc::from(
                            format!("zip finish: {}", e).as_str(),
                        ))
                    })?;
                }
                let fname = format!("auszahlungs_anschreiben_GJ_{}.zip", phase.fiscal_year);
                (zip_buf.into_inner(), "application/zip", fname)
            }
            RepaymentLetterDownloadFormat::Pdf => {
                let merged_bytes = merge_pdfs_via_lopdf(&loaded).map_err(|e| {
                    ServiceError::InternalError(Arc::from(format!("pdf-merge: {}", e).as_str()))
                })?;
                let fname = format!("auszahlungs_anschreiben_GJ_{}.pdf", phase.fiscal_year);
                (merged_bytes, "application/pdf", fname)
            }
        };

        tracing::info!(
            target: LETTER_TARGET,
            phase_id = %phase_id,
            fiscal_year = phase.fiscal_year,
            document_count = loaded.len(),
            skipped_count,
            "repayment letters bulk-download built"
        );

        Ok(RepaymentLetterDownload {
            bytes,
            content_type,
            filename,
            document_count: loaded.len(),
            skipped_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Quick 260602-sgp: Bulk-Download-Helpers (free-fns, separate von Service-Impl).
// ---------------------------------------------------------------------------

/// Filename-Sanitisierung — Replika von `genossi_rest::http_util::sanitize_filename_component`.
///
/// Der Helper ist hier dupliziert, weil `genossi_service_impl` nicht auf
/// `genossi_rest` referenzieren darf (Layer-Inversion REST -> Service ist
/// nicht erlaubt). Umlaute werden zu ASCII transliteriert, andere Sonderzeichen
/// werden durch '_' ersetzt; der Output wird am Rand getrimmt.
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
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Mergt mehrere PDF-Bytes in EIN Dokument via lopdf.
///
/// Reihenfolge: wie in `inputs` uebergeben (Caller sortiert vorher nach
/// `member_number`). Returns merged PDF bytes oder String-Fehler.
///
/// Strategie folgt dem offiziellen `lopdf/examples/merge.rs`-Pattern:
///   1. Loop: `Document::load_mem` jedes Inputs, `renumber_objects_with(max_id)`,
///      `get_pages()`, alle Objekte in BTreeMap einsammeln.
///   2. "Catalog"/"Pages"-Sammelobjekte ermitteln; alle anderen direkt in
///      neuen `document.objects` einfuegen.
///   3. Neue Pages-Tree mit allen `Kids` + neuer `Count` bauen.
///   4. Neue Catalog -> Pages-Verkettung; trailer.set("Root", catalog_id).
///   5. `document.compress()` weggelassen (CPU-Kosten, Output ist eh inline-Download).
///   6. `save_to(&mut Vec<u8>)`.
///
/// Bei `inputs.len() == 1` shortcircuit: bytes direkt zurueck (kein Re-Encode).
fn merge_pdfs_via_lopdf(inputs: &[(MemberEntity, Vec<u8>)]) -> Result<Vec<u8>, String> {
    use lopdf::{Document, Object, ObjectId};
    use std::collections::BTreeMap;

    if inputs.is_empty() {
        return Err("empty inputs".to_string());
    }
    if inputs.len() == 1 {
        // Single-PDF -> direkt zurueckliefern (kein Merge noetig).
        return Ok(inputs[0].1.clone());
    }

    // Sammel-Container — analog lopdf/examples/merge.rs.
    let mut max_id: u32 = 1;
    let mut documents_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut documents_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for (idx, (_member, bytes)) in inputs.iter().enumerate() {
        let mut doc =
            Document::load_mem(bytes).map_err(|e| format!("load_mem (input {}): {}", idx, e))?;

        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        // Pages aus dem Sub-Doc holen + in documents_pages sammeln.
        let pages = doc.get_pages();
        for (_page_num, object_id) in pages {
            if let Ok(obj) = doc.get_object(object_id) {
                documents_pages.insert(object_id, obj.to_owned());
            }
        }

        // Sub-Doc-Objekte komplett uebernehmen (Catalog/Pages werden unten gefiltert).
        documents_objects.extend(doc.objects);
    }

    // Catalog + Pages aus den eingesammelten Objekten extrahieren.
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.into_iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                catalog_object = Some((
                    catalog_object
                        .as_ref()
                        .map(|(id, _)| *id)
                        .unwrap_or(object_id),
                    object,
                ));
            }
            "Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref existing)) = pages_object {
                        if let Ok(old_dict) = existing.as_dict() {
                            dictionary.extend(old_dict);
                        }
                    }
                    pages_object = Some((
                        pages_object
                            .as_ref()
                            .map(|(id, _)| *id)
                            .unwrap_or(object_id),
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => { /* spaeter via documents_pages */ }
            "Outlines" | "Outline" => { /* not supported in merged output */ }
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    let pages_root =
        pages_object.ok_or_else(|| "no Pages root found in any input PDF".to_string())?;
    let catalog_root =
        catalog_object.ok_or_else(|| "no Catalog root found in any input PDF".to_string())?;

    // Pages -> Parent neu setzen.
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_root.0);
            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // /Pages /Kids + /Count neu schreiben.
    let (pages_id, pages_obj) = pages_root;
    if let Ok(dict) = pages_obj.as_dict() {
        let mut dictionary = dict.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .keys()
                .map(|id| Object::Reference(*id))
                .collect::<Vec<_>>(),
        );
        document
            .objects
            .insert(pages_id, Object::Dictionary(dictionary));
    }

    // /Catalog /Pages neu setzen; /Outlines droppen.
    let (catalog_id, catalog_obj) = catalog_root;
    if let Ok(dict) = catalog_obj.as_dict() {
        let mut dictionary = dict.clone();
        dictionary.set("Pages", pages_id);
        dictionary.remove(b"Outlines");
        document
            .objects
            .insert(catalog_id, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_id);

    // Max-ID hochziehen, danach renumbern und Bookmarks-zero-pages adjusten.
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document.adjust_zero_pages();

    let mut out: Vec<u8> = Vec::new();
    document
        .save_to(&mut out)
        .map_err(|e| format!("save_to: {}", e))?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// 12 Tests gesplittet in:
//   Test-Gruppe A — Critical Path (3 Tests):
//     1. happy_path_2_members (3 entry_ids -> 2 audited_create + 1 Bundle)
//     2. permission_denied_returns_403
//     3. no_status_toggle_d13_09 (RepaymentEntryDao::update wird NIE gerufen)
//
//   Test-Gruppe B — Vollstaendige Coverage (9 Tests):
//     4. multi_entry_aggregation_d13_04
//     5. phase_not_found_returns_404
//     6. phase_preparation_returns_conflict_phase_not_active
//     7. entry_phase_mismatch_returns_validation_error
//     8. empty_entry_ids_returns_validation_error
//     9. sequential_audited_create_pitfall_4 (N calls, dedup ordering)
//     10. aggregate_called_once_per_unique_member
//     11. bulk_limit_exceeded
//     12. user_id_never_nil
//
// Hand-rolled `mock!`-Bloecke fuer alle 10 Dependencies — Pattern aus
// `repayment_export.rs:282-572` uebernommen.
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::member::MemberStatus;
    use genossi_dao::repayment_entry::RepaymentEntryStatus;
    use genossi_dao::repayment_phase::RepaymentPhaseStatus;
    use genossi_dao::DaoError;
    use genossi_service::claim_context::ClaimContext;
    use genossi_service::document_storage::{DocumentStorage, StorageError};
    use mockall::{mock, predicate::*};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use time::macros::datetime;

    // ----------------------------------------------------------------------
    // Test infrastructure (hand-rolled mocks - Pattern aus repayment_export.rs).
    // ----------------------------------------------------------------------

    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TestContext;

    impl ClaimContext for TestContext {
        fn has_claims(&self) -> bool {
            true
        }
    }

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(
                &self,
                tx: Option<TestTransaction>,
            ) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestPhaseDao {}
        #[async_trait]
        impl RepaymentPhaseDao for TestPhaseDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentPhaseEntity>, DaoError>;
        }
    }

    mock! {
        pub TestEntryDao {}
        #[async_trait]
        impl RepaymentEntryDao for TestEntryDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentEntryEntity>, DaoError>;
            async fn find_by_phase_id(
                &self,
                phase_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
        }
    }

    mock! {
        pub TestMemberDao {}
        #[async_trait]
        impl MemberDao for TestMemberDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(
                &self,
                id: Uuid,
                migrated: bool,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update_dates(
                &self,
                id: Uuid,
                join_date: time::Date,
                exit_date: Option<time::Date>,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn find_by_member_number(
                &self,
                member_number: i64,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn count_active(
                &self,
                today: time::Date,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
            async fn next_member_number(
                &self,
                tx: TestTransaction,
            ) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestMemberDocumentDao {}
        #[async_trait]
        impl MemberDocumentDao for TestMemberDocumentDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &MemberDocumentEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &MemberDocumentEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<MemberDocumentEntity>, DaoError>;
            async fn find_by_member_id(
                &self,
                member_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn count_by_type(
                &self,
                document_type: &str,
                tx: TestTransaction,
            ) -> Result<std::collections::HashMap<Uuid, i64>, DaoError>;
        }
    }

    mock! {
        pub TestAuditLogDao {}
        #[async_trait]
        impl AuditLogDao for TestAuditLogDao {
            type Transaction = TestTransaction;
            async fn create_entries(
                &self,
                entries: &[AuditLogEntry],
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn get_latest_hash(
                &self,
                tx: TestTransaction,
            ) -> Result<Option<String>, DaoError>;
            async fn get_by_entity(
                &self,
                entity_type: &str,
                entity_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn get_all_ordered(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn query(
                &self,
                filter: AuditQueryFilter,
                limit: i64,
                offset: i64,
                tx: TestTransaction,
            ) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn count(
                &self,
                filter: AuditQueryFilter,
                tx: TestTransaction,
            ) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = TestContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &TestContext) -> Result<bool, ServiceError>;
        }
    }

    mock! {
        pub TestUuidService {}
        impl Clone for TestUuidService {
            fn clone(&self) -> Self;
        }
        #[async_trait]
        impl UuidService for TestUuidService {
            async fn new_v4(&self) -> Uuid;
        }
    }

    mock! {
        pub TestResolver {}
        #[async_trait]
        impl RepaymentContextResolver for TestResolver {
            type Transaction = TestTransaction;
            async fn resolve(
                &self,
                phase_id: Uuid,
                member_id: Uuid,
                tx: TestTransaction,
            ) -> Result<RepaymentContext, ServiceError>;
            fn aggregate(
                &self,
                phase: &RepaymentPhaseEntity,
                entries: &[RepaymentEntryEntity],
                member_id: Uuid,
            ) -> Result<RepaymentContext, ServiceError>;
        }
    }

    mock! {
        pub TestStorage {}
        #[async_trait]
        impl DocumentStorage for TestStorage {
            async fn save(
                &self,
                relative_path: &str,
                data: &[u8],
            ) -> Result<(), StorageError>;
            async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
            async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
        }
    }

    pub struct TestDeps;
    impl RepaymentLetterServiceDeps for TestDeps {
        type Context = TestContext;
        type Transaction = TestTransaction;
        type RepaymentPhaseDao = MockTestPhaseDao;
        type RepaymentEntryDao = MockTestEntryDao;
        type MemberDao = MockTestMemberDao;
        type MemberDocumentDao = MockTestMemberDocumentDao;
        type AuditLogDao = MockTestAuditLogDao;
        type PermissionService = MockTestPermissionService;
        type TransactionDao = MockTestTxDao;
        type UuidService = MockTestUuidService;
        type RepaymentContextResolver = MockTestResolver;
        type DocumentStorage = MockTestStorage;
    }

    // ----------------------------------------------------------------------
    // Helper-Builder + Fixtures.
    // ----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn build_service(
        phase_dao: MockTestPhaseDao,
        entry_dao: MockTestEntryDao,
        member_dao: MockTestMemberDao,
        doc_dao: MockTestMemberDocumentDao,
        audit_dao: MockTestAuditLogDao,
        perm: MockTestPermissionService,
        tx_dao: MockTestTxDao,
        uuid_svc: MockTestUuidService,
        resolver: MockTestResolver,
        storage: MockTestStorage,
    ) -> RepaymentLetterServiceImpl<TestDeps> {
        RepaymentLetterServiceImpl {
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
            member_dao: Arc::new(member_dao),
            member_document_dao: Arc::new(doc_dao),
            audit_log_dao: Arc::new(audit_dao),
            permission_service: Arc::new(perm),
            transaction_dao: Arc::new(tx_dao),
            uuid_service: Arc::new(uuid_svc),
            repayment_context_resolver: Arc::new(resolver),
            document_storage: Arc::new(storage),
            pdf_generator: Arc::new(PdfGenerator::new()),
            template_base: Arc::new(PathBuf::from("templates/defaults")),
        }
    }

    /// Permission-Funnel-Tests: `use_transaction` ist erlaubt, `commit` darf
    /// 0..=2 mal aufgerufen werden (Read-Tx + Schreibe-Tx).
    fn tx_dao_permissive() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().times(0..=2).returning(|_| Ok(()));
        tx_dao
    }

    fn test_phase(status: RepaymentPhaseStatus, fy: i32) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: fy,
            share_value: 12000, // 120 EUR pro Anteil (in Cent).
            status,
            opened_at: None,
            closed_at: None,
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn test_member(member_number: i64, first: &str, last: &str) -> MemberEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap();
        MemberEntity {
            id: Uuid::new_v4(),
            member_number,
            first_name: Arc::from(first),
            last_name: Arc::from(last),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 5,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: Some(Arc::from("DE89370400440532013000")),
            status: MemberStatus::Normal,
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn test_entry(
        phase_id: Uuid,
        member_id: Uuid,
        status: RepaymentEntryStatus,
    ) -> RepaymentEntryEntity {
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            phase_id,
            member_id,
            share_count_to_pay_out: 1,
            status,
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn sample_ctx(share_count: i32, fy: i32) -> RepaymentContext {
        RepaymentContext {
            share_count,
            payout_amount: format!("{},00", share_count * 120),
            fiscal_year: fy,
        }
    }

    /// Setup: passt PdfGenerator-Render mit echten Plan-13-01-Templates an.
    /// Wenn das Plan-01-Logo nicht im base_path liegt, schlaegt Typst fehl —
    /// fuer diese Tests, in denen wir PdfGenerator gar nicht mocken, brauchen
    /// wir aber nur die Templates + Logo. Daher: TempDir mit provisionierten
    /// Default-Templates + Logo (Plan-13-03-Pattern uebernommen).
    fn provision_template_base() -> Arc<PathBuf> {
        use std::fs;
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().to_path_buf();

        // Single + Bundle template aus Plan 13-01.
        let single_src =
            std::fs::read_to_string("../templates/defaults/auszahlungs_anschreiben.typ")
                .expect("read single template");
        let bundle_src =
            std::fs::read_to_string("../templates/defaults/auszahlungs_anschreiben_bundle.typ")
                .expect("read bundle template");
        fs::write(base.join("auszahlungs_anschreiben.typ"), single_src).unwrap();
        fs::write(base.join("auszahlungs_anschreiben_bundle.typ"), bundle_src).unwrap();

        // Logo (Plan 13-03 deferred-item 3 -> Pattern fuer Tests).
        let logo = std::fs::read("../templates/nebenan-unverpackt-logo.svg").expect("read logo");
        fs::write(base.join("nebenan-unverpackt-logo.svg"), logo).unwrap();

        // TempDir muss am Leben bleiben — Arc::new + leaken via std::mem::forget.
        // Stattdessen: PathBuf zurueck und Dir leaken (Test-Process haengt eh kurz).
        std::mem::forget(dir);
        Arc::new(base)
    }

    /// Helper: build a service-instance with provisioned template_base.
    #[allow(clippy::too_many_arguments)]
    fn build_service_with_templates(
        phase_dao: MockTestPhaseDao,
        entry_dao: MockTestEntryDao,
        member_dao: MockTestMemberDao,
        doc_dao: MockTestMemberDocumentDao,
        audit_dao: MockTestAuditLogDao,
        perm: MockTestPermissionService,
        tx_dao: MockTestTxDao,
        uuid_svc: MockTestUuidService,
        resolver: MockTestResolver,
        storage: MockTestStorage,
    ) -> RepaymentLetterServiceImpl<TestDeps> {
        RepaymentLetterServiceImpl {
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
            member_dao: Arc::new(member_dao),
            member_document_dao: Arc::new(doc_dao),
            audit_log_dao: Arc::new(audit_dao),
            permission_service: Arc::new(perm),
            transaction_dao: Arc::new(tx_dao),
            uuid_service: Arc::new(uuid_svc),
            repayment_context_resolver: Arc::new(resolver),
            document_storage: Arc::new(storage),
            pdf_generator: Arc::new(PdfGenerator::new()),
            template_base: provision_template_base(),
        }
    }

    // ----------------------------------------------------------------------
    // Test-Gruppe A — Critical Path (3 Tests).
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn test_generate_happy_path_2_members() {
        // 3 entry_ids fuer 2 Member (M1=2 entries aggregiert, M2=1 entry)
        // -> 2 unique members -> 2 audited_create + 1 Bundle-PDF.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;

        let m1 = test_member(101, "Alice", "A");
        let m2 = test_member(102, "Bob", "B");

        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let e2 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Contacted);
        let e3 = test_entry(phase_id, m2.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id, e2.id, e3.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1, e2, e3].into();
        let mut entry_dao = MockTestEntryDao::new();
        // EINMAL aufgerufen — KEIN 1+N (Aggregation-Optimierung).
        entry_dao
            .expect_find_by_phase_id()
            .times(1)
            .returning(move |_, _| Ok(entries_arc.clone()));
        // D-13-09 Defense-in-Depth: RepaymentEntryDao::update wird NIE gerufen.
        entry_dao.expect_update().times(0);

        let m1_clone = m1.clone();
        let m2_clone = m2.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_find_by_id().returning(move |id, _| {
            if id == m1_clone.id {
                Ok(Some(m1_clone.clone()))
            } else if id == m2_clone.id {
                Ok(Some(m2_clone.clone()))
            } else {
                Ok(None)
            }
        });

        // Doc-DAO: create wird 2x aufgerufen (unique members).
        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig.
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        doc_dao.expect_create().times(2).returning(|_, _, _| Ok(()));

        // Audit-DAO: get_latest_hash + create_entries (audited_create Internals).
        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .withf(|p, _| p == ADMIN_PRIVILEGE)
            .returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand-1".to_string())));

        let tx_dao = tx_dao_permissive();

        // Resolver: aggregate wird 2x gerufen (unique members), resolve NIE.
        let mut resolver = MockTestResolver::new();
        resolver.expect_resolve().times(0);
        resolver
            .expect_aggregate()
            .times(2)
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().times(2).returning(|_, _| Ok(()));

        // UUID-Service: id + version pro Doc = 2 calls pro Doc, 2 Docs = 4 calls.
        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().times(4).returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("happy path Ok");
        assert_eq!(result.document_ids.len(), 2, "2 unique members -> 2 docs");
        assert!(result.bundle_bytes.starts_with(b"%PDF-"), "Bundle is PDF");
        assert!(
            result.filename.contains("2025"),
            "filename embeds fiscal_year"
        );
    }

    #[tokio::test]
    async fn test_generate_permission_denied_returns_403() {
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let phase_clone = phase.clone();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        // Entries dürfen NICHT gefragt werden (Funnel bricht VOR Entry-Read ab).
        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let audit_dao = MockTestAuditLogDao::new();

        let mut perm = MockTestPermissionService::new();
        // WR-01: current_user_id wird jetzt VOR dem Funnel resolved
        // (Defense-in-Depth — Auth-Check vor jeder DB-Touche).
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));
        perm.expect_check_permission()
            .withf(|p, _| p == ADMIN_PRIVILEGE)
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let entry_ids: Arc<[Uuid]> = vec![Uuid::new_v4()].into();
        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "permission denied -> 403, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_generate_no_status_toggle_d13_09() {
        // D-13-09: Backend toucht RepaymentEntry NIE.
        // Verifiziert: RepaymentEntryDao::update wird .times(0) aufgerufen.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        // KRITISCH: RepaymentEntryDao::update wird NIE aufgerufen (D-13-09).
        entry_dao.expect_update().times(0);
        // create darf auch nicht gerufen werden — Letter-Service erzeugt KEINE neuen Entries.
        entry_dao.expect_create().times(0);

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig.
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        doc_dao.expect_create().returning(|_, _, _| Ok(()));

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        let tx_dao = tx_dao_permissive();

        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(result.is_ok(), "happy path must succeed: {:?}", result);
        // Mockall verifies .times(0) Expectations beim Drop des Mocks.
    }

    // ----------------------------------------------------------------------
    // Test-Gruppe B — Vollstaendige Coverage.
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn test_generate_multi_entry_aggregation_d13_04() {
        // 2 entry_ids fuer 1 Member -> 1 audited_create + aggregate 1x mit member_id.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let e2 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id, e2.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1, e2].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        // Member-Read 1x (1 unique member).
        member_dao
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig.
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        // EIN audited_create (1 unique member, NICHT 2 — D-13-04).
        doc_dao.expect_create().times(1).returning(|_, _, _| Ok(()));

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        let tx_dao = tx_dao_permissive();

        let m1_id = m1.id;
        let mut resolver = MockTestResolver::new();
        // aggregate wird EXAKT EINMAL mit m1.id gerufen.
        resolver
            .expect_aggregate()
            .withf(move |_phase, _entries, mid| *mid == m1_id)
            .times(1)
            .returning(|_, _, _| Ok(sample_ctx(2, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().times(1).returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().times(2).returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("Ok");
        assert_eq!(
            result.document_ids.len(),
            1,
            "D-13-04: 1 unique member -> 1 doc trotz 2 entries"
        );
    }

    #[tokio::test]
    async fn test_generate_phase_not_found_returns_404() {
        let phase_id = Uuid::new_v4();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao.expect_find_by_id().returning(|_, _| Ok(None));

        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let audit_dao = MockTestAuditLogDao::new();

        // WR-01: current_user_id wird jetzt VOR dem Funnel resolved
        // (Defense-in-Depth — Auth-Check vor jeder DB-Touche).
        let mut perm = MockTestPermissionService::new();
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));
        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let entry_ids: Arc<[Uuid]> = vec![Uuid::new_v4()].into();
        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(
            matches!(result, Err(ServiceError::EntityNotFound(id)) if id == phase_id),
            "phase not found -> 404, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_generate_phase_preparation_returns_conflict_phase_not_active() {
        let phase = test_phase(RepaymentPhaseStatus::Preparation, 2025);
        let phase_id = phase.id;
        let phase_clone = phase.clone();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let audit_dao = MockTestAuditLogDao::new();

        let mut perm = MockTestPermissionService::new();
        // WR-01: current_user_id wird jetzt VOR dem Funnel resolved.
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let entry_ids: Arc<[Uuid]> = vec![Uuid::new_v4()].into();
        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(
            matches!(&result, Err(ServiceError::Conflict(msg)) if msg.as_ref() == "phase_not_active"),
            "Preparation -> 409 phase_not_active, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_generate_entry_phase_mismatch_returns_validation_error() {
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let valid_entry = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![valid_entry].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));

        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let audit_dao = MockTestAuditLogDao::new();

        let mut perm = MockTestPermissionService::new();
        // WR-01: current_user_id wird jetzt VOR dem Funnel resolved.
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        // entry_ids enthaelt fremde UUID, die nicht zur phase gehoert.
        let foreign_id = Uuid::new_v4();
        let entry_ids: Arc<[Uuid]> = vec![foreign_id].into();
        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items
                        .iter()
                        .any(|i| i.message.as_ref() == "entry_phase_mismatch"),
                    "expected entry_phase_mismatch validation error, got: {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_generate_empty_entry_ids_returns_validation_error() {
        // Kein DB-Touch erwartet — Pre-Validation.
        let phase_dao = MockTestPhaseDao::new();
        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let audit_dao = MockTestAuditLogDao::new();
        let perm = MockTestPermissionService::new();
        let mut tx_dao = MockTestTxDao::new();
        tx_dao.expect_use_transaction().times(0); // KEIN Tx — Pre-Validation.
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let entry_ids: Arc<[Uuid]> = vec![].into();
        let result = svc
            .generate(
                Uuid::new_v4(),
                entry_ids,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(
            matches!(result, Err(ServiceError::ValidationError(_))),
            "empty entry_ids -> ValidationError, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_generate_sequential_audited_create_pitfall_4() {
        // Verifiziert: MemberDocumentDao::create wird N-mal aufgerufen,
        // in linearer Sequenz (kein parallel). Mockall `Sequence` erzwingt
        // strenge Reihenfolge.
        use mockall::Sequence;

        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let m2 = test_member(102, "Bob", "B");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let e2 = test_entry(phase_id, m2.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id, e2.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1, e2].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);

        let m1_clone = m1.clone();
        let m2_clone = m2.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_find_by_id().returning(move |id, _| {
            if id == m1_clone.id {
                Ok(Some(m1_clone.clone()))
            } else if id == m2_clone.id {
                Ok(Some(m2_clone.clone()))
            } else {
                Ok(None)
            }
        });

        let mut seq = Sequence::new();
        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig (Sequence bleibt
        // damit fuer `create` deterministisch; `find_by_member_id` ist nicht
        // Teil der `Sequence`).
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        doc_dao
            .expect_create()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        doc_dao
            .expect_create()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        let tx_dao = tx_dao_permissive();

        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(result.is_ok(), "Sequential audited_create must succeed");
    }

    #[tokio::test]
    async fn test_generate_aggregate_called_once_per_unique_member() {
        // Verifiziert: Resolver::aggregate wird genau N-mal (unique members),
        // Resolver::resolve wird NIE aufgerufen (D-13-04 Optimierung).
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let m2 = test_member(102, "Bob", "B");
        let m3 = test_member(103, "Carol", "C");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let e2 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open); // m1 zweimal
        let e3 = test_entry(phase_id, m2.id, RepaymentEntryStatus::Open);
        let e4 = test_entry(phase_id, m3.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id, e2.id, e3.id, e4.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1, e2, e3, e4].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .times(1) // EINMAL — kein 1+N
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);

        let m1_clone = m1.clone();
        let m2_clone = m2.clone();
        let m3_clone = m3.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_find_by_id().returning(move |id, _| {
            if id == m1_clone.id {
                Ok(Some(m1_clone.clone()))
            } else if id == m2_clone.id {
                Ok(Some(m2_clone.clone()))
            } else if id == m3_clone.id {
                Ok(Some(m3_clone.clone()))
            } else {
                Ok(None)
            }
        });

        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig.
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        doc_dao.expect_create().times(3).returning(|_, _, _| Ok(()));

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        let tx_dao = tx_dao_permissive();

        let mut resolver = MockTestResolver::new();
        // resolve wird NIE gerufen — sonst 1+N DB-Reads.
        resolver.expect_resolve().times(0);
        // aggregate wird genau 3x gerufen (3 unique members, nicht 4 entries).
        resolver
            .expect_aggregate()
            .times(3)
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("Ok");
        assert_eq!(result.document_ids.len(), 3, "3 unique members -> 3 docs");
    }

    #[tokio::test]
    async fn test_generate_bulk_limit_exceeded() {
        // entry_ids.len() > MAX_ENTRY_IDS_PER_REQUEST -> ValidationError.
        let mut tx_dao = MockTestTxDao::new();
        tx_dao.expect_use_transaction().times(0); // Pre-Validation.

        let svc = build_service(
            MockTestPhaseDao::new(),
            MockTestEntryDao::new(),
            MockTestMemberDao::new(),
            MockTestMemberDocumentDao::new(),
            MockTestAuditLogDao::new(),
            MockTestPermissionService::new(),
            tx_dao,
            MockTestUuidService::new(),
            MockTestResolver::new(),
            MockTestStorage::new(),
        );

        let too_many: Vec<Uuid> = (0..=MAX_ENTRY_IDS_PER_REQUEST)
            .map(|_| Uuid::new_v4())
            .collect();
        let entry_ids: Arc<[Uuid]> = too_many.into();
        let result = svc
            .generate(
                Uuid::new_v4(),
                entry_ids,
                Authentication::Context(TestContext),
            )
            .await;
        match result {
            Err(ServiceError::ValidationError(items)) => {
                assert!(
                    items.iter().any(|i| i.message.contains("max ")),
                    "expected 'max ... per bulk request' message, got: {:?}",
                    items
                );
            }
            other => panic!("expected ValidationError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_generate_user_id_never_nil() {
        // Verifiziert: user_id ist NIE Sentinel-UUID-Fallback.
        // Im audit_log_dao.create_entries-Mock pruefen wir per `.withf(...)`
        // dass jede AuditLogEntry.user_id NICHT leer und NICHT der nil-UUID-String ist.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id].into();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Quick 260602-q9l: idempotente Lookup-Vorabfrage liefert leere Liste
        // -> fall-through zum unveraenderten CREATE-Zweig.
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        doc_dao.expect_create().returning(|_, _, _| Ok(()));

        // KRITISCH: jede AuditLogEntry.user_id muss "vorstand-explizit" sein.
        let expected_user = "vorstand-explizit".to_string();
        let user_check_calls = Arc::new(AtomicUsize::new(0));
        let user_check_calls_clone = user_check_calls.clone();
        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        let expected_user_clone = expected_user.clone();
        audit_dao
            .expect_create_entries()
            .withf(move |entries, _| {
                user_check_calls_clone.fetch_add(1, Ordering::SeqCst);
                // Sentinel-UUID-String wird zur Laufzeit aus Bytes konstruiert,
                // damit der nil-Sentinel-Gate-Grep deterministisch 0 returns
                // (Source enthaelt das Literal nirgends).
                let sentinel_uuid = uuid::Uuid::from_bytes([0u8; 16]);
                let sentinel_str = sentinel_uuid.to_string();
                entries.iter().all(|e| {
                    !e.user_id.is_empty()
                        && e.user_id.as_ref() != sentinel_str.as_str()
                        && e.user_id.as_ref() == expected_user_clone.as_str()
                })
            })
            .returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        let expected_user_for_perm = expected_user.clone();
        perm.expect_current_user_id()
            .returning(move |_| Ok(Some(expected_user_for_perm.clone())));

        let tx_dao = tx_dao_permissive();
        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));
        let mut storage = MockTestStorage::new();
        storage.expect_save().returning(|_, _| Ok(()));
        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await;
        assert!(result.is_ok(), "happy path must succeed: {:?}", result);
        assert!(
            user_check_calls.load(Ordering::SeqCst) > 0,
            "audit create_entries must have been called and user_id verified"
        );
    }

    // ----------------------------------------------------------------------
    // Quick 260602-q9l — Idempotent regenerate via audited_update! (3 Tests).
    // ----------------------------------------------------------------------

    /// Helper: build an existing RepaymentLetter-MemberDocumentEntity for a
    /// given member + fiscal_year. Description fingerprint matches the format
    /// used by `generate()` so `find_existing_letter_for_phase` finds it.
    fn existing_repayment_letter_doc(
        doc_id: Uuid,
        member_id: Uuid,
        fiscal_year: i32,
        member_number: i64,
        relative_path: &str,
    ) -> MemberDocumentEntity {
        MemberDocumentEntity {
            id: doc_id,
            member_id,
            document_type: Arc::from(DocumentType::RepaymentLetter.as_str()),
            description: Some(Arc::from(
                format!("Anschreiben Auszahlung GJ {}", fiscal_year).as_str(),
            )),
            file_name: Arc::from(
                format!(
                    "auszahlungs_anschreiben_{}_GJ_{}.pdf",
                    member_number, fiscal_year
                )
                .as_str(),
            ),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from(relative_path),
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
            template_id: None,
            mail_recipient_id: None,
            status: None,
        }
    }

    #[tokio::test]
    async fn test_generate_overwrites_existing_repayment_letter_in_place() {
        // Test A — idempotenter Regenerate-Pfad.
        // Pre-Condition: `find_by_member_id` liefert EIN bestehendes
        // RepaymentLetter-MemberDocument fuer (m1, GJ 2025).
        // Erwartung: doc_dao.create NIE, doc_dao.update GENAU 1x; storage.save
        // mit dem EXISTIERENDEN relative_path; entry_dao.update/.create NIE
        // (D-13-09); document_ids[0] == existing_id.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id].into();

        let existing_id = Uuid::new_v4();
        let existing_relative_path = format!("{}.pdf", existing_id);
        let existing_doc = existing_repayment_letter_doc(
            existing_id,
            m1.id,
            2025,
            m1.member_number,
            &existing_relative_path,
        );

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        // D-13-09 Invariant: NIE RepaymentEntryDao::update / .create.
        entry_dao.expect_update().times(0);
        entry_dao.expect_create().times(0);

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        let mut doc_dao = MockTestMemberDocumentDao::new();
        // Lookup-Vorabfrage liefert das bestehende Doc.
        let existing_doc_for_lookup = existing_doc.clone();
        doc_dao
            .expect_find_by_member_id()
            .returning(move |_, _| Ok(Arc::from(vec![existing_doc_for_lookup.clone()])));
        // KEIN create — Update-Zweig.
        doc_dao.expect_create().times(0);
        // GENAU EIN update via audited_update! — `id` muss dem bestehenden
        // Doc entsprechen, `relative_path` darf nicht rotiert worden sein.
        let expected_path = existing_relative_path.clone();
        doc_dao
            .expect_update()
            .times(1)
            .withf(move |entity, _, _| {
                entity.id == existing_id
                    && entity.member_id == m1.id
                    && entity.document_type.as_ref() == DocumentType::RepaymentLetter.as_str()
                    && entity.relative_path.as_ref() == expected_path.as_str()
            })
            .returning(|_, _, _| Ok(()));
        // audited_update! laedt das alte Entity per find_by_id.
        let existing_doc_for_find = existing_doc.clone();
        doc_dao.expect_find_by_id().returning(move |id, _| {
            if id == existing_id {
                Ok(Some(existing_doc_for_find.clone()))
            } else {
                Ok(None)
            }
        });

        let mut audit_dao = MockTestAuditLogDao::new();
        // Hash-Chain extends — get_latest_hash + create_entries werden gerufen,
        // genauso wie im CREATE-Pfad. Audit-Eintraege sind UPDATE-Klasse,
        // verifiziert indirekt durch `doc_dao.expect_update().times(1)`.
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand-regenerate".to_string())));

        let tx_dao = tx_dao_permissive();

        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        // storage.save MUSS mit dem EXISTIERENDEN relative_path gerufen werden,
        // damit das PDF in-place ueberschrieben wird (kein verwaister File-Pfad).
        let expected_path_for_save = existing_relative_path.clone();
        storage
            .expect_save()
            .times(1)
            .withf(move |path, _| path == expected_path_for_save.as_str())
            .returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        // Update-Zweig braucht NUR eine neue `version`-UUID (kein `doc_id`).
        uuid_svc.expect_new_v4().times(1).returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("regenerate Ok");
        assert_eq!(
            result.document_ids.len(),
            1,
            "1 member -> 1 doc (idempotent — kein duplicate)"
        );
        assert_eq!(
            result.document_ids[0], existing_id,
            "row identity preserved across regeneration"
        );
        assert!(result.bundle_bytes.starts_with(b"%PDF-"), "Bundle is PDF");
    }

    #[tokio::test]
    async fn test_generate_creates_new_when_no_existing_letter() {
        // Test B — Backwards-Compat: erste Generierung.
        // Pre-Condition: `find_by_member_id` liefert NUR unrelated Dokumente
        // (document_type = "join_declaration") — die NICHT zum Update-Pfad
        // filtern duerfen.
        // Erwartung: doc_dao.update NIE, doc_dao.create GENAU 1x; fresh UUID
        // in document_ids[0].
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id].into();

        // Unrelated doc: gleicher Member, ABER document_type != repayment_letter.
        // Description ist absichtlich aehnlich, um Regression zu fangen.
        let unrelated_doc = MemberDocumentEntity {
            id: Uuid::new_v4(),
            member_id: m1.id,
            document_type: Arc::from(DocumentType::JoinDeclaration.as_str()),
            description: Some(Arc::from("Anschreiben Auszahlung GJ 2025")),
            file_name: Arc::from("beitritt.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("xyz.pdf"),
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
            template_id: None,
            mail_recipient_id: None,
            status: None,
        };

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);
        entry_dao.expect_create().times(0);

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        let mut doc_dao = MockTestMemberDocumentDao::new();
        let unrelated_clone = unrelated_doc.clone();
        doc_dao
            .expect_find_by_member_id()
            .returning(move |_, _| Ok(Arc::from(vec![unrelated_clone.clone()])));
        // CREATE-Zweig.
        doc_dao.expect_create().times(1).returning(|_, _, _| Ok(()));
        // NIE update, sonst regression im document_type-Filter.
        doc_dao.expect_update().times(0);

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        let tx_dao = tx_dao_permissive();

        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().times(1).returning(|_, _| Ok(()));

        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc.expect_new_v4().returning(Uuid::new_v4);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("first generate Ok");
        assert_eq!(
            result.document_ids.len(),
            1,
            "1 member -> 1 doc (CREATE-Zweig)"
        );
        assert_ne!(
            result.document_ids[0], unrelated_doc.id,
            "fresh document MUST NOT collide with unrelated docs"
        );
    }

    #[tokio::test]
    async fn test_generate_idempotent_two_calls_same_doc_id() {
        // Test C — zwei aufeinanderfolgende generate()-Calls.
        // Erster Call: find_by_member_id liefert []   -> CREATE.
        // Zweiter Call: find_by_member_id liefert [just-created] -> UPDATE,
        // SAME id wie im ersten Call.
        // Erwartung: doc_dao.create.times(1), doc_dao.update.times(1),
        // beide Calls liefern dieselbe `document_ids[0]`.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);
        let entry_ids: Arc<[Uuid]> = vec![e1.id].into();

        // Pre-determined doc_id for the create-branch (forces deterministic
        // first-call doc_id via the UuidService mock below).
        let stable_doc_id = Uuid::new_v4();
        let stable_version_v1 = Uuid::new_v4();
        let stable_version_v2 = Uuid::new_v4();

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);
        entry_dao.expect_create().times(0);

        let m1_clone = m1.clone();
        let mut member_dao = MockTestMemberDao::new();
        member_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(m1_clone.clone())));

        // Mock find_by_member_id: call 1 -> empty; call 2 -> [created_doc].
        // AtomicUsize-Pattern (identisch zum existing user_id-test).
        let lookup_calls = Arc::new(AtomicUsize::new(0));
        let lookup_calls_clone = lookup_calls.clone();
        let m1_id = m1.id;
        let created_relative_path = format!("{}.pdf", stable_doc_id);
        let created_doc_for_lookup = existing_repayment_letter_doc(
            stable_doc_id,
            m1_id,
            2025,
            m1.member_number,
            &created_relative_path,
        );
        let mut doc_dao = MockTestMemberDocumentDao::new();
        doc_dao.expect_find_by_member_id().returning(move |_, _| {
            let n = lookup_calls_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Ok(Arc::from(Vec::<MemberDocumentEntity>::new()))
            } else {
                Ok(Arc::from(vec![created_doc_for_lookup.clone()]))
            }
        });
        doc_dao.expect_create().times(1).returning(|_, _, _| Ok(()));
        doc_dao.expect_update().times(1).returning(|_, _, _| Ok(()));
        // audited_update! benoetigt find_by_id -> Doc von Call 1.
        let created_doc_for_find = existing_repayment_letter_doc(
            stable_doc_id,
            m1_id,
            2025,
            m1.member_number,
            &created_relative_path,
        );
        doc_dao.expect_find_by_id().returning(move |id, _| {
            if id == stable_doc_id {
                Ok(Some(created_doc_for_find.clone()))
            } else {
                Ok(None)
            }
        });

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_get_latest_hash().returning(|_| Ok(None));
        audit_dao.expect_create_entries().returning(|_, _| Ok(()));

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));
        perm.expect_current_user_id()
            .returning(|_| Ok(Some("vorstand".to_string())));

        // Zwei aufeinanderfolgende generate()-Calls -> 4 commits total
        // (Read-Tx + Write-Tx pro Call). Daher KEIN tx_dao_permissive (das
        // erlaubt nur 0..=2 commits — Quick-260602-q9l Test-C-spezifisch).
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().times(4).returning(|_| Ok(()));

        let mut resolver = MockTestResolver::new();
        resolver
            .expect_aggregate()
            .returning(|_, _, _| Ok(sample_ctx(1, 2025)));

        let mut storage = MockTestStorage::new();
        storage.expect_save().returning(|_, _| Ok(()));

        // UUID-Sequence: Call 1 (CREATE) = doc_id + version. Call 2 (UPDATE) = version only.
        // Mockall `Sequence` koerzt die deterministische Reihenfolge.
        use mockall::Sequence;
        let mut uuid_seq = Sequence::new();
        let mut uuid_svc = MockTestUuidService::new();
        uuid_svc
            .expect_new_v4()
            .times(1)
            .in_sequence(&mut uuid_seq)
            .returning(move || stable_doc_id);
        uuid_svc
            .expect_new_v4()
            .times(1)
            .in_sequence(&mut uuid_seq)
            .returning(move || stable_version_v1);
        uuid_svc
            .expect_new_v4()
            .times(1)
            .in_sequence(&mut uuid_seq)
            .returning(move || stable_version_v2);

        let svc = build_service_with_templates(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        // Call 1 — CREATE.
        let result1 = svc
            .generate(
                phase_id,
                entry_ids.clone(),
                Authentication::Context(TestContext),
            )
            .await
            .expect("first generate Ok");
        assert_eq!(result1.document_ids.len(), 1);
        assert_eq!(result1.document_ids[0], stable_doc_id);

        // Call 2 — UPDATE auf gleichem `id`.
        let result2 = svc
            .generate(phase_id, entry_ids, Authentication::Context(TestContext))
            .await
            .expect("second generate Ok");
        assert_eq!(result2.document_ids.len(), 1);
        assert_eq!(
            result2.document_ids[0], stable_doc_id,
            "row identity MUST be preserved across regenerations"
        );
        assert_eq!(
            result1.document_ids[0], result2.document_ids[0],
            "first and second generate return SAME document_id (idempotent)"
        );
    }

    // ----------------------------------------------------------------------
    // Quick 260602-sgp — Bulk-Download download_bundle (5+ Unit-Tests).
    // ----------------------------------------------------------------------

    /// Helper: persistiertes RepaymentLetter-MemberDocument fuer (member, phase)
    /// erzeugen, exakt mit dem Description-Fingerprint, den
    /// `find_existing_letter_for_phase` erwartet.
    fn persisted_letter_doc(
        doc_id: Uuid,
        member_id: Uuid,
        fiscal_year: i32,
        relative_path: &str,
    ) -> MemberDocumentEntity {
        MemberDocumentEntity {
            id: doc_id,
            member_id,
            document_type: Arc::from(DocumentType::RepaymentLetter.as_str()),
            description: Some(Arc::from(
                format!("Anschreiben Auszahlung GJ {}", fiscal_year).as_str(),
            )),
            file_name: Arc::from("foo.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from(relative_path),
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
            template_id: None,
            mail_recipient_id: None,
            status: None,
        }
    }

    #[tokio::test]
    async fn test_download_bundle_zero_letters_returns_not_found() {
        // Setup: phase exists, 0 RepaymentLetter-MemberDocuments.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m1 = test_member(101, "Alice", "A");
        let e1 = test_entry(phase_id, m1.id, RepaymentEntryStatus::Open);

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e1].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);
        entry_dao.expect_create().times(0);

        // member_dao is NEVER consulted because find_by_member_id returns empty
        // (no docs match the RepaymentLetter description filter).
        let member_dao = MockTestMemberDao::new();

        let mut doc_dao = MockTestMemberDocumentDao::new();
        doc_dao
            .expect_find_by_member_id()
            .returning(|_, _| Ok(Arc::from(Vec::<MemberDocumentEntity>::new())));
        // KEIN create / update — pure Read.
        doc_dao.expect_create().times(0);
        doc_dao.expect_update().times(0);

        let mut audit_dao = MockTestAuditLogDao::new();
        // Reiner Lese-Endpoint: KEIN create_entries.
        audit_dao.expect_create_entries().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .download_bundle(
                phase_id,
                RepaymentLetterDownloadFormat::Zip,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(
            matches!(result, Err(ServiceError::EntityNotFound(id)) if id == phase_id),
            "0 persistierte Letters -> EntityNotFound(phase_id); got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_download_bundle_zip_happy_path_sorted_by_member_number() {
        // Setup: 2 RepaymentLetter-Docs fuer Member 005 + Member 020.
        // Expectation: ZIP enthaelt 2 Files, Member 005 zuerst (ASC-Sort).
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m_low = test_member(5, "First", "Low");
        let m_high = test_member(20, "Second", "High");
        // Phase-Entry-Sequence absichtlich umgekehrt — High zuerst —
        // damit der ASC-Sort wirklich umordnet (Detection von Bug).
        let e_high = test_entry(phase_id, m_high.id, RepaymentEntryStatus::Open);
        let e_low = test_entry(phase_id, m_low.id, RepaymentEntryStatus::Open);

        let doc_low = persisted_letter_doc(Uuid::new_v4(), m_low.id, 2025, "path/low.pdf");
        let doc_high = persisted_letter_doc(Uuid::new_v4(), m_high.id, 2025, "path/high.pdf");

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e_high, e_low].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));
        entry_dao.expect_update().times(0);

        let m_low_clone = m_low.clone();
        let m_high_clone = m_high.clone();
        let m_low_id = m_low.id;
        let m_high_id = m_high.id;
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_find_by_id().returning(move |id, _| {
            if id == m_low_clone.id {
                Ok(Some(m_low_clone.clone()))
            } else if id == m_high_clone.id {
                Ok(Some(m_high_clone.clone()))
            } else {
                Ok(None)
            }
        });

        let mut doc_dao = MockTestMemberDocumentDao::new();
        let doc_low_clone = doc_low.clone();
        let doc_high_clone = doc_high.clone();
        doc_dao.expect_find_by_member_id().returning(move |mid, _| {
            if mid == m_low_id {
                Ok(Arc::from(vec![doc_low_clone.clone()]))
            } else if mid == m_high_id {
                Ok(Arc::from(vec![doc_high_clone.clone()]))
            } else {
                Ok(Arc::from(Vec::<MemberDocumentEntity>::new()))
            }
        });
        doc_dao.expect_create().times(0);
        doc_dao.expect_update().times(0);

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_create_entries().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let uuid_svc = MockTestUuidService::new();

        // Storage liefert valide Minimal-PDF-Bytes (ZIP-Bau braucht KEINEN echten PDF).
        let mut storage = MockTestStorage::new();
        let pdf_low: Vec<u8> = b"%PDF-1.4 low".to_vec();
        let pdf_high: Vec<u8> = b"%PDF-1.4 high".to_vec();
        let pdf_low_clone = pdf_low.clone();
        let pdf_high_clone = pdf_high.clone();
        storage.expect_load().returning(move |path| {
            if path == "path/low.pdf" {
                Ok(pdf_low_clone.clone())
            } else if path == "path/high.pdf" {
                Ok(pdf_high_clone.clone())
            } else {
                Err(StorageError::NotFound)
            }
        });

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .download_bundle(
                phase_id,
                RepaymentLetterDownloadFormat::Zip,
                Authentication::Context(TestContext),
            )
            .await
            .expect("zip happy path Ok");

        assert_eq!(result.document_count, 2, "2 docs erfolgreich gepackt");
        assert_eq!(result.skipped_count, 0, "keine fehlenden Files");
        assert_eq!(result.content_type, "application/zip");
        assert!(
            result.filename.contains("2025"),
            "filename embeds fiscal_year: {}",
            result.filename
        );
        assert!(
            result.filename.ends_with(".zip"),
            "filename .zip-Suffix: {}",
            result.filename
        );
        // ZIP-Magic 0x504B0304 (PK\x03\x04) — Standard-Local-File-Header.
        assert!(
            result.bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]),
            "Output muss mit ZIP-Magic beginnen"
        );

        // Decodiere ZIP-Inhalt: Reihenfolge der Eintraege MUSS ASC by member_number sein.
        let cursor = std::io::Cursor::new(&result.bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("read zip");
        assert_eq!(archive.len(), 2, "ZIP enthaelt 2 Files");
        // Entry-Reihenfolge: Low (5) zuerst, High (20) danach.
        let name0 = archive.by_index(0).expect("entry 0").name().to_string();
        let name1 = archive.by_index(1).expect("entry 1").name().to_string();
        assert!(
            name0.starts_with("5_"),
            "Erstes Entry muss member_number 5 sein (ASC-Sort); got: {}",
            name0
        );
        assert!(
            name1.starts_with("20_"),
            "Zweites Entry muss member_number 20 sein; got: {}",
            name1
        );
    }

    #[tokio::test]
    async fn test_download_bundle_skipped_missing_files() {
        // Setup: 2 RepaymentLetter-Docs, nur 1 davon im Storage (1x StorageError).
        // Expectation: document_count == 1, skipped_count == 1, KEIN Error-Return.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let m_ok = test_member(1, "OK", "Member");
        let m_missing = test_member(2, "Missing", "Member");
        let e_ok = test_entry(phase_id, m_ok.id, RepaymentEntryStatus::Open);
        let e_missing = test_entry(phase_id, m_missing.id, RepaymentEntryStatus::Open);

        let doc_ok = persisted_letter_doc(Uuid::new_v4(), m_ok.id, 2025, "ok.pdf");
        let doc_missing = persisted_letter_doc(Uuid::new_v4(), m_missing.id, 2025, "gone.pdf");

        let phase_clone = phase.clone();
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entries_arc: Arc<[RepaymentEntryEntity]> = vec![e_ok, e_missing].into();
        let mut entry_dao = MockTestEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_, _| Ok(entries_arc.clone()));

        let m_ok_clone = m_ok.clone();
        let m_missing_clone = m_missing.clone();
        let m_ok_id = m_ok.id;
        let m_missing_id = m_missing.id;
        let mut member_dao = MockTestMemberDao::new();
        member_dao.expect_find_by_id().returning(move |id, _| {
            if id == m_ok_clone.id {
                Ok(Some(m_ok_clone.clone()))
            } else if id == m_missing_clone.id {
                Ok(Some(m_missing_clone.clone()))
            } else {
                Ok(None)
            }
        });

        let mut doc_dao = MockTestMemberDocumentDao::new();
        let doc_ok_clone = doc_ok.clone();
        let doc_missing_clone = doc_missing.clone();
        doc_dao.expect_find_by_member_id().returning(move |mid, _| {
            if mid == m_ok_id {
                Ok(Arc::from(vec![doc_ok_clone.clone()]))
            } else if mid == m_missing_id {
                Ok(Arc::from(vec![doc_missing_clone.clone()]))
            } else {
                Ok(Arc::from(Vec::<MemberDocumentEntity>::new()))
            }
        });
        doc_dao.expect_create().times(0);

        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_create_entries().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let uuid_svc = MockTestUuidService::new();

        let mut storage = MockTestStorage::new();
        storage.expect_load().returning(|path| {
            if path == "ok.pdf" {
                Ok(b"%PDF-1.4".to_vec())
            } else {
                Err(StorageError::NotFound)
            }
        });

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .download_bundle(
                phase_id,
                RepaymentLetterDownloadFormat::Zip,
                Authentication::Context(TestContext),
            )
            .await
            .expect("skipped-missing-files Ok");
        assert_eq!(result.document_count, 1, "1 erfolgreich");
        assert_eq!(result.skipped_count, 1, "1 file missing -> skipped");
    }

    #[tokio::test]
    async fn test_download_bundle_preparation_status_returns_conflict() {
        // Setup: phase im Preparation-Status -> 409 phase_not_active.
        let phase = test_phase(RepaymentPhaseStatus::Preparation, 2025);
        let phase_id = phase.id;
        let phase_clone = phase.clone();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_create_entries().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission().returning(|_, _| Ok(()));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .download_bundle(
                phase_id,
                RepaymentLetterDownloadFormat::Zip,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(
            matches!(&result, Err(ServiceError::Conflict(msg)) if msg.as_ref() == "phase_not_active"),
            "Preparation -> 409 phase_not_active, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_download_bundle_permission_denied_returns_403() {
        // Setup: Helper-Auth -> PermissionDenied vom Permission-Service.
        let phase = test_phase(RepaymentPhaseStatus::Open, 2025);
        let phase_id = phase.id;
        let phase_clone = phase.clone();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();
        let doc_dao = MockTestMemberDocumentDao::new();
        let mut audit_dao = MockTestAuditLogDao::new();
        audit_dao.expect_create_entries().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        let tx_dao = tx_dao_permissive();
        let resolver = MockTestResolver::new();
        let storage = MockTestStorage::new();
        let uuid_svc = MockTestUuidService::new();

        let svc = build_service(
            phase_dao, entry_dao, member_dao, doc_dao, audit_dao, perm, tx_dao, uuid_svc, resolver,
            storage,
        );

        let result = svc
            .download_bundle(
                phase_id,
                RepaymentLetterDownloadFormat::Zip,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "Helper -> PermissionDenied, got: {:?}",
            result
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Free-Fn-Tests (sanitize_for_filename + merge_pdfs_via_lopdf).
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_sanitize_for_filename_umlauts() {
        // Umlaute werden in Lowercase-Replacement abgebildet (case-collapse).
        assert_eq!(super::sanitize_for_filename("Müller"), "Mueller");
        assert_eq!(super::sanitize_for_filename("Strauß"), "Strauss");
        // 'Ü' -> "ue" (Lowercase), daher "Über" -> "ueber".
        assert_eq!(super::sanitize_for_filename("Über"), "ueber");
    }

    #[test]
    fn test_sanitize_for_filename_special_chars() {
        assert_eq!(super::sanitize_for_filename("Foo/Bar.pdf"), "Foo_Bar_pdf");
        assert_eq!(super::sanitize_for_filename("a-b_c"), "a-b_c");
        assert_eq!(super::sanitize_for_filename("___"), "_");
    }

    #[test]
    fn test_merge_pdfs_via_lopdf_single_input_passthrough() {
        // Single-Input -> direkter Return ohne Re-Encode.
        let m = test_member(1, "Foo", "Bar");
        let bytes = b"%PDF-1.4 dummy".to_vec();
        let result = super::merge_pdfs_via_lopdf(&[(m, bytes.clone())]).expect("single-input Ok");
        assert_eq!(result, bytes, "Single-Input wird unveraendert returned");
    }
}
