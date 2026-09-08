//! Quick 260908-cjo: Regressionsgate fuer die Trennung zwischen dem
//! LIVE-Versandpfad und dem Startup-BACKFILL.
//!
//! Der Test ruft die **echte** geteilte Render-Funktion
//! `genossi_mail::render::resolve_rendered_content` mit den **echten**
//! Resolver-Implementierungen aus `genossi_service_impl::repayment_context`.
//! Nur die DAOs/Resolver-Traits ringsherum sind Mocks — kein SQLite-Pool
//! noetig, weil `aggregate` keine DB anfasst.
//!
//! Die abgedeckte Lage ist die Produktionslage aus Quick 260908-9ud:
//! abgeschlossene Repayment-Phase, alle Eintraege `PaidOut`, Template
//! referenziert `fiscal_year`.

use std::sync::Arc;

use genossi_config::service::MockConfigService;
use genossi_dao::member::{MemberEntity, MemberStatus, PostalStatus, Salutation};
use genossi_dao::repayment_entry::{
    MockRepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus,
};
use genossi_dao::repayment_phase::{
    MockRepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus,
};
use genossi_dao::{MockTransaction, MockTransactionDao};
use genossi_mail::dao::{MailJob, MailRecipient};
use genossi_mail::render::resolve_rendered_content;
use genossi_mail::template::{MockApplicationResolver, MockMemberResolver};
use genossi_service_impl::repayment_context::{
    HistoricRepaymentContextResolverImpl, RepaymentContextResolverDeps,
    RepaymentContextResolverImpl,
};
use uuid::Uuid;

// ── Deps-Marker fuer beide Resolver-Impls ────────────────────────────────
//
// Beide Resolver bekommen frische Mock-DAOs OHNE Erwartungen: ihre DAO-Felder
// werden auf dem `aggregate`-Pfad nie benutzt. Phase und Entries laedt
// `render.rs` selbst ueber die separat gemockten DAOs.
struct TestDeps;

impl RepaymentContextResolverDeps for TestDeps {
    type Transaction = MockTransaction;
    type RepaymentPhaseDao = MockRepaymentPhaseDao;
    type RepaymentEntryDao = MockRepaymentEntryDao;
}

fn historic_resolver() -> HistoricRepaymentContextResolverImpl<TestDeps> {
    HistoricRepaymentContextResolverImpl {
        repayment_phase_dao: Arc::new(MockRepaymentPhaseDao::new()),
        repayment_entry_dao: Arc::new(MockRepaymentEntryDao::new()),
    }
}

fn live_resolver() -> RepaymentContextResolverImpl<TestDeps> {
    RepaymentContextResolverImpl {
        repayment_phase_dao: Arc::new(MockRepaymentPhaseDao::new()),
        repayment_entry_dao: Arc::new(MockRepaymentEntryDao::new()),
    }
}

// ── Fixtures (analog genossi_mail/src/render.rs, dort privat) ────────────

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
        postal_status: PostalStatus::Erreichbar,
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
        body_html: None,
    }
}

/// Die Produktionslage: die Phase ist ABGESCHLOSSEN.
fn make_closed_phase(fiscal_year: i32, share_value: i64) -> RepaymentPhaseEntity {
    RepaymentPhaseEntity {
        id: Uuid::new_v4(),
        fiscal_year,
        share_value,
        status: RepaymentPhaseStatus::Closed,
        opened_at: None,
        closed_at: Some(sample_datetime()),
        created: sample_datetime(),
        deleted: None,
        version: Uuid::new_v4(),
    }
}

fn entry(
    phase_id: Uuid,
    member_id: Uuid,
    share_count: i32,
    status: RepaymentEntryStatus,
) -> RepaymentEntryEntity {
    RepaymentEntryEntity {
        id: Uuid::new_v4(),
        phase_id,
        member_id,
        share_count_to_pay_out: share_count,
        status,
        created: sample_datetime(),
        deleted: None,
        version: Uuid::new_v4(),
    }
}

fn paid_out_entry(phase_id: Uuid, member_id: Uuid, share_count: i32) -> RepaymentEntryEntity {
    entry(
        phase_id,
        member_id,
        share_count,
        RepaymentEntryStatus::PaidOut,
    )
}

/// Rekursiv klonbarer Mock-Transaction-Helfer (Muster aus
/// `genossi_mail/src/render.rs`).
fn clonable_tx() -> MockTransaction {
    let mut tx = MockTransaction::new();
    tx.expect_clone().returning(clonable_tx);
    tx
}

fn tx_dao() -> MockTransactionDao {
    let mut tx_dao = MockTransactionDao::new();
    tx_dao.expect_transaction().returning(|| Ok(clonable_tx()));
    tx_dao.expect_commit().returning(|_| Ok(()));
    tx_dao
}

fn member_resolver(member: MemberEntity) -> MockMemberResolver {
    let mut resolver = MockMemberResolver::new();
    resolver
        .expect_find_member_by_id()
        .returning(move |_| Ok(Some(member.clone())));
    resolver
}

fn phase_dao(phase: RepaymentPhaseEntity) -> MockRepaymentPhaseDao {
    let mut dao = MockRepaymentPhaseDao::new();
    dao.expect_find_by_id()
        .returning(move |_, _| Ok(Some(phase.clone())));
    dao
}

fn entry_dao(entries: Vec<RepaymentEntryEntity>) -> MockRepaymentEntryDao {
    let mut dao = MockRepaymentEntryDao::new();
    let entries: Arc<[RepaymentEntryEntity]> = entries.into();
    dao.expect_find_by_phase_id()
        .returning(move |_, _| Ok(entries.clone()));
    dao
}

