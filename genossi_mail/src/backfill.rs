//! Quick 260614-b1t: one-shot startup backfill for legacy mail_recipients rows.
//!
//! Before Quick 260614-9zf the worker rendered each recipient's subject/body but
//! discarded the result after sending. Those legacy rows now have NULL
//! rendered_subject/rendered_body. This job runs ONCE on server start (after
//! migrations) and retroactively re-renders them, marking each filled row
//! `rendered_reconstructed = true` so the frontend can flag it as a
//! reconstruction (NOT the byte-accurate original from the send moment).
//!
//! Idempotency: `find_recipients_without_rendered` only returns NULL-rendered
//! rows, so a second run after a successful fill is a no-op. Rows whose member
//! cannot be loaded or whose template fails to render are skipped (stay NULL) and
//! retried on the next start.

use std::sync::Arc;

use uuid::Uuid;

use crate::dao::{MailJobDao, MailRecipientDao};
use crate::render::resolve_rendered_content;
use crate::template::MemberResolver;
use genossi_service::repayment_context::RepaymentContextResolver;

#[allow(clippy::too_many_arguments)]
pub async fn run_rendered_backfill<R, J, M, RE, RP, TX, RCR>(
    recipient_dao: Arc<R>,
    job_dao: Arc<J>,
    member_resolver: Arc<M>,
    repayment_entry_dao: Arc<RE>,
    repayment_phase_dao: Arc<RP>,
    transaction_dao: Arc<TX>,
    repayment_context_resolver: Arc<RCR>,
) where
    R: MailRecipientDao,
    J: MailJobDao,
    M: MemberResolver,
    RE: genossi_dao::repayment_entry::RepaymentEntryDao + Send + Sync,
    RP: genossi_dao::repayment_phase::RepaymentPhaseDao<Transaction = RE::Transaction> + Send + Sync,
    TX: genossi_dao::TransactionDao<Transaction = RE::Transaction> + Send + Sync,
    RCR: RepaymentContextResolver<Transaction = RE::Transaction> + Send + Sync,
{
    let recipients = match recipient_dao.find_recipients_without_rendered().await {
        Ok(rs) => rs,
        Err(e) => {
            tracing::error!("rendered backfill: failed to load recipients: {:?}", e);
            return;
        }
    };

    let total = recipients.len();
    let mut filled = 0usize;
    let mut skipped = 0usize;

    for recipient in recipients.iter() {
        let job = match job_dao.find_by_id(recipient.mail_job_id).await {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!(
                    "rendered backfill: skip recipient {} — job {} lookup failed: {:?}",
                    recipient.id,
                    recipient.mail_job_id,
                    e
                );
                skipped += 1;
                continue;
            }
        };

        match resolve_rendered_content(
            recipient,
            &job,
            member_resolver.as_ref(),
            repayment_entry_dao.as_ref(),
            repayment_phase_dao.as_ref(),
            transaction_dao.as_ref(),
            repayment_context_resolver.as_ref(),
        )
        .await
        {
            Ok(rendered) => {
                // Phase 23: destructure to the new RenderedContent shape;
                // rendered_html_body persistence is a Plan 04 concern — this
                // backfill only fills the legacy text fields.
                let mut updated = recipient.clone();
                updated.version = Uuid::new_v4();
                updated.rendered_subject = Some(Arc::from(rendered.subject.as_str()));
                updated.rendered_body = Some(Arc::from(rendered.body.as_str()));
                updated.rendered_reconstructed = true;
                if let Err(e) = recipient_dao.update(&updated).await {
                    tracing::warn!(
                        "rendered backfill: skip recipient {} — update failed: {:?}",
                        recipient.id,
                        e
                    );
                    skipped += 1;
                } else {
                    filled += 1;
                }
            }
            Err(failure) => {
                // Missing member or render error — leave the row NULL so the next
                // start can retry once the underlying data is fixed.
                tracing::warn!(
                    "rendered backfill: skip recipient {} — {}",
                    recipient.id,
                    failure.message
                );
                skipped += 1;
            }
        }
    }

    tracing::info!(
        "rendered backfill: {} von {} befüllt, {} übersprungen",
        filled,
        total,
        skipped
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::{MailJob, MailRecipient, MockMailJobDao, MockMailRecipientDao};
    use crate::template::MockMemberResolver;
    use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
    use genossi_dao::repayment_entry::MockRepaymentEntryDao;
    use genossi_dao::repayment_phase::MockRepaymentPhaseDao;
    use genossi_dao::MockTransactionDao;
    use genossi_service::repayment_context::MockRepaymentContextResolver;

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
            title: None,
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
            status: Arc::from("sent"),
            error: None,
            sent_at: Some(sample_datetime()),
            message_id: None,
            rendered_subject: None,
            rendered_body: None,
            rendered_html_body: None,
            rendered_reconstructed: false,
        }
    }

    fn make_job() -> MailJob {
        MailJob {
            id: Uuid::new_v4(),
            created: sample_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
            subject: Arc::from("Hallo {{ first_name }}"),
            body: Arc::from("Lieber {{ last_name }}"),
            status: Arc::from("done"),
            total_count: 1,
            sent_count: 1,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id: None,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    #[tokio::test]
    async fn test_backfill_fills_null_rows_and_sets_flag_true() {
        let member = make_member();
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));
        let job_id = recipient.mail_job_id;

        let mut recipient_dao = MockMailRecipientDao::new();
        let row = recipient.clone();
        recipient_dao
            .expect_find_recipients_without_rendered()
            .returning(move || Ok(vec![row.clone()].into()));
        // update MUST be called with the rendered content + flag=true.
        recipient_dao
            .expect_update()
            .times(1)
            .withf(|r: &MailRecipient| {
                r.rendered_reconstructed
                    && r.rendered_subject.as_deref() == Some("Hallo Max")
                    && r.rendered_body.as_deref() == Some("Lieber Mustermann")
            })
            .returning(|_| Ok(()));

        let mut job_dao = MockMailJobDao::new();
        job_dao
            .expect_find_by_id()
            .withf(move |id: &Uuid| *id == job_id)
            .returning(move |_| Ok(make_job()));

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        run_rendered_backfill(
            Arc::new(recipient_dao),
            Arc::new(job_dao),
            Arc::new(resolver),
            Arc::new(MockRepaymentEntryDao::new()),
            Arc::new(MockRepaymentPhaseDao::new()),
            Arc::new(MockTransactionDao::new()),
            Arc::new(MockRepaymentContextResolver::new()),
        )
        .await;
    }

    #[tokio::test]
    async fn test_backfill_skips_missing_member_leaves_null() {
        let recipient = make_recipient(Some(Uuid::new_v4()));

        let mut recipient_dao = MockMailRecipientDao::new();
        let row = recipient.clone();
        recipient_dao
            .expect_find_recipients_without_rendered()
            .returning(move || Ok(vec![row.clone()].into()));
        // update must NOT be called for a row whose member cannot be loaded.
        recipient_dao.expect_update().never();

        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_find_by_id().returning(move |_| Ok(make_job()));

        let mut resolver = MockMemberResolver::new();
        resolver.expect_find_member_by_id().returning(|_| Ok(None));

        run_rendered_backfill(
            Arc::new(recipient_dao),
            Arc::new(job_dao),
            Arc::new(resolver),
            Arc::new(MockRepaymentEntryDao::new()),
            Arc::new(MockRepaymentPhaseDao::new()),
            Arc::new(MockTransactionDao::new()),
            Arc::new(MockRepaymentContextResolver::new()),
        )
        .await;
    }

    #[tokio::test]
    async fn test_backfill_idempotent_does_not_touch_filled_rows() {
        let mut recipient_dao = MockMailRecipientDao::new();
        // Empty list (all rows already filled) → no job lookups, no updates.
        recipient_dao
            .expect_find_recipients_without_rendered()
            .returning(|| Ok(Vec::<MailRecipient>::new().into()));
        recipient_dao.expect_update().never();

        let mut job_dao = MockMailJobDao::new();
        job_dao.expect_find_by_id().never();

        run_rendered_backfill(
            Arc::new(recipient_dao),
            Arc::new(job_dao),
            Arc::new(MockMemberResolver::new()),
            Arc::new(MockRepaymentEntryDao::new()),
            Arc::new(MockRepaymentPhaseDao::new()),
            Arc::new(MockTransactionDao::new()),
            Arc::new(MockRepaymentContextResolver::new()),
        )
        .await;
    }
}
