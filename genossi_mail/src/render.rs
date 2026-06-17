//! Quick 260614-b1t: single source of truth for per-recipient template rendering.
//!
//! Extracted verbatim from the inline render block that used to live in
//! `worker.rs::start_mail_worker` (~378-577). Both the live mail worker AND the
//! startup backfill (`crate::backfill::run_rendered_backfill`) call this one
//! function, so the rendered subject/body is computed identically in both paths
//! (DRY — must-have truth "Die Render-Logik existiert nur einmal").
//!
//! Behavior contract (byte-for-byte identical to the pre-refactor worker path):
//! - `recipient.member_id == None` → plain passthrough `(job.subject, job.body)`.
//! - `recipient.member_id == Some(id)` → load member via MemberResolver:
//!     - member not found / resolver error → `Err(RenderFailure)`.
//!     - `job.repayment_phase_id == Some(phase_id)` → open a read tx, look up the
//!       phase + entries, derive `share_value_str` from `phase.share_value`, and:
//!         - aggregate Ok → merge the repayment context into the member context.
//!         - aggregate Err(EntityNotFound) → leave context unmerged (strict-env
//!           render fails downstream if the template references repayment vars —
//!           the intended D-05 behavior).
//!         - aggregate Err(other) → `Err(RenderFailure)`.
//!     - render subject + body with the (possibly merged) context.

use genossi_service::repayment_context::RepaymentContextResolver;
use genossi_service::ServiceError;

use crate::dao::{MailJob, MailRecipient};
use crate::template::{member_to_template_context, merge_repayment_context, render_template, MemberResolver};

/// Failure cause from `resolve_rendered_content`. The caller (worker) maps the
/// `message` onto `mark_recipient_failed`; the backfill logs it and skips the row.
#[derive(Debug, Clone)]
pub struct RenderFailure {
    pub message: String,
}

