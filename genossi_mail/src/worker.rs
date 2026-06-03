use std::sync::Arc;

use crate::dao::{
    InboundMailDao, MailJobDao, MailJobStaticAttachmentDao, MailRecipientAttachment,
    MailRecipientAttachmentDao, MailRecipientDao,
};
use crate::service::{build_transport, load_smtp_config, MailServiceError};
use crate::template::{
    member_to_template_context, merge_repayment_context, render_template, MemberResolver,
};
use genossi_config::service::ConfigService;
use genossi_service::document_storage::DocumentStorage;
use genossi_service::member_document::DocumentType;
// Quick 260603-h0r: Phase 10 D-04 / D-06 aggregation delegated to the shared
// resolver (Single Source of Truth with the Letter-Service — Phase 13 D-13-04 /
// D-13-10). Replaces the inline filter+sum+German-format block that used to
// live in start_mail_worker().
use genossi_service::repayment_context::RepaymentContextResolver;
use genossi_service::ServiceError;

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
pub async fn start_mail_worker<C, J, R, A, SA, D, M, IB, MD, AL, MT, RE, RP, TX, RCR>(
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

        // Render template subject/body if recipient has a member_id
        let (rendered_subject, rendered_body) = if let Some(member_id) = next.member_id {
            match member_resolver.find_member_by_id(member_id).await {
                Ok(Some(member)) => {
                    let mut ctx = member_to_template_context(&member);

                    // Phase 10 D-04: merge per-recipient repayment context (only if
                    // job is repayment-linked). D-06 filter: deleted IS NULL AND
                    // status IN (Open, Contacted). D-05: only merge when at least
                    // one relevant entry exists; otherwise the strict-env render
                    // will fail on referenced `payout_amount`/`share_count`/
                    // `fiscal_year` variables and the recipient is marked failed.
                    if let Some(phase_id) = job.repayment_phase_id {
                        let agg_tx = match transaction_dao.transaction().await {
                            Ok(t) => t,
                            Err(e) => {
                                mark_recipient_failed(
                                    recipient_dao.as_ref(),
                                    job_dao.as_ref(),
                                    &next,
                                    &mut job,
                                    &format!(
                                        "Worker: cannot open tx for repayment context: {:?}",
                                        e
                                    ),
                                )
                                .await;
                                let interval = get_send_interval(config_service.as_ref()).await;
                                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                                continue;
                            }
                        };

                        let phase_opt = match repayment_phase_dao
                            .find_by_id(phase_id, agg_tx.clone())
                            .await
                        {
                            Ok(p) => p,
                            Err(e) => {
                                mark_recipient_failed(
                                    recipient_dao.as_ref(),
                                    job_dao.as_ref(),
                                    &next,
                                    &mut job,
                                    &format!("Worker: repayment_phase lookup failed: {:?}", e),
                                )
                                .await;
                                let interval = get_send_interval(config_service.as_ref()).await;
                                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                                continue;
                            }
                        };

                        if let Some(phase) = phase_opt {
                            let entries = match repayment_entry_dao
                                .find_by_phase_id(phase_id, agg_tx.clone())
                                .await
                            {
                                Ok(es) => es,
                                Err(e) => {
                                    mark_recipient_failed(
                                        recipient_dao.as_ref(),
                                        job_dao.as_ref(),
                                        &next,
                                        &mut job,
                                        &format!("Worker: repayment_entry lookup failed: {:?}", e),
                                    )
                                    .await;
                                    let interval = get_send_interval(config_service.as_ref()).await;
                                    tokio::time::sleep(std::time::Duration::from_secs(interval))
                                        .await;
                                    continue;
                                }
                            };

                            // Quick 260603-h0r: Phase 10 D-04 / D-06 aggregation delegated
                            // to RepaymentContextResolver (Phase 13 D-13-04 / D-13-10 — Single
                            // Source of Truth shared with RepaymentLetterServiceImpl). The
                            // resolver returns share_count + German-locale payout_amount +
                            // fiscal_year; share_value_str is the phase-wide Anteilswert and
                            // is derived from phase.share_value locally because the resolver's
                            // RepaymentContext does not carry it (Quick 260602-r2i kept the
                            // share_value variable in the merged template context; resolver
                            // was scoped to per-recipient aggregation only).
                            let share_value_str = format!(
                                "{},{:02}",
                                phase.share_value / 100,
                                phase.share_value % 100,
                            );
                            match repayment_context_resolver.aggregate(
                                &phase,
                                &entries,
                                member.id,
                            ) {
                                Ok(rc) => {
                                    // D-05: at least one relevant entry -> merge full context.
                                    ctx = merge_repayment_context(
                                        ctx,
                                        &rc.payout_amount,
                                        rc.share_count,
                                        &share_value_str,
                                        rc.fiscal_year,
                                    );
                                }
                                Err(ServiceError::EntityNotFound(_)) => {
                                    // D-05 edge-case: no Open/Contacted entries for this member.
                                    // Skip merge — strict-env template render will fail on any
                                    // referenced payout_amount/share_count/fiscal_year var
                                    // without `{% if X is defined %}`-guard, which triggers
                                    // mark_recipient_failed downstream. This preserves the
                                    // pre-refactor behavior byte-for-byte (the old inline
                                    // `if !relevant.is_empty()` block was a no-op when empty).
                                }
                                Err(e) => {
                                    mark_recipient_failed(
                                        recipient_dao.as_ref(),
                                        job_dao.as_ref(),
                                        &next,
                                        &mut job,
                                        &format!(
                                            "Worker: repayment_context aggregate failed: {:?}",
                                            e
                                        ),
                                    )
                                    .await;
                                    let interval =
                                        get_send_interval(config_service.as_ref()).await;
                                    tokio::time::sleep(std::time::Duration::from_secs(interval))
                                        .await;
                                    continue;
                                }
                            }
                        }

                        // Release the read tx — best-effort, errors ignored as the tx
                        // was read-only.
                        let _ = transaction_dao.commit(agg_tx).await;
                    }

                    let subject = match render_template(&job.subject, &ctx) {
                        Ok(s) => s,
                        Err(e) => {
                            mark_recipient_failed(
                                recipient_dao.as_ref(),
                                job_dao.as_ref(),
                                &next,
                                &mut job,
                                &format!("Template render error (subject): {}", e.message),
                            )
                            .await;
                            let interval = get_send_interval(config_service.as_ref()).await;
                            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                            continue;
                        }
                    };
                    let body = match render_template(&job.body, &ctx) {
                        Ok(s) => s,
                        Err(e) => {
                            mark_recipient_failed(
                                recipient_dao.as_ref(),
                                job_dao.as_ref(),
                                &next,
                                &mut job,
                                &format!("Template render error (body): {}", e.message),
                            )
                            .await;
                            let interval = get_send_interval(config_service.as_ref()).await;
                            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                            continue;
                        }
                    };
                    (subject, body)
                }
                Ok(None) => {
                    mark_recipient_failed(
                        recipient_dao.as_ref(),
                        job_dao.as_ref(),
                        &next,
                        &mut job,
                        &format!("Member {} not found for template rendering", member_id),
                    )
                    .await;
                    let interval = get_send_interval(config_service.as_ref()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    continue;
                }
                Err(e) => {
                    mark_recipient_failed(
                        recipient_dao.as_ref(),
                        job_dao.as_ref(),
                        &next,
                        &mut job,
                        &format!("Failed to load member for template rendering: {:?}", e),
                    )
                    .await;
                    let interval = get_send_interval(config_service.as_ref()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    continue;
                }
            }
        } else {
            // No member_id — plain text passthrough
            (job.subject.to_string(), job.body.to_string())
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
            &attachments,
            document_storage.as_ref(),
            reply_message_id.as_deref(),
        )
        .await;

        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        let mut updated_recipient = next.clone();
        updated_recipient.version = uuid::Uuid::new_v4();

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

async fn send_mail_for_recipient<C: ConfigService, D: DocumentStorage>(
    config_service: &C,
    to: &str,
    subject: &str,
    body: &str,
    attachments: &[crate::dao::MailRecipientAttachment],
    document_storage: &D,
    in_reply_to: Option<&str>,
) -> Result<Option<String>, MailServiceError> {
    use lettre::message::{Attachment, MultiPart, SinglePart};
    use lettre::{AsyncTransport, Message};

    let smtp_config = load_smtp_config(config_service).await?;
    let transport = build_transport(&smtp_config)?;

    let from = smtp_config
        .from
        .parse()
        .map_err(|e: lettre::address::AddressError| {
            MailServiceError::SmtpError(Arc::from(format!("Invalid from address: {}", e)))
        })?;
    let to_addr = to.parse().map_err(|e: lettre::address::AddressError| {
        MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
    })?;

    // Build the text body via SinglePart::plain in both paths so that
    // Content-Type: text/plain; charset=utf-8 is always set. Without this,
    // MessageBuilder::body() emits text/plain without a charset parameter,
    // which causes clients like GMX Android to mis-decode umlauts.
    let text_part = SinglePart::plain(body.to_string());

    let mut builder = Message::builder()
        .from(from)
        .to(to_addr)
        .subject(subject)
        .message_id(None);

    if let Some(ref_id) = in_reply_to {
        let bracketed = format!("<{}>", ref_id);
        builder = builder.in_reply_to(bracketed.clone()).references(bracketed);
    }

    let email = if attachments.is_empty() {
        builder
            .singlepart(text_part)
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?
    } else {
        // Multipart mail with attachments
        let mut multipart = MultiPart::mixed().singlepart(text_part);

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

            let content_type = lettre::message::header::ContentType::parse(&att.mime_type)
                .unwrap_or(
                    lettre::message::header::ContentType::parse("application/octet-stream")
                        .unwrap(),
                );

            let attachment =
                Attachment::new(att.file_name.to_string()).body(file_bytes, content_type);
            multipart = multipart.singlepart(attachment);
        }

        builder
            .multipart(multipart)
            .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))?
    };

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

    /// Build a mail without attachments and verify that the serialized bytes
    /// carry `charset=utf-8` and the umlauts survive the round-trip.
    ///
    /// This mirrors the exact Message-building pattern used in
    /// `send_mail_for_recipient` (no-attachments branch), so we don't need a
    /// real SMTP transport.
    #[test]
    fn plain_mail_body_has_utf8_charset() {
        use lettre::message::SinglePart;
        use lettre::Message;

        let body = "Hallo Jürgen, schöne Grüße! ä ö ü ß";
        let text_part = SinglePart::plain(body.to_string());

        let email = Message::builder()
            .from("sender@example.com".parse().unwrap())
            .to("recipient@example.com".parse().unwrap())
            .subject("Test")
            .singlepart(text_part)
            .expect("build plain mail");

        let formatted = email.formatted();
        let text = String::from_utf8_lossy(&formatted);

        assert!(
            text.contains("charset=utf-8"),
            "plain mail must declare charset=utf-8, got:\n{}",
            text
        );
        // A transfer encoding must be declared so non-ASCII bytes survive SMTP.
        assert!(
            text.contains("Content-Transfer-Encoding: quoted-printable")
                || text.contains("Content-Transfer-Encoding: base64"),
            "plain mail must declare a non-7bit transfer encoding, got:\n{}",
            text
        );
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

    /// Building a lettre Message auto-generates a Message-ID header, which we
    /// must be able to read back before sending so we can persist it.
    #[test]
    fn built_message_exposes_message_id_header() {
        use lettre::message::SinglePart;
        use lettre::Message;

        let email = Message::builder()
            .from("sender@example.com".parse().unwrap())
            .to("recipient@example.com".parse().unwrap())
            .subject("Test")
            .message_id(None)
            .singlepart(SinglePart::plain("hi".to_string()))
            .expect("build mail");

        let raw = email
            .headers()
            .get_raw("Message-ID")
            .expect("lettre should set a Message-ID");
        let normalized = crate::dao::normalize_message_id(raw).expect("normalized Message-ID");
        assert!(
            !normalized.contains('<') && !normalized.contains('>'),
            "normalized Message-ID must not contain angle brackets: {normalized}"
        );
        assert!(
            normalized.contains('@'),
            "Message-ID should have an at sign: {normalized}"
        );
    }

    /// Build a mail with an attachment part and verify the text body still
    /// declares charset=utf-8. This guards the multipart branch against
    /// regressions.
    #[test]
    fn multipart_mail_body_has_utf8_charset() {
        use lettre::message::header::ContentType;
        use lettre::message::{Attachment, MultiPart, SinglePart};
        use lettre::Message;

        let body = "Anbei die Bescheinigung für Herrn Müller.";
        let text_part = SinglePart::plain(body.to_string());

        let attachment = Attachment::new("test.pdf".to_string()).body(
            b"%PDF-fake".to_vec(),
            ContentType::parse("application/pdf").unwrap(),
        );

        let multipart = MultiPart::mixed()
            .singlepart(text_part)
            .singlepart(attachment);

        let email = Message::builder()
            .from("sender@example.com".parse().unwrap())
            .to("recipient@example.com".parse().unwrap())
            .subject("Test")
            .multipart(multipart)
            .expect("build multipart mail");

        let formatted = email.formatted();
        let text = String::from_utf8_lossy(&formatted);

        assert!(
            text.contains("charset=utf-8"),
            "multipart text part must declare charset=utf-8, got:\n{}",
            text
        );
        assert!(
            text.contains("Content-Transfer-Encoding: quoted-printable")
                || text.contains("Content-Transfer-Encoding: base64"),
            "text part must declare a non-7bit transfer encoding, got:\n{}",
            text
        );
    }

    /// Verify that building a reply mail includes In-Reply-To and References headers.
    #[test]
    fn reply_mail_includes_in_reply_to_header() {
        use lettre::message::SinglePart;
        use lettre::Message;

        let ref_id = "abc.123@example.com";
        let bracketed = format!("<{}>", ref_id);

        let email = Message::builder()
            .from("sender@example.com".parse().unwrap())
            .to("recipient@example.com".parse().unwrap())
            .subject("Re: Test")
            .message_id(None)
            .in_reply_to(bracketed.clone())
            .references(bracketed)
            .singlepart(SinglePart::plain("reply body".to_string()))
            .expect("build reply mail");

        let formatted = email.formatted();
        let text = String::from_utf8_lossy(&formatted);

        assert!(
            text.contains("In-Reply-To: <abc.123@example.com>"),
            "reply mail must contain In-Reply-To header, got:\n{}",
            text
        );
        assert!(
            text.contains("References: <abc.123@example.com>"),
            "reply mail must contain References header, got:\n{}",
            text
        );
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

    /// Verify that a non-reply mail does NOT include In-Reply-To headers.
    #[test]
    fn non_reply_mail_has_no_in_reply_to_header() {
        use lettre::message::SinglePart;
        use lettre::Message;

        let email = Message::builder()
            .from("sender@example.com".parse().unwrap())
            .to("recipient@example.com".parse().unwrap())
            .subject("Test")
            .message_id(None)
            .singlepart(SinglePart::plain("body".to_string()))
            .expect("build plain mail");

        let formatted = email.formatted();
        let text = String::from_utf8_lossy(&formatted);

        assert!(
            !text.contains("In-Reply-To:"),
            "non-reply mail must not contain In-Reply-To header, got:\n{}",
            text
        );
    }
}
