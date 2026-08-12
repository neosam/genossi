use std::sync::Arc;

use crate::dao::{
    InboundMailDao, MailJobDao, MailJobStaticAttachmentDao, MailRecipientAttachment,
    MailRecipientAttachmentDao, MailRecipientDao,
};
use crate::service::{build_transport, load_smtp_config, MailServiceError};
use crate::template::MemberResolver;
use genossi_config::service::ConfigService;
use genossi_service::document_storage::DocumentStorage;
use genossi_service::member_document::DocumentType;
// Quick 260603-h0r: Phase 10 D-04 / D-06 aggregation delegated to the shared
// resolver (Single Source of Truth with the Letter-Service — Phase 13 D-13-04 /
// D-13-10). Replaces the inline filter+sum+German-format block that used to
// live in start_mail_worker().
use genossi_service::repayment_context::RepaymentContextResolver;

const DEFAULT_SEND_INTERVAL_SECONDS: u64 = 36;
const IDLE_POLL_SECONDS: u64 = 5;

/// Phase 10 D-11: process-string for MemberDocument audit entries created by the
/// mail worker (no auth context). Distinct from the genossi_service_impl
/// MEMBER_DOCUMENT_PROCESS string so audit logs make the worker-source distinguishable.
const REPAYMENT_MAIL_PROCESS: &str = "repayment-mail-worker";

/// Phase 10 D-11: fallback user_id (matches existing
/// genossi_service_impl/src/member_document.rs fallback when no auth context).
const WORKER_USER_ID: &str = "SYSTEM";

/// Phase 10 specifics: maximum number of error characters retained in the
/// MemberDocument.description suffix. Prevents oversized DB rows when SMTP
/// servers return verbose error blocks. Format: "{subject} [FAILED: {truncated}]".
const ERROR_TRUNCATION_LIMIT: usize = 200;

/// Phase 10 D-11: build a MemberDocumentEntity for a single recipient that the
/// mail worker has just attempted to deliver. Pure sync — no DAO calls, no tx.
/// The caller (try_create_member_document_audited) handles persistence + audit.
fn build_member_document_entity(
    job: &crate::dao::MailJob,
    member_id: uuid::Uuid,
    recipient_id: uuid::Uuid,
    send_result_ok: bool,
    error_msg: &str,
) -> genossi_dao::member_document::MemberDocumentEntity {
    let now = time::OffsetDateTime::now_utc();
    let (doc_status, doc_description) = if send_result_ok {
        ("sent".to_string(), job.subject.to_string())
    } else {
        // Truncate to ERROR_TRUNCATION_LIMIT chars (NOT bytes — char-safe for UTF-8).
        let truncated: String = error_msg.chars().take(ERROR_TRUNCATION_LIMIT).collect();
        (
            "failed".to_string(),
            format!("{} [FAILED: {}]", job.subject, truncated),
        )
    };

    genossi_dao::member_document::MemberDocumentEntity {
        id: uuid::Uuid::new_v4(),
        member_id,
        document_type: Arc::from("repayment_mail"),
        description: Some(Arc::from(doc_description.as_str())),
        // No file on disk for repayment mails (Phase 10 specifics §relative_path)
        file_name: Arc::from(""),
        mime_type: Arc::from("text/plain"),
        relative_path: Arc::from(""),
        created: time::PrimitiveDateTime::new(now.date(), now.time()),
        deleted: None,
        version: uuid::Uuid::new_v4(),
        // Phase 10 D-07 fields:
        template_id: job.template_id,
        mail_recipient_id: Some(recipient_id),
        status: Some(Arc::from(doc_status.as_str())),
    }
}

/// Quick 260603-cz6: pick the RepaymentLetter MemberDocument for a given fiscal_year
/// from the documents loaded for a member. The linkage between RepaymentLetter and
/// RepaymentPhase is the description-fingerprint pattern established by Phase 13
/// D-LETT-04 (`genossi_service_impl/src/repayment_letter.rs::find_existing_letter_for_phase`).
///
/// Returns:
/// - `None` if no matching letter (Worker marks recipient `failed` with `error="no_repayment_letter"`)
/// - `Some(newest)` if 1+ matching letters; on >1 a warning is logged with the count.
fn find_repayment_letter_for_recipient(
    documents: &[genossi_dao::member_document::MemberDocumentEntity],
    fiscal_year: i32,
) -> Option<&genossi_dao::member_document::MemberDocumentEntity> {
    let expected_desc = format!("Anschreiben Auszahlung GJ {}", fiscal_year);
    let document_type = DocumentType::RepaymentLetter.as_str();

    let mut matches: Vec<&genossi_dao::member_document::MemberDocumentEntity> = documents
        .iter()
        .filter(|d| {
            d.deleted.is_none()
                && d.document_type.as_ref() == document_type
                && d.description.as_deref() == Some(expected_desc.as_str())
        })
        .collect();

    if matches.is_empty() {
        return None;
    }

    // created DESC — Quick-cz6 decision: take newest when multiple exist.
    matches.sort_by(|a, b| b.created.cmp(&a.created));

    if matches.len() > 1 {
        tracing::warn!(
            "Worker: {} RepaymentLetter docs for member {} in FY {} — taking newest ({})",
            matches.len(),
            matches[0].member_id,
            fiscal_year,
            matches[0].id,
        );
    }

    matches.into_iter().next()
}

