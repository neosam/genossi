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
use crate::template::{ApplicationResolver, MemberResolver};
use genossi_config::service::ConfigService;
use genossi_service::repayment_context::RepaymentContextResolver;

#[allow(clippy::too_many_arguments)]
pub async fn run_rendered_backfill<R, J, M, RE, RP, TX, RCR, AR, CS>(
    recipient_dao: Arc<R>,
    job_dao: Arc<J>,
    member_resolver: Arc<M>,
    repayment_entry_dao: Arc<RE>,
    repayment_phase_dao: Arc<RP>,
    transaction_dao: Arc<TX>,
    repayment_context_resolver: Arc<RCR>,
    // Phase 31 (APMAIL-01, D-04, Pitfall 4): the Application-Zweig is practically
    // never active in the backfill (legacy rows are member-bound), but the two
    // new resolve_rendered_content args must be threaded through so the shared
    // render function keeps ONE signature at both call sites.
    application_resolver: Arc<AR>,
    config_service: Arc<CS>,
) where
    R: MailRecipientDao,
    J: MailJobDao,
    M: MemberResolver,
    RE: genossi_dao::repayment_entry::RepaymentEntryDao + Send + Sync,
    RP: genossi_dao::repayment_phase::RepaymentPhaseDao<Transaction = RE::Transaction>
        + Send
        + Sync,
    TX: genossi_dao::TransactionDao<Transaction = RE::Transaction> + Send + Sync,
    RCR: RepaymentContextResolver<Transaction = RE::Transaction> + Send + Sync,
    AR: ApplicationResolver,
    CS: ConfigService,
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
                // Quick 260908-9ud: structured fields instead of one opaque
                // formatted string — the ids are what a diagnosis starts from.
                tracing::warn!(
                    recipient_id = %recipient.id,
                    mail_job_id = %recipient.mail_job_id,
                    error = ?e,
                    "rendered backfill: skip recipient — job lookup failed"
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
            application_resolver.as_ref(),
            config_service.as_ref(),
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
                        recipient_id = %recipient.id,
                        mail_job_id = %recipient.mail_job_id,
                        error = ?e,
                        "rendered backfill: skip recipient — update failed"
                    );
                    skipped += 1;
                } else {
                    filled += 1;
                }
            }
            Err(failure) => {
                // Missing member or render error — leave the row NULL so the next
                // start can retry once the underlying data is fixed.
                //
                // Quick 260908-9ud: this was THE informationless line — it named
                // only the recipient id and the short message. Optional uuids go
                // through the `?` (Debug) sigil so a `None` stays visible instead
                // of silently disappearing from the log.
                tracing::warn!(
                    recipient_id = %recipient.id,
                    mail_job_id = %recipient.mail_job_id,
                    member_id = ?recipient.member_id,
                    application_id = ?recipient.application_id,
                    template_id = ?job.template_id,
                    repayment_phase_id = ?job.repayment_phase_id,
                    diagnosis = ?failure.diagnosis,
                    "rendered backfill: skip recipient — {}",
                    failure.message
                );
                skipped += 1;
            }
        }
    }

    tracing::info!(
        filled,
        total,
        skipped,
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
    use crate::template::{MockApplicationResolver, MockMemberResolver};
    use genossi_config::service::MockConfigService;
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
            application_id: None,
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
            Arc::new(MockApplicationResolver::new()),
            Arc::new(MockConfigService::new()),
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
        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(make_job()));

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
            Arc::new(MockApplicationResolver::new()),
            Arc::new(MockConfigService::new()),
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
            Arc::new(MockApplicationResolver::new()),
            Arc::new(MockConfigService::new()),
        )
        .await;
    }

    // ---------------------------------------------------------------------
    // Quick 260908-9ud: die Skip-Warnung muss allein zur Ursachenbenennung
    // genuegen — und darf dabei keine Mitglieds-Werte enthalten (D-02).
    // ---------------------------------------------------------------------

    /// Sammelt die `tracing`-Ausgabe eines Laufs in einem gemeinsamen Puffer.
    #[derive(Clone, Default)]
    struct CapturedLog(Arc<std::sync::Mutex<Vec<u8>>>);

    impl CapturedLog {
        fn contents(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    impl std::io::Write for CapturedLog {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
        type Writer = CapturedLog;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// Fuehrt `fut` unter einem WARN-Subscriber aus, dessen Ausgabe eingesammelt
    /// wird, und liefert den Log-Text zurueck.
    ///
    /// `with_default` ist thread-lokal (parallel-test-sicher); der
    /// current_thread-Runtime haelt alle await-Punkte auf demselben Thread,
    /// sodass der Dispatcher ueber die gesamte Ausfuehrung greift.
    fn capture_warn_logs<F: std::future::Future<Output = ()>>(fut: F) -> String {
        let logs = CapturedLog::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tracing::subscriber::with_default(subscriber, || rt.block_on(fut));
        logs.contents()
    }

    fn make_job_with_body(body: &str) -> MailJob {
        MailJob {
            body: Arc::from(body),
            ..make_job()
        }
    }

    /// Baut die Backfill-Lage, in der ein Recipient am Strict-Render scheitert:
    /// Member existiert, `repayment_phase_id: None` (also kein Merge), aber der
    /// Job-Body referenziert `{{ payout_amount }}`.
    fn run_backfill_with_failing_render(member: MemberEntity) -> String {
        let member_id = member.id;
        let recipient = make_recipient(Some(member_id));

        let mut recipient_dao = MockMailRecipientDao::new();
        let row = recipient.clone();
        recipient_dao
            .expect_find_recipients_without_rendered()
            .returning(move || Ok(vec![row.clone()].into()));
        recipient_dao.expect_update().never();

        let mut job_dao = MockMailJobDao::new();
        job_dao
            .expect_find_by_id()
            .returning(move |_| Ok(make_job_with_body("Auszahlung: {{ payout_amount }} EUR")));

        let mut resolver = MockMemberResolver::new();
        resolver
            .expect_find_member_by_id()
            .returning(move |_| Ok(Some(member.clone())));

        capture_warn_logs(async move {
            run_rendered_backfill(
                Arc::new(recipient_dao),
                Arc::new(job_dao),
                Arc::new(resolver),
                Arc::new(MockRepaymentEntryDao::new()),
                Arc::new(MockRepaymentPhaseDao::new()),
                Arc::new(MockTransactionDao::new()),
                Arc::new(MockRepaymentContextResolver::new()),
                Arc::new(MockApplicationResolver::new()),
                Arc::new(MockConfigService::new()),
            )
            .await;
        })
    }

    /// Die Kernaussage des Quicks, automatisiert belegt: das Log allein genuegt
    /// zur Ursachenbenennung.
    #[test]
    fn backfill_skip_log_carries_ids_and_variable_name() {
        let member = make_member();
        let member_id = member.id;

        let logs = run_backfill_with_failing_render(member);

        for field in [
            "recipient_id",
            "mail_job_id",
            "member_id",
            "application_id",
            "template_id",
            "repayment_phase_id",
            "diagnosis",
        ] {
            assert!(
                logs.contains(field),
                "Skip-Warnung nennt das Feld `{}` nicht:\n{}",
                field,
                logs
            );
        }
        assert!(
            logs.contains(&member_id.to_string()),
            "Skip-Warnung nennt die member_id nicht:\n{}",
            logs
        );
        assert!(
            logs.contains("payout_amount"),
            "Skip-Warnung nennt die fehlende Template-Variable nicht:\n{}",
            logs
        );
    }

    /// D-02-Gate: derselbe Lauf mit einem Member, dessen IBAN und Nachname
    /// unverwechselbar sind — keiner der beiden Werte darf im Log landen.
    #[test]
    fn backfill_skip_log_omits_member_values() {
        let iban = "DE89370400440532013000";
        let mut member = make_member();
        member.bank_account = Some(Arc::from(iban));
        member.last_name = Arc::from("Unverwechselbarname");

        let logs = run_backfill_with_failing_render(member);

        assert!(
            !logs.contains(iban),
            "Log enthaelt die unmaskierte IBAN:\n{}",
            logs
        );
        assert!(
            !logs.contains("Unverwechselbarname"),
            "Log enthaelt den Nachnamen:\n{}",
            logs
        );
    }
}