impl RenderFailure {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Render the subject + body a single recipient should receive.
///
/// Returns `Ok((subject, body))` on success, `Err(RenderFailure)` on
/// member-resolution / repayment-aggregation / template-render errors.
#[allow(clippy::too_many_arguments)]
pub async fn resolve_rendered_content<M, RE, RP, TX, RCR>(
    recipient: &MailRecipient,
    job: &MailJob,
    member_resolver: &M,
    repayment_entry_dao: &RE,
    repayment_phase_dao: &RP,
    transaction_dao: &TX,
    repayment_context_resolver: &RCR,
) -> Result<(String, String), RenderFailure>
where
    M: MemberResolver,
    RE: genossi_dao::repayment_entry::RepaymentEntryDao + Send + Sync,
    RP: genossi_dao::repayment_phase::RepaymentPhaseDao<Transaction = RE::Transaction> + Send + Sync,
    TX: genossi_dao::TransactionDao<Transaction = RE::Transaction> + Send + Sync,
    RCR: RepaymentContextResolver<Transaction = RE::Transaction> + Send + Sync,
{
    let Some(member_id) = recipient.member_id else {
        // No member_id — plain text passthrough (no template interpolation).
        return Ok((job.subject.to_string(), job.body.to_string()));
    };

    let member = match member_resolver.find_member_by_id(member_id).await {
        Ok(Some(member)) => member,
        Ok(None) => {
            return Err(RenderFailure::new(format!(
                "Member {} not found for template rendering",
                member_id
            )));
        }
        Err(e) => {
            return Err(RenderFailure::new(format!(
                "Failed to load member for template rendering: {:?}",
                e
            )));
        }
    };

    let mut ctx = member_to_template_context(&member);

    if let Some(phase_id) = job.repayment_phase_id {
        let agg_tx = transaction_dao.transaction().await.map_err(|e| {
            RenderFailure::new(format!("Worker: cannot open tx for repayment context: {:?}", e))
        })?;

        let phase_opt = match repayment_phase_dao.find_by_id(phase_id, agg_tx.clone()).await {
            Ok(p) => p,
            Err(e) => {
                return Err(RenderFailure::new(format!(
                    "Worker: repayment_phase lookup failed: {:?}",
                    e
                )));
            }
        };

        if let Some(phase) = phase_opt {
            let entries = match repayment_entry_dao.find_by_phase_id(phase_id, agg_tx.clone()).await
            {
                Ok(es) => es,
                Err(e) => {
                    return Err(RenderFailure::new(format!(
                        "Worker: repayment_entry lookup failed: {:?}",
                        e
                    )));
                }
            };

            // Phase-wide Anteilswert (German euro string) derived locally —
            // RepaymentContext does not carry it (Quick 260602-r2i).
            let share_value_str =
                format!("{},{:02}", phase.share_value / 100, phase.share_value % 100);

            match repayment_context_resolver.aggregate(&phase, &entries, member.id) {
                Ok(rc) => {
                    ctx = merge_repayment_context(
                        ctx,
                        &rc.payout_amount,
                        rc.share_count,
                        &share_value_str,
                        rc.fiscal_year,
                    );
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    // D-05 edge-case: no Open/Contacted entries — leave context
                    // unmerged. Strict-env render fails on referenced repayment
                    // vars (intended), preserving pre-refactor behavior.
                }
                Err(e) => {
                    return Err(RenderFailure::new(format!(
                        "Worker: repayment_context aggregate failed: {:?}",
                        e
                    )));
                }
            }
        }

        // Release the read tx — best-effort, errors ignored (read-only).
        let _ = transaction_dao.commit(agg_tx).await;
    }

    let subject = render_template(&job.subject, &ctx)
        .map_err(|e| RenderFailure::new(format!("Template render error (subject): {}", e.message)))?;
    let body = render_template(&job.body, &ctx)
        .map_err(|e| RenderFailure::new(format!("Template render error (body): {}", e.message)))?;

    Ok((subject, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::MockMemberResolver;
    use std::sync::Arc;
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
    use genossi_dao::repayment_entry::{MockRepaymentEntryDao, RepaymentEntryEntity};
    use genossi_dao::repayment_phase::{MockRepaymentPhaseDao, RepaymentPhaseEntity};
    use genossi_dao::{MockTransaction, MockTransactionDao};
    use genossi_service::repayment_context::{MockRepaymentContextResolver, RepaymentContext};
    use uuid::Uuid;

    fn sample_datetime() -> time::PrimitiveDateTime {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        )
    }

    fn make_member() -> MemberEntity {
        let date = time::Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberEntity {
            id: Uuid::new_v4(),
            member_number: 42,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: Some(Salutation::Herr),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("max@example.com")),
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 3,
            current_balance: 15000,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: MemberStatus::Normal,
            account_holder: None,
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn make_recipient(member_id: Option<Uuid>) -> MailRecipient {
        MailRecipient {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            mail_job_id: Uuid::new_v4(),
            to_address: Arc::from("max@example.com"),
            member_id,
            status: Arc::from("pending"),
            error: None,
            sent_at: None,
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_reconstructed: false,
        }
    }

    fn make_job(subject: &str, body: &str, repayment_phase_id: Option<Uuid>) -> MailJob {
        MailJob {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from(subject),
            body: Arc::from(body),
            status: Arc::from("running"),
            total_count: 1,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id,
            attach_repayment_letter: false,
        }
    }

    fn make_phase(fiscal_year: i32, share_value: i64) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year,
            share_value,
            status: genossi_dao::repayment_phase::RepaymentPhaseStatus::Open,
            opened_at: None,
            closed_at: None,
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_member_only() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let job = make_job("Hallo {{ first_name }}", "Lieber {{ last_name }}", None);

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let (subject, body) = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(subject, "Hallo Max");
        assert_eq!(body, "Lieber Mustermann");
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_repayment_merge() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let phase_id = Uuid::new_v4();
        let job = make_job(
            "Auszahlung {{ payout_amount }}",
            "{{ share_count }} Anteile a {{ share_value }} EUR GJ {{ fiscal_year }}",
            Some(phase_id),
        );

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        let mut tx_dao = MockTransactionDao::new();
        tx_dao.expect_transaction().returning(|| {
            // The render path clones the tx for each DAO call; supply a tx whose
            // clone() yields fresh clonable mocks.
            fn clonable_tx() -> MockTransaction {
                let mut tx = MockTransaction::new();
                tx.expect_clone().returning(clonable_tx);
                tx
            }
            Ok(clonable_tx())
        });
        tx_dao.expect_commit().returning(|_| Ok(()));

        let mut phase_dao = MockRepaymentPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(make_phase(2026, 2000))));

        let mut entry_dao = MockRepaymentEntryDao::new();
        entry_dao
            .expect_find_by_phase_id()
            .returning(|_, _| Ok(Vec::<RepaymentEntryEntity>::new().into()));

        let mut rcr = MockRepaymentContextResolver::new();
        rcr.expect_aggregate().returning(|_, _, _| {
            Ok(RepaymentContext {
                share_count: 3,
                payout_amount: "60,00".to_string(),
                fiscal_year: 2026,
            })
        });

        let (subject, body) = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(subject, "Auszahlung 60,00");
        // share_value derived from phase.share_value=2000 → "20,00".
        assert_eq!(body, "3 Anteile a 20,00 EUR GJ 2026");
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_plain_passthrough_no_member() {
        let recipient = make_recipient(None);
        let job = make_job("Plain Subject", "Plain Body", None);

        let resolver = MockMemberResolver::new();
        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let (subject, body) = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await
        .unwrap();

        assert_eq!(subject, "Plain Subject");
        assert_eq!(body, "Plain Body");
    }

    #[tokio::test]
    async fn test_resolve_rendered_content_missing_member_returns_err() {
        let recipient = make_recipient(Some(Uuid::new_v4()));
        let job = make_job("Hallo {{ first_name }}", "Body", None);

        let mut resolver = MockMemberResolver::new();
        resolver.expect_find_member_by_id().returning(|_| Ok(None));

        let entry_dao = MockRepaymentEntryDao::new();
        let phase_dao = MockRepaymentPhaseDao::new();
        let tx_dao = MockTransactionDao::new();
        let rcr = MockRepaymentContextResolver::new();

        let result = resolve_rendered_content(
            &recipient, &job, &resolver, &entry_dao, &phase_dao, &tx_dao, &rcr,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not found"));
    }
}