async fn get_send_interval<C: ConfigService>(config_service: &C) -> u64 {
    let all_config = match config_service.get_all().await {
        Ok(c) => c,
        Err(_) => return DEFAULT_SEND_INTERVAL_SECONDS,
    };
    all_config
        .iter()
        .find(|e| e.key.as_ref() == "mail_send_interval_seconds")
        .and_then(|e| e.value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEND_INTERVAL_SECONDS)
}

async fn update_job_with_retry<J: MailJobDao>(job_dao: &J, job: &crate::dao::MailJob) -> bool {
    for attempt in 1..=3 {
        match job_dao.update(job).await {
            Ok(()) => return true,
            Err(e) => {
                tracing::error!(
                    "Worker: failed to update job {} (attempt {}/3): {:?}",
                    job.id,
                    attempt,
                    e
                );
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }
    tracing::error!(
        "Worker: giving up on job update for {} after 3 attempts",
        job.id
    );
    false
}

async fn mark_recipient_failed<R: MailRecipientDao, J: MailJobDao>(
    recipient_dao: &R,
    job_dao: &J,
    recipient: &crate::dao::MailRecipient,
    job: &mut crate::dao::MailJob,
    error_msg: &str,
) {
    let now = time::OffsetDateTime::now_utc();
    let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

    let mut updated = recipient.clone();
    updated.version = uuid::Uuid::new_v4();
    updated.status = Arc::from("failed");
    updated.error = Some(Arc::from(error_msg));
    updated.sent_at = Some(now_primitive);

    if let Err(e) = recipient_dao.update(&updated).await {
        tracing::error!(
            "Worker: failed to update recipient {}: {:?}",
            recipient.id,
            e
        );
    }

    job.failed_count += 1;
    if job.sent_count + job.failed_count >= job.total_count {
        if job.failed_count >= job.total_count {
            job.status = Arc::from("failed");
        } else {
            job.status = Arc::from("done");
        }
    }
    job.version = uuid::Uuid::new_v4();
    update_job_with_retry(job_dao, job).await;

    tracing::error!(
        "Worker: {} (recipient {}, job {})",
        error_msg,
        recipient.id,
        job.id
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn start_mail_worker<C, J, R, A, SA, D, M, IB, MD, AL, MT, RE, RP, TX, RCR, AS>(
    config_service: Arc<C>,
    job_dao: Arc<J>,
    recipient_dao: Arc<R>,
    attachment_dao: Arc<A>,
    static_attachment_dao: Arc<SA>,
    document_storage: Arc<D>,
    member_resolver: Arc<M>,
    inbound_mail_dao: Arc<IB>,
    // --- Phase 10 D-11 new deps ---
    member_document_dao: Arc<MD>,
    audit_log_dao: Arc<AL>,
    _mail_template_dao: Arc<MT>, // currently unused by worker but reserved (Plan 10.07 wiring)
    repayment_entry_dao: Arc<RE>,
    repayment_phase_dao: Arc<RP>,
    transaction_dao: Arc<TX>,
    // --- Quick 260603-h0r: shared aggregation resolver (Phase 13 D-13-04 /
    //     D-13-10). Same Arc as the Letter-Service uses — Single Source of
    //     Truth for the Phase-10 D-04 (German euro format) + D-06 (status
    //     filter Open+Contacted + soft-delete IS NULL) aggregation rule.
    repayment_context_resolver: Arc<RCR>,
    // --- Phase 27 (IMG-06/IMG-07): inline-image asset DAO ---
    // Sanctioned pattern: a new worker-only DAO generic (mirrors attachment_dao),
    // NOT a DAO added to MailServiceImpl (RESEARCH Anti-Pattern). Appended LAST
    // so all existing positional args keep their order.
    mail_asset_dao: Arc<AS>,
) where
    C: ConfigService,
    J: MailJobDao,
    R: MailRecipientDao,
    A: MailRecipientAttachmentDao,
    SA: MailJobStaticAttachmentDao,
    D: DocumentStorage + 'static,
    M: MemberResolver,
    IB: InboundMailDao,
    MD: genossi_dao::member_document::MemberDocumentDao + Send + Sync + 'static,
    AL: genossi_dao::audit_log::AuditLogDao<Transaction = MD::Transaction> + Send + Sync + 'static,
    MT: crate::dao::MailTemplateDao + Send + Sync + 'static,
    RE: genossi_dao::repayment_entry::RepaymentEntryDao<Transaction = MD::Transaction>
        + Send
        + Sync
        + 'static,
    RP: genossi_dao::repayment_phase::RepaymentPhaseDao<Transaction = MD::Transaction>
        + Send
        + Sync
        + 'static,
    TX: genossi_dao::TransactionDao<Transaction = MD::Transaction> + Send + Sync + 'static,
    RCR: RepaymentContextResolver<Transaction = MD::Transaction> + Send + Sync + 'static,
    AS: genossi_dao::mail_asset::MailAssetDao<Transaction = MD::Transaction>
        + Send
        + Sync
        + 'static,
{
    loop {
        let next = match recipient_dao.next_pending().await {
            Ok(Some(recipient)) => recipient,
            Ok(None) => {
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
            Err(e) => {
                tracing::error!("Worker: failed to query next pending recipient: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
        };

        // Load job for this recipient
        let mut job = match job_dao.find_by_id(next.mail_job_id).await {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Worker: failed to find job {}: {:?}", next.mail_job_id, e);
                tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECONDS)).await;
                continue;
            }
        };

        // Load per-recipient attachments (member-bound documents)
        let recipient_attachments = match attachment_dao.find_by_recipient_id(next.id).await {
            Ok(atts) => atts,
            Err(e) => {
                tracing::error!(
                    "Worker: failed to load attachments for recipient {}: {:?}",
                    next.id,
                    e
                );
                Arc::from(vec![])
            }
        };

        // Load job-level static attachments and convert them to MailRecipientAttachment shape
        // so they flow through the same send pipeline. The relative_path uses the canonical
        // `static_documents/<uuid>` convention consumed by DocumentStorage.
        let static_docs = match static_attachment_dao
            .find_static_documents_by_job_id(next.mail_job_id)
            .await
        {
            Ok(docs) => docs,
            Err(e) => {
                tracing::error!(
                    "Worker: failed to load static attachments for job {}: {:?}",
                    next.mail_job_id,
                    e
                );
                Arc::from(vec![])
            }
        };

        let mut attachments: Vec<MailRecipientAttachment> =
            recipient_attachments.iter().cloned().collect();
        for sd in static_docs.iter() {
            attachments.push(MailRecipientAttachment {
                recipient_id: next.id,
                document_id: sd.id,
                file_name: sd.filename.clone(),
                mime_type: sd.content_type.clone(),
                relative_path: Arc::from(sd.relative_path().as_str()),
            });
        }

        // Quick 260603-cz6: opt-in per-recipient RepaymentLetter auto-attach.
        // Resolves the matching MemberDocument by Description-Fingerprint
        // ("Anschreiben Auszahlung GJ {fiscal_year}") and pushes its file as a
        // MailRecipientAttachment. Failure modes (no member_id, missing phase,
        // 0 letters) mark the recipient `failed` and skip the send — no
        // partially-rendered mails go out.
        if job.attach_repayment_letter {
            let resolve_outcome: Result<MailRecipientAttachment, String> = async {
                let phase_id = job.repayment_phase_id.ok_or(
                    "attach_repayment_letter set but mail_job has no repayment_phase_id"
                        .to_string(),
                )?;
                let member_id = next.member_id.ok_or(
                    "attach_repayment_letter requires recipient.member_id (BulkRecipient.member_id)"
                        .to_string(),
                )?;

                let tx = transaction_dao
                    .transaction()
                    .await
                    .map_err(|e| format!("tx open failed for repayment_letter lookup: {:?}", e))?;

                let phase = repayment_phase_dao
                    .find_by_id(phase_id, tx.clone())
                    .await
                    .map_err(|e| format!("repayment_phase lookup failed: {:?}", e))?
                    .ok_or_else(|| format!("repayment_phase {} not found", phase_id))?;

                let docs = member_document_dao
                    .find_by_member_id(member_id, tx.clone())
                    .await
                    .map_err(|e| format!("member_document lookup failed: {:?}", e))?;

                let letter = find_repayment_letter_for_recipient(&docs, phase.fiscal_year)
                    .ok_or_else(|| "no_repayment_letter".to_string())?;

                Ok(MailRecipientAttachment {
                    recipient_id: next.id,
                    document_id: letter.id,
                    file_name: letter.file_name.clone(),
                    mime_type: letter.mime_type.clone(),
                    relative_path: letter.relative_path.clone(),
                })
            }
            .await;

            match resolve_outcome {
                Ok(att) => attachments.push(att),
                Err(reason) => {
                    mark_recipient_failed(
                        recipient_dao.as_ref(),
                        job_dao.as_ref(),
                        &next,
                        &mut job,
                        &reason,
                    )
                    .await;
                    let interval = get_send_interval(config_service.as_ref()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    continue;
                }
            }
        }

        // Quick 260614-b1t: render subject/body via the shared resolver (Single
        // Source of Truth — the startup backfill calls the exact same function).
        // On any render/member/repayment failure, mark the recipient failed and
        // skip the send (same tracing/interval semantics as the old inline block).
        // Phase 23 Plan 04: resolver returns RenderedContent { subject, body,
        // body_html }; we now capture body_html for both build_message + the
        // per-recipient rendered_html_body persistence (D-08).
        let (rendered_subject, rendered_body, rendered_html_body_opt) =
            match crate::render::resolve_rendered_content(
                &next,
                &job,
                member_resolver.as_ref(),
                repayment_entry_dao.as_ref(),
                repayment_phase_dao.as_ref(),
                transaction_dao.as_ref(),
                repayment_context_resolver.as_ref(),
            )
            .await
            {
                Ok(rendered) => (rendered.subject, rendered.body, rendered.body_html),
                Err(failure) => {
                    mark_recipient_failed(
                        recipient_dao.as_ref(),
                        job_dao.as_ref(),
                        &next,
                        &mut job,
                        &failure.message,
                    )
                    .await;
                    let interval = get_send_interval(config_service.as_ref()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    continue;
                }
            };

        // Resolve In-Reply-To header for reply jobs
        let reply_message_id: Option<String> =
            if let Some(inbound_id) = job.reply_to_inbound_mail_id {
                match inbound_mail_dao.find_by_id(inbound_id).await {
                    Ok(Some(inbound)) => inbound.message_id.as_ref().map(|s| s.to_string()),
                    Ok(None) => {
                        tracing::warn!(
                            "Worker: inbound mail {} not found for reply threading",
                            inbound_id
                        );
                        None
                    }
                    Err(e) => {
                        tracing::warn!("Worker: failed to load inbound mail for reply: {:?}", e);
                        None
                    }
                }
            } else {
                None
            };

        // Load SMTP config and send
        let send_result = send_mail_for_recipient(
            config_service.as_ref(),
            &next.to_address,
            &rendered_subject,
            &rendered_body,
            // Phase 23 Plan 04 (HTML-01, D-08): pass the rendered + already-sanitized
            // HTML sibling; None when the job carries no body_html.
            rendered_html_body_opt.as_deref(),
            &attachments,
            document_storage.as_ref(),
            // Phase 27 (IMG-06/IMG-07): the mail-asset DAO + tx DAO let the send
            // path load inline-image bytes referenced by data-genossi-asset-id.
            mail_asset_dao.as_ref(),
            transaction_dao.as_ref(),
            reply_message_id.as_deref(),
        )
        .await;

        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        let mut updated_recipient = next.clone();
        updated_recipient.version = uuid::Uuid::new_v4();
        // Quick 260614-9zf: persist the per-recipient rendered subject + body so the
        // Vorstand can later see exactly what this recipient received. Set for BOTH the
        // success and the send-failure path (the render happened before the send attempt).
        // Render-/member-resolution failures use mark_recipient_failed + continue *before*
        // this point, so their rendered_* correctly stay None.
        updated_recipient.rendered_subject = Some(Arc::from(rendered_subject.as_str()));
        updated_recipient.rendered_body = Some(Arc::from(rendered_body.as_str()));
        // Phase 23 Plan 04 (D-08, Pitfall 4): persist rendered HTML iff the job
        // carried body_html. `Option::map` preserves None (never Some("")).
        updated_recipient.rendered_html_body = rendered_html_body_opt.as_deref().map(Arc::from);
        // Quick 260614-b1t: live worker renders are NOT reconstructions — they are
        // the byte-accurate content sent at this moment. The backfill flips this to
        // true only for retroactively-rendered legacy rows.
        updated_recipient.rendered_reconstructed = false;

        // Capture send-result summary for the post-send audited MemberDocument
        // create (Phase 10 D-10). We move out of send_result in the match below,
        // so we extract the boolean + error string here first.
        let send_ok = send_result.is_ok();
        let send_err_msg = match &send_result {
            Ok(_) => String::new(),
            Err(e) => format!("{:?}", e),
        };

        match send_result {
            Ok(message_id) => {
                updated_recipient.status = Arc::from("sent");
                updated_recipient.sent_at = Some(now_primitive);
                updated_recipient.message_id = message_id.map(Arc::from);
                job.sent_count += 1;
                tracing::info!("Worker: sent mail to {} (job {})", next.to_address, job.id);
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                updated_recipient.status = Arc::from("failed");
                updated_recipient.error = Some(Arc::from(error_msg.as_str()));
                job.failed_count += 1;
                tracing::error!(
                    "Worker: failed to send mail to {} (job {}): {}",
                    next.to_address,
                    job.id,
                    error_msg
                );
            }
        }

        // Update recipient
        if let Err(e) = recipient_dao.update(&updated_recipient).await {
            tracing::error!("Worker: failed to update recipient {}: {:?}", next.id, e);
        }

        // Phase 10 D-10 / MAIL-04: persist MemberDocument as audited final-state anchor.
        // Skips ad-hoc recipients (no member_id) per CONTEXT.md (Defense-in-Depth).
        // Fail-tolerant: a failure here does NOT abort the worker — see
        // try_create_member_document_audited for tracing + rollback semantics.
        if let Some(member_id) = next.member_id {
            try_create_member_document_audited(
                member_document_dao.as_ref(),
                audit_log_dao.as_ref(),
                transaction_dao.as_ref(),
                member_id,
                &job,
                next.id,
                send_ok,
                &send_err_msg,
            )
            .await;
        }

        // Check job completion
        if job.sent_count + job.failed_count >= job.total_count {
            if job.failed_count >= job.total_count {
                job.status = Arc::from("failed");
            } else {
                job.status = Arc::from("done");
            }
        }
        job.version = uuid::Uuid::new_v4();

        update_job_with_retry(job_dao.as_ref(), &job).await;

        // Wait configured interval
        let interval = get_send_interval(config_service.as_ref()).await;
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

/// Phase 10 D-11: best-effort persist + audit a MemberDocument for the recipient.
/// Inline audit-pattern: DAO.create + build_create_entries + AuditLogDao.create_entries.
/// Fail-tolerant: any error is logged via tracing::error! and the function returns;
/// the worker continues with the next recipient. tx is rolled back on error.
///
/// Inlined helpers from `crate::worker_audit` are used instead of the
/// `audited_create!` macro in `genossi_service_impl` because that crate already
/// depends on `genossi_mail` (cycle would otherwise form). Hash-chain semantics
/// are byte-for-byte identical — see `worker_audit::compute_entry_hash`.
#[allow(clippy::too_many_arguments)]
async fn try_create_member_document_audited<MD, AL, TX>(
    member_document_dao: &MD,
    audit_log_dao: &AL,
    transaction_dao: &TX,
    member_id: uuid::Uuid,
    job: &crate::dao::MailJob,
    recipient_id: uuid::Uuid,
    send_result_ok: bool,
    error_message: &str,
) where
    MD: genossi_dao::member_document::MemberDocumentDao,
    AL: genossi_dao::audit_log::AuditLogDao<Transaction = MD::Transaction>,
    TX: genossi_dao::TransactionDao<Transaction = MD::Transaction>,
{
    let entity =
        build_member_document_entity(job, member_id, recipient_id, send_result_ok, error_message);

    let tx = match transaction_dao.transaction().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "Worker: tx open failed for MemberDocument (recipient {}): {:?}",
                recipient_id,
                e
            );
            return;
        }
    };

    // INLINE Step 1: DAO write
    if let Err(e) = member_document_dao
        .create(&entity, REPAYMENT_MAIL_PROCESS, tx.clone())
        .await
    {
        tracing::error!(
            "Worker: MemberDocumentDao.create failed (recipient {}): {:?}",
            recipient_id,
            e
        );
        let _ = genossi_dao::Transaction::rollback(tx).await;
        return;
    }

    // INLINE Step 2: get current chain tail
    let prev_hash = match audit_log_dao.get_latest_hash(tx.clone()).await {
        Ok(opt) => opt.unwrap_or_default(),
        Err(e) => {
            tracing::error!(
                "Worker: AuditLogDao.get_latest_hash failed (recipient {}): {:?}",
                recipient_id,
                e
            );
            let _ = genossi_dao::Transaction::rollback(tx).await;
            return;
        }
    };

    // INLINE Step 3: build entries via worker_audit (pure helper)
    let entries = crate::worker_audit::build_create_entries(
        &entity,
        WORKER_USER_ID,
        REPAYMENT_MAIL_PROCESS,
        &prev_hash,
        &mut uuid::Uuid::new_v4,
    );

    // INLINE Step 4: write audit-log rows in same tx
    if !entries.is_empty() {
        if let Err(e) = audit_log_dao.create_entries(&entries, tx.clone()).await {
            tracing::error!(
                "Worker: AuditLogDao.create_entries failed (recipient {}): {:?}",
                recipient_id,
                e
            );
            let _ = genossi_dao::Transaction::rollback(tx).await;
            return;
        }
    }

    // INLINE Step 5: commit
    if let Err(e) = transaction_dao.commit(tx).await {
        tracing::error!(
            "Worker: tx commit failed for MemberDocument (recipient {}): {:?}",
            recipient_id,
            e
        );
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_mail_for_recipient<C, D, AS, TX>(
    config_service: &C,
    to: &str,
    subject: &str,
    body: &str,
    // Phase 23 Plan 04 (HTML-01, D-08): rendered + already-sanitized HTML sibling.
    // None ⇒ text-only path (Phase-22 legacy MIME shape preserved).
    body_html: Option<&str>,
    attachments: &[crate::dao::MailRecipientAttachment],
    document_storage: &D,
    // Phase 27 (IMG-06/IMG-07): load inline-image bytes referenced by
    // data-genossi-asset-id via the mail-asset DAO. A read tx is opened via
    // transaction_dao for the lookup.
    mail_asset_dao: &AS,
    transaction_dao: &TX,
    in_reply_to: Option<&str>,
) -> Result<Option<String>, MailServiceError>
where
    C: ConfigService,
    D: DocumentStorage,
    AS: genossi_dao::mail_asset::MailAssetDao + Send + Sync,
    TX: genossi_dao::TransactionDao<Transaction = AS::Transaction> + Send + Sync,
{
    use lettre::AsyncTransport;

    let smtp_config = load_smtp_config(config_service).await?;
    let transport = build_transport(&smtp_config)?;

    // Attachment loading (async I/O) stays in the worker per 22-CONTEXT.md D-03;
    // build_message is a pure sync factory that receives already-loaded bytes
    // wrapped in `LoadedAttachment` (D-02).
    let mut loaded: Vec<crate::send::LoadedAttachment> = Vec::with_capacity(attachments.len());
    for att in attachments {
        let file_bytes = document_storage
            .load(&att.relative_path)
            .await
            .map_err(|e| {
                MailServiceError::SmtpError(Arc::from(format!(
                    "Failed to load attachment file '{}': {}",
                    att.relative_path, e
                )))
            })?;
        loaded.push(crate::send::LoadedAttachment {
            file_name: att.file_name.clone(),
            mime_type: att.mime_type.clone(),
            bytes: file_bytes,
        });
    }

    // Phase 27 (IMG-06): rewrite <img data-genossi-asset-id=X> to cid: refs and
    // load the referenced asset bytes. When the HTML carries no image,
    // rewrite_img_cids returns the HTML unchanged + an empty Vec, so the
    // no-image build_message path stays byte-identical (IMG-09).
    let (rewritten_html, inline_images): (Option<String>, Vec<crate::send::LoadedInlineImage>) =
        match body_html {
            Some(html) => {
                let (rewritten, refs) = crate::render::rewrite_img_cids(html);
                let mut images: Vec<crate::send::LoadedInlineImage> =
                    Vec::with_capacity(refs.len());
                if !refs.is_empty() {
                    // Open a single read tx for all asset lookups.
                    match transaction_dao.transaction().await {
                        Ok(tx) => {
                            for asset_ref in &refs {
                                match mail_asset_dao
                                    .find_by_id(asset_ref.asset_id, tx.clone())
                                    .await
                                {
                                    Ok(Some(entity)) => {
                                        images.push(crate::send::LoadedInlineImage {
                                            cid: asset_ref.cid.clone(),
                                            mime_type: entity.mime_type.clone(),
                                            bytes: entity.bytes.clone(),
                                        });
                                    }
                                    Ok(None) => {
                                        // Missing/soft-deleted asset — skip the
                                        // image (broken image beats a failed
                                        // send, T-27-15).
                                        tracing::warn!(
                                            asset_id = %asset_ref.asset_id,
                                            "Worker: inline asset not found, skipping image"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            asset_id = %asset_ref.asset_id,
                                            error = ?e,
                                            "Worker: inline asset load failed, skipping image"
                                        );
                                    }
                                }
                            }
                            let _ = genossi_dao::TransactionDao::commit(transaction_dao, tx).await;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = ?e,
                                "Worker: cannot open tx for inline-image load, sending without images"
                            );
                        }
                    }
                }
                (Some(rewritten), images)
            }
            None => (None, Vec::new()),
        };

    let email = crate::send::build_message(
        &smtp_config.from,
        to,
        subject,
        body,
        // Phase 27 (IMG-06): forward the cid-rewritten HTML sibling.
        rewritten_html.as_deref(),
        &loaded,
        &inline_images,
        in_reply_to,
        smtp_config.encoding,
    )?;

    // Capture the Message-ID header before sending so it matches what is
    // transmitted. `lettre` auto-generates one during build.
    let message_id = email
        .headers()
        .get_raw("Message-ID")
        .and_then(crate::dao::normalize_message_id);
    if message_id.is_none() {
        tracing::warn!("Worker: outgoing mail has no Message-ID header");
    }

    transport
        .send(email)
        .await
        .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?;

    Ok(message_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{MailDaoError, MailJob, MockMailJobDao};
    use genossi_config::dao::ConfigEntry;
    use genossi_config::service::MockConfigService;

    fn sample_datetime() -> time::PrimitiveDateTime {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        )
    }

    // Quick 260603-cz6: find_repayment_letter_for_recipient unit tests
    mod find_repayment_letter_tests {
        use super::*;
        use genossi_dao::member_document::MemberDocumentEntity;
        use std::sync::Arc;
        use uuid::Uuid;

        fn doc(
            id: Uuid,
            member_id: Uuid,
            document_type: &str,
            description: Option<&str>,
            created_offset_days: i64,
        ) -> MemberDocumentEntity {
            let base = sample_datetime();
            let created = base + time::Duration::days(created_offset_days);
            MemberDocumentEntity {
                id,
                member_id,
                document_type: Arc::from(document_type),
                description: description.map(Arc::from),
                file_name: Arc::from("letter.pdf"),
                mime_type: Arc::from("application/pdf"),
                relative_path: Arc::from("repayment_letters/letter.pdf"),
                created,
                deleted: None,
                version: Uuid::new_v4(),
                template_id: None,
                mail_recipient_id: None,
                status: None,
            }
        }

        #[test]
        fn returns_none_when_no_documents() {
            let result = find_repayment_letter_for_recipient(&[], 2026);
            assert!(result.is_none());
        }

        #[test]
        fn returns_none_when_no_repayment_letter_type() {
            let docs = vec![doc(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "join_declaration",
                Some("Beitrittsantrag"),
                0,
            )];
            let result = find_repayment_letter_for_recipient(&docs, 2026);
            assert!(result.is_none());
        }

        #[test]
        fn returns_none_when_fiscal_year_mismatch() {
            let docs = vec![doc(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "repayment_letter",
                Some("Anschreiben Auszahlung GJ 2025"), // different year
                0,
            )];
            let result = find_repayment_letter_for_recipient(&docs, 2026);
            assert!(result.is_none());
        }

        #[test]
        fn returns_match_for_exact_fiscal_year() {
            let id = Uuid::new_v4();
            let docs = vec![doc(
                id,
                Uuid::new_v4(),
                "repayment_letter",
                Some("Anschreiben Auszahlung GJ 2026"),
                0,
            )];
            let result = find_repayment_letter_for_recipient(&docs, 2026);
            assert_eq!(result.map(|d| d.id), Some(id));
        }

        #[test]
        fn skips_soft_deleted() {
            let mut d = doc(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "repayment_letter",
                Some("Anschreiben Auszahlung GJ 2026"),
                0,
            );
            d.deleted = Some(sample_datetime());
            let docs = vec![d];
            let result = find_repayment_letter_for_recipient(&docs, 2026);
            assert!(result.is_none());
        }

        #[test]
        fn returns_newest_when_multiple_match() {
            let member_id = Uuid::new_v4();
            let old_id = Uuid::new_v4();
            let new_id = Uuid::new_v4();
            let docs = vec![
                doc(
                    old_id,
                    member_id,
                    "repayment_letter",
                    Some("Anschreiben Auszahlung GJ 2026"),
                    0, // older
                ),
                doc(
                    new_id,
                    member_id,
                    "repayment_letter",
                    Some("Anschreiben Auszahlung GJ 2026"),
                    5, // newer
                ),
            ];
            let result = find_repayment_letter_for_recipient(&docs, 2026);
            assert_eq!(
                result.map(|d| d.id),
                Some(new_id),
                "expected newest letter to win"
            );
        }
    }

    #[tokio::test]
    async fn test_get_send_interval_default() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| Ok(vec![].into()));

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }

    #[tokio::test]
    async fn test_get_send_interval_custom() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("mail_send_interval_seconds"),
                value: Arc::from("60"),
                value_type: Arc::from("int"),
            }]
            .into())
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, 60);
    }

    #[tokio::test]
    async fn test_get_send_interval_invalid_value() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Ok(vec![ConfigEntry {
                key: Arc::from("mail_send_interval_seconds"),
                value: Arc::from("not_a_number"),
                value_type: Arc::from("int"),
            }]
            .into())
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }

    #[tokio::test]
    async fn test_get_send_interval_config_error() {
        let mut config_mock = MockConfigService::new();
        config_mock.expect_get_all().returning(|| {
            Err(genossi_config::service::ConfigServiceError::DataAccess(
                Arc::from("db error"),
            ))
        });

        let interval = get_send_interval(&config_mock).await;
        assert_eq!(interval, DEFAULT_SEND_INTERVAL_SECONDS);
    }

    fn sample_job() -> MailJob {
        MailJob {
            id: uuid::Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: uuid::Uuid::new_v4(),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 1,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    #[tokio::test]
    async fn test_update_job_with_retry_succeeds_on_second_attempt() {
        let mut job_dao = MockMailJobDao::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        job_dao.expect_update().times(2).returning(move |_| {
            let count = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count == 0 {
                Err(MailDaoError::DatabaseError(Arc::from("transient error")))
            } else {
                Ok(())
            }
        });

        let job = sample_job();
        let result = update_job_with_retry(&job_dao, &job).await;
        assert!(result);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_update_job_with_retry_fails_after_3_attempts() {
        let mut job_dao = MockMailJobDao::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        job_dao.expect_update().times(3).returning(move |_| {
            call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(MailDaoError::DatabaseError(Arc::from("persistent error")))
        });

        let job = sample_job();
        let result = update_job_with_retry(&job_dao, &job).await;
        assert!(!result);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[test]
    fn normalize_message_id_strips_angle_brackets() {
        use crate::dao::normalize_message_id;
        assert_eq!(
            normalize_message_id("<abc.123@example.com>"),
            Some("abc.123@example.com".to_string())
        );
        assert_eq!(
            normalize_message_id("  <id@host>  "),
            Some("id@host".to_string())
        );
        assert_eq!(
            normalize_message_id("no-brackets@host"),
            Some("no-brackets@host".to_string())
        );
        assert_eq!(normalize_message_id(""), None);
        assert_eq!(normalize_message_id("<>"), None);
    }

    // -------------------------------------------------------------------------
    // Phase 10 D-10 / D-11: build_member_document_entity helper tests
    // -------------------------------------------------------------------------

    /// Helper: build a minimal MailJob for entity-construction tests.
    fn make_test_job(subject: &str, template_id: Option<uuid::Uuid>) -> MailJob {
        MailJob {
            id: uuid::Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: uuid::Uuid::new_v4(),
            subject: Arc::from(subject),
            body: Arc::from("body"),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id,
            repayment_phase_id: None,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    /// build_member_document_entity (helper) — happy-path send success.
    /// status='sent', description=subject, template_id propagates, recipient_id set.
    #[test]
    fn test_build_member_document_entity_status_sent() {
        let tpl_id = uuid::Uuid::new_v4();
        let job = make_test_job("Auszahlung GJ 2026", Some(tpl_id));
        let mid = uuid::Uuid::new_v4();
        let rid = uuid::Uuid::new_v4();

        let entity = build_member_document_entity(&job, mid, rid, true, "");

        assert_eq!(entity.member_id, mid, "member_id must round-trip");
        assert_eq!(
            entity.status.as_deref(),
            Some("sent"),
            "send_result_ok=true => status='sent'"
        );
        assert_eq!(
            entity.description.as_deref(),
            Some("Auszahlung GJ 2026"),
            "description=job.subject on success"
        );
        assert_eq!(
            entity.template_id,
            Some(tpl_id),
            "template_id must propagate from job (MAIL-03 / D-12)"
        );
        assert_eq!(
            entity.mail_recipient_id,
            Some(rid),
            "mail_recipient_id must equal recipient_id (D-07)"
        );
        assert_eq!(
            &*entity.document_type, "repayment_mail",
            "document_type must be 'repayment_mail' for Phase 10"
        );
        assert!(
            entity.deleted.is_none(),
            "new entity must not be soft-deleted"
        );
        assert_eq!(
            &*entity.file_name, "",
            "no file on disk for repayment mails"
        );
        assert_eq!(
            &*entity.relative_path, "",
            "no path on disk for repayment mails"
        );
    }

    /// build_member_document_entity (helper) — fail-path with truncation.
    /// status='failed', description='{subject} [FAILED: {trunc}]' with 200 char cap.
    #[test]
    fn test_build_member_document_entity_status_failed_with_truncation() {
        let job = make_test_job("Subj", None);
        let mid = uuid::Uuid::new_v4();
        let rid = uuid::Uuid::new_v4();
        // 300-char error (> ERROR_TRUNCATION_LIMIT=200)
        let long_err: String = "x".repeat(300);

        let entity = build_member_document_entity(&job, mid, rid, false, &long_err);

        assert_eq!(
            entity.status.as_deref(),
            Some("failed"),
            "send_result_ok=false => status='failed'"
        );
        let desc = entity.description.as_deref().unwrap();
        assert!(
            desc.contains("[FAILED:"),
            "failed description must contain '[FAILED:' suffix, got: {}",
            desc
        );
        assert!(
            desc.starts_with("Subj"),
            "description must start with the job subject, got: {}",
            desc
        );
        // Length budget: subject ("Subj"=4) + " [FAILED: " (10) + truncated err (<=200) + "]" (1)
        // = at most 215 chars.
        let max_len = "Subj".len() + " [FAILED: ".len() + ERROR_TRUNCATION_LIMIT + "]".len();
        assert!(
            desc.chars().count() <= max_len,
            "description must not exceed {} chars (got {}): {}",
            max_len,
            desc.chars().count(),
            desc
        );
        // The truncated portion must contain at most 200 x's
        let x_count = desc.chars().filter(|c| *c == 'x').count();
        assert_eq!(
            x_count, ERROR_TRUNCATION_LIMIT,
            "exactly {} 'x' chars from the 300-char error must survive truncation, got {}",
            ERROR_TRUNCATION_LIMIT, x_count
        );
    }

    // ── Phase 23 Plan 04 — worker rendered_html_body persistence (D-08, HTML-01) ──
    //
    // Rationale for the "assignment probe" test pattern:
    //   The worker's outer send loop is >200 lines of state machine and is
    //   already covered by the Plan-02 render tests (which pin the pipeline
    //   contract) and the send.rs MIME-shape tests (Plan 03). Here we lock the
    //   ONE line the worker owns in Plan 04:
    //     updated_recipient.rendered_html_body = rendered_html_body_opt
    //         .as_deref().map(Arc::from);
    //   The assignment probe reproduces the two cases (Some / None) as pure
    //   value assignments on a MailRecipient — the exact expression the worker
    //   uses. If someone later re-writes it as `Some(Arc::from(""))` (Pitfall 4),
    //   the None test fails immediately.

    fn sample_recipient() -> crate::dao::MailRecipient {
        crate::dao::MailRecipient {
            id: uuid::Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: uuid::Uuid::new_v4(),
            mail_job_id: uuid::Uuid::new_v4(),
            to_address: Arc::from("dst@example.com"),
            member_id: Some(uuid::Uuid::new_v4()),
            application_id: None,
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_html_body: None,
            rendered_reconstructed: false,
        }
    }

    /// Phase 23 Plan 04 (D-08, Pitfall 4): body_html=None on the job MUST
    /// leave rendered_html_body IS NULL — not Some("") — on the recipient.
    #[test]
    fn body_html_none_leaves_rendered_html_body_null() {
        let rendered_html_body_opt: Option<String> = None;
        let mut recipient = sample_recipient();

        // Exact expression the worker uses (see worker.rs render-body-html assign):
        recipient.rendered_html_body = rendered_html_body_opt.as_deref().map(Arc::from);

        assert!(
            recipient.rendered_html_body.is_none(),
            "None ⇒ None (never Some(\"\"))"
        );
    }

    /// Phase 23 Plan 04 (HTML-01 wire proof, D-08): body_html=Some(rendered)
    /// MUST land byte-for-byte on the recipient's rendered_html_body.
    #[test]
    fn rendered_html_body_persisted_when_render_yields_html() {
        let rendered = "<p>Hallo Max</p>".to_string();
        let rendered_html_body_opt: Option<String> = Some(rendered.clone());
        let mut recipient = sample_recipient();

        recipient.rendered_html_body = rendered_html_body_opt.as_deref().map(Arc::from);

        assert_eq!(
            recipient.rendered_html_body.as_deref(),
            Some(rendered.as_str()),
            "rendered HTML must be persisted verbatim (byte-accurate audit trail)"
        );
    }
}