/// Template exakt in Produktions-Form: `fiscal_year` ZUERST, damit der
/// strict-Render an derselben Variable scheitert wie auf Produktion.
const PROD_BODY: &str =
    "Geschaeftsjahr {{ fiscal_year }}: {{ share_count }} Anteile zu je {{ share_value }} EUR = \
     {{ payout_amount }} EUR";

// ── Tests ────────────────────────────────────────────────────────────────

/// Pflichttest (a): abgeschlossene Phase, nur `PaidOut`, Backfill-Pfad liefert
/// den vollstaendigen Kontext und das Template rendert.
#[tokio::test]
async fn backfill_resolver_renders_closed_phase_with_paid_out_entries_only() {
    let member = make_member();
    let member_id = member.id;
    let phase = make_closed_phase(2024, 12000);
    let phase_id = phase.id;

    let recipient = make_recipient(Some(member_id));
    let job = make_job("Auszahlung", PROD_BODY, Some(phase_id));

    let rendered = resolve_rendered_content(
        &recipient,
        &job,
        &member_resolver(member),
        &entry_dao(vec![paid_out_entry(phase_id, member_id, 5)]),
        &phase_dao(phase),
        &tx_dao(),
        &historic_resolver(),
        &MockApplicationResolver::new(),
        &MockConfigService::new(),
    )
    .await
    .expect("Backfill-Resolver muss die abgeschlossene Phase rendern koennen");

    assert_eq!(
        rendered.body,
        "Geschaeftsjahr 2024: 5 Anteile zu je 120,00 EUR = 600,00 EUR"
    );
}

/// Pflichttest (b): derselbe Fall scheitert im Live-Pfad weiterhin — mit
/// derselben Meldung und derselben `fiscal_year`-Diagnose wie auf Produktion.
#[tokio::test]
async fn live_resolver_still_fails_on_closed_phase_with_paid_out_entries_only() {
    let member = make_member();
    let member_id = member.id;
    let phase = make_closed_phase(2024, 12000);
    let phase_id = phase.id;

    let recipient = make_recipient(Some(member_id));
    let job = make_job("Auszahlung", PROD_BODY, Some(phase_id));

    let failure = resolve_rendered_content(
        &recipient,
        &job,
        &member_resolver(member),
        &entry_dao(vec![paid_out_entry(phase_id, member_id, 5)]),
        &phase_dao(phase),
        &tx_dao(),
        &live_resolver(),
        &MockApplicationResolver::new(),
        &MockConfigService::new(),
    )
    .await
    .expect_err("Live-Pfad muss unveraendert scheitern");

    assert!(
        failure
            .message
            .starts_with("Template render error (body): "),
        "unerwartete Meldung: {}",
        failure.message
    );
    let diagnosis = failure
        .diagnosis
        .expect("Quick 260908-9ud: Diagnose muss durchgereicht werden");
    assert!(
        diagnosis.contains("fiscal_year"),
        "Diagnose muss das Produktionssymptom nennen: {}",
        diagnosis
    );
}

/// Pinnt D-02 (status-agnostisch) an der Stelle, an der es weh tut: bei
/// gemischten Eintraegen divergieren Live- und Backfill-Pfad sichtbar.
#[tokio::test]
async fn live_resolver_still_fails_for_mixed_open_and_paid_out_amounts() {
    let member = make_member();
    let member_id = member.id;
    let phase = make_closed_phase(2024, 12000);
    let phase_id = phase.id;

    let recipient = make_recipient(Some(member_id));
    let job = make_job("Auszahlung", "{{ share_count }}", Some(phase_id));

    let entries = vec![
        entry(phase_id, member_id, 2, RepaymentEntryStatus::Open),
        paid_out_entry(phase_id, member_id, 3),
    ];

    let live = resolve_rendered_content(
        &recipient,
        &job,
        &member_resolver(member.clone()),
        &entry_dao(entries.clone()),
        &phase_dao(phase.clone()),
        &tx_dao(),
        &live_resolver(),
        &MockApplicationResolver::new(),
        &MockConfigService::new(),
    )
    .await
    .expect("Live-Pfad rendert — es gibt einen offenen Eintrag");
    assert_eq!(live.body, "2", "Live-Pfad zaehlt nur Open/Contacted");

    let backfill = resolve_rendered_content(
        &recipient,
        &job,
        &member_resolver(member),
        &entry_dao(entries),
        &phase_dao(phase),
        &tx_dao(),
        &historic_resolver(),
        &MockApplicationResolver::new(),
        &MockConfigService::new(),
    )
    .await
    .expect("Backfill-Pfad rendert");
    assert_eq!(
        backfill.body, "5",
        "D-02: der Backfill summiert status-agnostisch alle Eintraege"
    );
}

/// Billiges Gate dagegen, dass ein spaeterer Refactor die beiden
/// `genossi_bin`-Aliase auf denselben Typ zusammenfallen laesst.
#[test]
fn live_and_backfill_resolver_types_are_distinct() {
    use std::any::TypeId;

    let live = TypeId::of::<RepaymentContextResolverImpl<TestDeps>>();
    let historic = TypeId::of::<HistoricRepaymentContextResolverImpl<TestDeps>>();
    assert_ne!(
        live, historic,
        "Live- und Backfill-Resolver muessen unterschiedliche Typen bleiben"
    );
}
