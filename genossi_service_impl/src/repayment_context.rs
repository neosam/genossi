//! Phase 13 D-13-04 / D-13-10: Impl von RepaymentContextResolver.
//!
//! Pure-Function `aggregate_for_member` ist mockless direkt testbar — 1:1
//! Mirror der Phase-10-Worker-Logik aus `genossi_mail/src/worker.rs:332-360`
//! (Filter Open+Contacted, SUM share_count, deutsche Euro-Formatierung).
//!
//! Trait-Methode `aggregate` ist ein duenner Wrapper (kein DB-Round-Trip),
//! `resolve` laedt Phase + Entries via DAOs und delegiert an `aggregate`.
//!
//! D-13-10: Phase-10-Mail-Worker bleibt UNVERAENDERT — der Worker-Refactor
//! laeuft als separates `/gsd-quick` nach Phase 13 (Todo
//! `.planning/todos/pending/phase-10-worker-refactor-resolver.md`).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity};
use genossi_dao::Transaction;
use genossi_service::repayment_context::{RepaymentContext, RepaymentContextResolver};
use genossi_service::ServiceError;

/// Dependency-Injection-Trait fuer `RepaymentContextResolverImpl`.
/// Vorbild: `RepaymentExportServiceDeps` (abgespeckt — nur die zwei DAOs,
/// die Resolver tatsaechlich benoetigt).
pub trait RepaymentContextResolverDeps: Send + Sync + 'static {
    type Transaction: Transaction;
    type RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> + Send + Sync;
    type RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> + Send + Sync;
}

/// Konkrete Resolver-Implementation. Plan 13.04 instanziiert sie mit den
/// Production-`Deps`, die `genossi_bin` bereitstellt.
pub struct RepaymentContextResolverImpl<Deps: RepaymentContextResolverDeps> {
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
}

/// Pure aggregation function — direkt ohne Mocks testbar.
/// 1:1 Spiegel von `genossi_mail/src/worker.rs:332-360`.
///
/// Returns `None` wenn keine relevanten Entries (`Open`/`Contacted`,
/// `deleted IS NULL`, `member_id == X`) fuer das Member existieren.
///
/// `payout_amount` wird in deutscher Lokalisierung formatiert
/// (`format!("{},{:02}", cents/100, cents%100)`) — KEIN Tausenderpunkt,
/// KEIN Euro-Symbol (Phase 10 D-04 Konvention).
pub fn aggregate_for_member(
    phase: &RepaymentPhaseEntity,
    entries: &[RepaymentEntryEntity],
    member_id: Uuid,
) -> Option<RepaymentContext> {
    let relevant: Vec<&RepaymentEntryEntity> = entries
        .iter()
        .filter(|e| {
            e.deleted.is_none()
                && e.member_id == member_id
                && matches!(
                    e.status,
                    RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted,
                )
        })
        .collect();

    if relevant.is_empty() {
        return None;
    }

    Some(context_from_relevant(phase, &relevant))
}

/// Quick 260908-cjo: gemeinsamer Summen-/Formatierungskern von
/// [`aggregate_for_member`] und [`aggregate_for_member_historic`].
///
/// Verhaltensneutral herausgezogen — Inhalt ist 1:1 der frühere Rumpf von
/// `aggregate_for_member` nach dem `is_empty()`-Guard. Damit lebt die deutsche
/// Euro-Formatierung (Phase 10 D-04: KEIN Tausenderpunkt, KEIN Euro-Symbol,
/// zero-padded Cents) an genau einer Stelle und kann zwischen den beiden
/// Aggregations-Semantiken nicht driften.
///
/// Precondition: `relevant` ist nicht leer (die Aufrufer prüfen das).
fn context_from_relevant(
    phase: &RepaymentPhaseEntity,
    relevant: &[&RepaymentEntryEntity],
) -> RepaymentContext {
    let share_count: i32 = relevant.iter().map(|e| e.share_count_to_pay_out).sum();
    let cents: i64 = (share_count as i64) * (phase.share_value);
    // German Locale "X,YZ" — Phase 10 D-04 Pattern-konstant.
    let payout_amount = format!("{},{:02}", cents / 100, cents % 100);

    RepaymentContext {
        share_count,
        payout_amount,
        fiscal_year: phase.fiscal_year,
    }
}

/// Quick 260908-cjo (D-02): Ergebnis der HISTORISCHEN Aggregation.
///
/// Trägt neben dem eigentlichen [`RepaymentContext`] die beiden Zähler, aus
/// denen der historische Resolver sein `warn!` speist — und macht die
/// Divergenz zur strikten Aggregation damit testbar, ohne einen
/// `tracing`-Subscriber in den Tests aufsetzen zu müssen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricAggregate {
    pub context: RepaymentContext,
    /// Anzahl der summierten (nicht-gelöschten) Einträge des Mitglieds.
    pub entry_count: usize,
    /// Anzahl davon, die NICHT mehr `Open`/`Contacted` sind.
    pub stale_status_count: usize,
    /// `stale_status_count > 0` — mindestens ein summierter Eintrag hätte die
    /// strikte Aggregation nicht passiert, das Ergebnis weicht also vom
    /// Live-Pfad ab (kleinere Zahl oder `None`).
    pub diverges_from_live: bool,
}

/// Quick 260908-cjo (D-02): HISTORISCHE Aggregation — **status-agnostisch**.
///
/// Summiert ALLE nicht-gelöschten Einträge des Mitglieds in der Phase,
/// unabhängig vom `RepaymentEntryStatus`. Die übrigen Filter
/// (`deleted.is_none()`, `member_id`) bleiben gegenüber
/// [`aggregate_for_member`] unverändert.
///
/// Zweck: der Startup-Backfill (`genossi_mail::backfill::run_rendered_backfill`)
/// rekonstruiert Texte bereits VERSENDETER Mails. Ist die Phase inzwischen
/// abgeschlossen, stehen alle Einträge auf `PaidOut` und die strikte
/// Aggregation liefert `None` — der Repayment-Kontext bliebe unmerged und der
/// strict-Render bräche an `fiscal_year` ab.
///
/// Bewusst NICHT "nur wenn kein Open/Contacted existiert, dann auch PaidOut":
/// diese Regel wäre unstetig (1 Open + 5 PaidOut ⇒ 1, 0 Open + 5 PaidOut ⇒ 5)
/// und lieferte bei gemischten Einträgen zu kleine Beträge, weil Einträge
/// monoton `Open → Contacted → PaidOut` wandern und zum Versandzeitpunkt noch
/// nicht ausgezahlt waren.
///
/// **Nie im Live-Versandpfad verwenden** — dort verhindert der
/// Open/Contacted-Filter die Auszahlungsmail an bereits ausgezahlte Mitglieder.
pub fn aggregate_for_member_historic(
    phase: &RepaymentPhaseEntity,
    entries: &[RepaymentEntryEntity],
    member_id: Uuid,
) -> Option<HistoricAggregate> {
    let relevant: Vec<&RepaymentEntryEntity> = entries
        .iter()
        .filter(|e| e.deleted.is_none() && e.member_id == member_id)
        .collect();

    if relevant.is_empty() {
        return None;
    }

    let stale_status_count = relevant
        .iter()
        .filter(|e| {
            !matches!(
                e.status,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted,
            )
        })
        .count();

    Some(HistoricAggregate {
        context: context_from_relevant(phase, &relevant),
        entry_count: relevant.len(),
        stale_status_count,
        diverges_from_live: stale_status_count > 0,
    })
}

/// Quick 260908-cjo: geteilte DAO-Ladelogik beider `resolve`-Implementierungen.
///
/// Inhalt = die früheren Schritte 1 und 2 von
/// `RepaymentContextResolverImpl::resolve`, wörtlich übernommen. Existiert,
/// damit die Ladelogik zwischen striktem und historischem Resolver nicht
/// driftet — nur die anschließende Aggregation unterscheidet sich.
async fn load_phase_and_entries<PD, ED, T>(
    phase_dao: &PD,
    entry_dao: &ED,
    phase_id: Uuid,
    tx: T,
) -> Result<(RepaymentPhaseEntity, Vec<RepaymentEntryEntity>), ServiceError>
where
    T: Transaction,
    PD: RepaymentPhaseDao<Transaction = T> + Send + Sync,
    ED: RepaymentEntryDao<Transaction = T> + Send + Sync,
{
    // 1. Load phase (404 if missing).
    let phase = phase_dao
        .find_by_id(phase_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(phase_id))?;

    // 2. Load entries fuer die Phase. find_by_phase_id filtert soft-deleted
    //    via Default-Impl; aggregate filtert nochmal (Defense-in-Depth).
    let entries: Vec<RepaymentEntryEntity> = entry_dao
        .find_by_phase_id(phase_id, tx)
        .await?
        .iter()
        .cloned()
        .collect();

    Ok((phase, entries))
}

#[async_trait]
impl<Deps: RepaymentContextResolverDeps> RepaymentContextResolver
    for RepaymentContextResolverImpl<Deps>
{
    type Transaction = Deps::Transaction;

    async fn resolve(
        &self,
        phase_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<RepaymentContext, ServiceError> {
        // 1./2. Phase + Entries laden (Quick 260908-cjo: geteilt mit der
        //       historischen Impl, damit die Ladelogik nicht driftet).
        let (phase, entries) = load_phase_and_entries(
            self.repayment_phase_dao.as_ref(),
            self.repayment_entry_dao.as_ref(),
            phase_id,
            tx,
        )
        .await?;

        // 3. Aggregate via Trait-Methode (zentrale Stelle, eine Wahrheits-Quelle).
        self.aggregate(&phase, &entries, member_id)
    }

    fn aggregate(
        &self,
        phase: &RepaymentPhaseEntity,
        entries: &[RepaymentEntryEntity],
        member_id: Uuid,
    ) -> Result<RepaymentContext, ServiceError> {
        aggregate_for_member(phase, entries, member_id)
            .ok_or(ServiceError::EntityNotFound(member_id))
    }
}

/// Quick 260908-cjo (D-01): HISTORISCHER Resolver — **ausschliesslich** fuer den
/// Startup-Backfill (`genossi_mail::backfill::run_rendered_backfill`).
///
/// NIE in `start_mail_worker` und NIE im `RepaymentLetterService` verwenden:
/// dort ist der `Open`/`Contacted`-Filter der strikten Impl fachlich richtig,
/// weil er die Auszahlungsmail an bereits ausgezahlte Mitglieder verhindert.
///
/// Der Backfill rekonstruiert dagegen Texte BEREITS VERSENDETER Mails; dort ist
/// eine abgeschlossene Phase (alle Eintraege `PaidOut`) der Normalfall, und der
/// Statusfilter wuerde den Repayment-Kontext ausblenden statt ihn zu schuetzen.
///
/// Die Trennung laeuft ueber den Typ (Trait-Impl), nicht ueber ein Flag oder
/// einen Modus-Parameter: `genossi_mail::render::resolve_rendered_content` ist
/// generisch ueber `RCR: RepaymentContextResolver` und bleibt unveraendert
/// (D-04 — `genossi_mail` bekommt keine Zeile Diff).
///
/// Struct-Form identisch zu [`RepaymentContextResolverImpl`] (dieselben zwei
/// DAO-`Arc`s, dieselben [`RepaymentContextResolverDeps`]), damit `genossi_bin`
/// denselben `Deps`-Typ und dieselben DAO-Arcs wiederverwenden kann
/// (Single-Arc-per-Process).
pub struct HistoricRepaymentContextResolverImpl<Deps: RepaymentContextResolverDeps> {
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
}

#[async_trait]
impl<Deps: RepaymentContextResolverDeps> RepaymentContextResolver
    for HistoricRepaymentContextResolverImpl<Deps>
{
    type Transaction = Deps::Transaction;

    async fn resolve(
        &self,
        phase_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<RepaymentContext, ServiceError> {
        let (phase, entries) = load_phase_and_entries(
            self.repayment_phase_dao.as_ref(),
            self.repayment_entry_dao.as_ref(),
            phase_id,
            tx,
        )
        .await?;

        self.aggregate(&phase, &entries, member_id)
    }

    fn aggregate(
        &self,
        phase: &RepaymentPhaseEntity,
        entries: &[RepaymentEntryEntity],
        member_id: Uuid,
    ) -> Result<RepaymentContext, ServiceError> {
        let historic = aggregate_for_member_historic(phase, entries, member_id)
            // Identische Fehlerkonvention wie die strikte Impl.
            .ok_or(ServiceError::EntityNotFound(member_id))?;

        if historic.diverges_from_live {
            // D-05/D-06: Ids + Anzahlen, KEINE Geldwerte und KEINE
            // Anteilszahlen — Server-Logs sind weder zugriffsbeschraenkt noch
            // loeschfristen-verwaltet (Quick 260908-9ud D-02).
            tracing::warn!(
                phase_id = %phase.id,
                member_id = %member_id,
                entry_count = historic.entry_count,
                stale_status_count = historic.stale_status_count,
                "backfill: historische Rekonstruktion aus nicht mehr offenen Repayment-Eintraegen \
                 — Betraege und Anteilszahlen stammen aus dem HEUTIGEN Datenstand und koennen vom \
                 urspruenglich versendeten Text abweichen"
            );
        }

        Ok(historic.context)
    }
}

#[cfg(test)]
mod tests {
    // RED-Phase: alle Referenzen schlagen fehl (aggregate_for_member +
    // RepaymentContextResolverImpl + RepaymentContextResolverDeps noch
    // nicht definiert). GREEN-Commit fuegt sie hinzu.

    use std::sync::Arc;
    use time::macros::datetime;
    use uuid::Uuid;

    use genossi_dao::repayment_entry::{RepaymentEntryEntity, RepaymentEntryStatus};
    use genossi_dao::repayment_phase::{RepaymentPhaseEntity, RepaymentPhaseStatus};

    use super::*;

    fn sample_phase(share_value_cents: i64) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2025,
            share_value: share_value_cents,
            status: RepaymentPhaseStatus::Open,
            opened_at: None,
            closed_at: None,
            created: time::PrimitiveDateTime::new(
                datetime!(2025-01-01 0:00).date(),
                datetime!(2025-01-01 0:00).time(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn sample_entry(
        phase_id: Uuid,
        member_id: Uuid,
        share_count: i32,
        status: RepaymentEntryStatus,
        deleted: bool,
    ) -> RepaymentEntryEntity {
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            phase_id,
            member_id,
            share_count_to_pay_out: share_count,
            status,
            created: time::PrimitiveDateTime::new(
                datetime!(2025-01-01 0:00).date(),
                datetime!(2025-01-01 0:00).time(),
            ),
            deleted: if deleted {
                Some(time::PrimitiveDateTime::new(
                    datetime!(2025-01-02 0:00).date(),
                    datetime!(2025-01-02 0:00).time(),
                ))
            } else {
                None
            },
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_aggregate_single_open_entry() {
        let phase = sample_phase(12000); // 120,00 EUR pro Anteil
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 3, RepaymentEntryStatus::Open, false);
        let ctx = aggregate_for_member(&phase, &[e], member).expect("Some");
        assert_eq!(ctx.share_count, 3);
        assert_eq!(ctx.payout_amount, "360,00");
        assert_eq!(ctx.fiscal_year, 2025);
    }

    #[test]
    fn test_aggregate_multi_entry_sums_d13_04() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e1 = sample_entry(phase.id, member, 2, RepaymentEntryStatus::Open, false);
        let e2 = sample_entry(phase.id, member, 3, RepaymentEntryStatus::Contacted, false);
        let ctx = aggregate_for_member(&phase, &[e1, e2], member).expect("Some");
        assert_eq!(ctx.share_count, 5, "D-13-04: SUM aller relevanten Entries");
        assert_eq!(ctx.payout_amount, "600,00");
    }

    #[test]
    fn test_aggregate_filters_paid_out() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e1 = sample_entry(phase.id, member, 3, RepaymentEntryStatus::Open, false);
        let e2 = sample_entry(phase.id, member, 99, RepaymentEntryStatus::PaidOut, false);
        let ctx = aggregate_for_member(&phase, &[e1, e2], member).expect("Some");
        assert_eq!(ctx.share_count, 3, "PaidOut wird gefiltert");
    }

    #[test]
    fn test_aggregate_filters_soft_deleted() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e1 = sample_entry(phase.id, member, 99, RepaymentEntryStatus::Open, true);
        assert!(aggregate_for_member(&phase, &[e1], member).is_none());
    }

    #[test]
    fn test_aggregate_cross_member_isolation() {
        let phase = sample_phase(12000);
        let member_x = Uuid::new_v4();
        let member_y = Uuid::new_v4();
        let e1 = sample_entry(phase.id, member_y, 5, RepaymentEntryStatus::Open, false);
        // Call mit member_x — Entry fuer Y darf nicht zaehlen.
        assert!(aggregate_for_member(&phase, &[e1], member_x).is_none());
    }

    #[test]
    fn test_aggregate_contacted_included() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 2, RepaymentEntryStatus::Contacted, false);
        let ctx = aggregate_for_member(&phase, &[e], member).expect("Some");
        assert_eq!(ctx.share_count, 2);
    }

    #[test]
    fn test_aggregate_paid_out_excluded() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e_paid = sample_entry(phase.id, member, 99, RepaymentEntryStatus::PaidOut, false);
        assert!(aggregate_for_member(&phase, &[e_paid], member).is_none());
    }

    #[test]
    fn test_aggregate_empty_returns_none() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        assert!(aggregate_for_member(&phase, &[], member).is_none());
    }

    #[test]
    fn test_payout_amount_format_cents_zero_padded() {
        // 1 share × 105 cents = 105 cents = "1,05"
        let phase = sample_phase(105);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 1, RepaymentEntryStatus::Open, false);
        let ctx = aggregate_for_member(&phase, &[e], member).expect("Some");
        assert_eq!(
            ctx.payout_amount, "1,05",
            "Cent-Wert muss zero-padded sein (Phase 10 D-04)"
        );
    }

    #[test]
    fn test_payout_amount_no_euro_symbol_no_thousand_dot() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 1, RepaymentEntryStatus::Open, false);
        let ctx = aggregate_for_member(&phase, &[e], member).expect("Some");
        assert!(
            !ctx.payout_amount.contains('\u{20AC}'),
            "KEIN Euro-Symbol im payout_amount (Template fuegt es ein)"
        );
        assert!(
            !ctx.payout_amount.contains('.'),
            "KEIN Tausenderpunkt (Phase 10 D-04 deutsche Lokalisierung)"
        );
    }

    // ── Trait-aggregate-Wrapper-Tests + resolve-Tests ─────────────────────
    use async_trait::async_trait;
    use genossi_dao::repayment_entry::RepaymentEntryDao;
    use genossi_dao::repayment_phase::RepaymentPhaseDao;
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::ServiceError;
    use mockall::{mock, predicate::*};

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

    pub struct TestDeps;
    impl RepaymentContextResolverDeps for TestDeps {
        type Transaction = TestTransaction;
        type RepaymentPhaseDao = MockTestPhaseDao;
        type RepaymentEntryDao = MockTestEntryDao;
    }

    fn build_impl(
        phase_dao: MockTestPhaseDao,
        entry_dao: MockTestEntryDao,
    ) -> RepaymentContextResolverImpl<TestDeps> {
        RepaymentContextResolverImpl {
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
        }
    }

    /// Quick 260908-cjo: Pendant zu `build_impl` fuer die historische Impl.
    fn build_historic_impl(
        phase_dao: MockTestPhaseDao,
        entry_dao: MockTestEntryDao,
    ) -> HistoricRepaymentContextResolverImpl<TestDeps> {
        HistoricRepaymentContextResolverImpl {
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
        }
    }

    #[test]
    fn test_trait_aggregate_happy_path() {
        let r = build_impl(MockTestPhaseDao::new(), MockTestEntryDao::new());
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 3, RepaymentEntryStatus::Open, false);
        let ctx = r.aggregate(&phase, &[e], member).expect("Ok");
        assert_eq!(ctx.share_count, 3);
        assert_eq!(ctx.payout_amount, "360,00");
    }

    #[test]
    fn test_trait_aggregate_empty_returns_entity_not_found() {
        let r = build_impl(MockTestPhaseDao::new(), MockTestEntryDao::new());
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let err = r.aggregate(&phase, &[], member).unwrap_err();
        assert!(matches!(err, ServiceError::EntityNotFound(id) if id == member));
    }

    #[tokio::test]
    async fn test_resolve_happy_path() {
        let phase = sample_phase(12000);
        let phase_id = phase.id;
        let member_id = Uuid::new_v4();
        let entry = sample_entry(phase_id, member_id, 3, RepaymentEntryStatus::Open, false);

        let mut phase_dao = MockTestPhaseDao::new();
        let phase_clone = phase.clone();
        phase_dao
            .expect_find_by_id()
            .returning(move |_id, _tx| Ok(Some(phase_clone.clone())));

        let mut entry_dao = MockTestEntryDao::new();
        let entries: Arc<[RepaymentEntryEntity]> = vec![entry].into();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_pid, _tx| Ok(entries.clone()));

        let r = build_impl(phase_dao, entry_dao);
        let ctx = r
            .resolve(phase_id, member_id, TestTransaction)
            .await
            .expect("Ok");
        assert_eq!(ctx.share_count, 3);
        assert_eq!(ctx.payout_amount, "360,00");
        assert_eq!(ctx.fiscal_year, 2025);
    }

    #[tokio::test]
    async fn test_resolve_phase_not_found() {
        let phase_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao.expect_find_by_id().returning(|_id, _tx| Ok(None));

        // entry_dao darf nicht aufgerufen werden (kein expect).
        let entry_dao = MockTestEntryDao::new();

        let r = build_impl(phase_dao, entry_dao);
        let err = r
            .resolve(phase_id, member_id, TestTransaction)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ServiceError::EntityNotFound(id) if id == phase_id),
            "phase fehlt -> EntityNotFound(phase_id)"
        );
    }

    #[tokio::test]
    async fn test_resolve_no_relevant_entries_returns_entity_not_found_member() {
        let phase = sample_phase(12000);
        let phase_id = phase.id;
        let member_id = Uuid::new_v4();
        // Entry existiert, aber fuer ANDEREN Member.
        let other_member = Uuid::new_v4();
        let entry = sample_entry(phase_id, other_member, 3, RepaymentEntryStatus::Open, false);

        let mut phase_dao = MockTestPhaseDao::new();
        let phase_clone = phase.clone();
        phase_dao
            .expect_find_by_id()
            .returning(move |_id, _tx| Ok(Some(phase_clone.clone())));

        let mut entry_dao = MockTestEntryDao::new();
        let entries: Arc<[RepaymentEntryEntity]> = vec![entry].into();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_pid, _tx| Ok(entries.clone()));

        let r = build_impl(phase_dao, entry_dao);
        let err = r
            .resolve(phase_id, member_id, TestTransaction)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ServiceError::EntityNotFound(id) if id == member_id),
            "keine relevanten Entries -> EntityNotFound(member_id)"
        );
    }

    // ── Quick 260908-cjo: historische (status-agnostische) Aggregation ────
    // D-02: der Backfill rekonstruiert BEREITS VERSENDETE Mails; eine
    // abgeschlossene Phase (alle Eintraege PaidOut) ist dort der Normalfall.

    #[test]
    fn historic_aggregate_includes_paid_out_only() {
        // Die Produktionslage: abgeschlossene Phase, nur PaidOut-Eintraege.
        // Die strikte Aggregation liefert hier None (siehe
        // test_aggregate_paid_out_excluded) — die historische muss liefern.
        let phase = sample_phase(12000); // 120,00 EUR pro Anteil
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 5, RepaymentEntryStatus::PaidOut, false);

        let h = aggregate_for_member_historic(&phase, &[e], member).expect("Some");
        assert_eq!(h.context.share_count, 5);
        assert_eq!(h.context.payout_amount, "600,00");
        assert_eq!(h.context.fiscal_year, 2025);
        assert!(h.diverges_from_live, "PaidOut -> Divergenz zum Live-Pfad");
        assert_eq!(h.stale_status_count, 1);
        assert_eq!(h.entry_count, 1);
    }

    #[test]
    fn historic_aggregate_mixed_statuses_sums_all() {
        // Gegenstueck zu test_aggregate_filters_paid_out (oben), das fuer
        // denselben Input 3 behauptet. Die Differenz ist Absicht (D-02):
        // status-agnostisch statt "PaidOut zusaetzlich".
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e_open = sample_entry(phase.id, member, 2, RepaymentEntryStatus::Open, false);
        let e_paid = sample_entry(phase.id, member, 3, RepaymentEntryStatus::PaidOut, false);

        let h = aggregate_for_member_historic(&phase, &[e_open, e_paid], member).expect("Some");
        assert_eq!(h.context.share_count, 5, "D-02: ALLE Eintraege zaehlen");
        assert_eq!(h.context.payout_amount, "600,00");
        assert!(h.diverges_from_live);
        assert_eq!(h.stale_status_count, 1);
        assert_eq!(h.entry_count, 2);
    }

    #[test]
    fn historic_aggregate_open_only_equals_strict() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 3, RepaymentEntryStatus::Open, false);

        let strict = aggregate_for_member(&phase, std::slice::from_ref(&e), member).expect("Some");
        let h = aggregate_for_member_historic(&phase, &[e], member).expect("Some");

        assert_eq!(
            h.context, strict,
            "ohne PaidOut/stale Eintraege identisch zur strikten Aggregation"
        );
        assert!(!h.diverges_from_live);
        assert_eq!(h.stale_status_count, 0);
    }

    #[test]
    fn historic_aggregate_still_filters_soft_deleted() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 99, RepaymentEntryStatus::PaidOut, true);
        assert!(
            aggregate_for_member_historic(&phase, &[e], member).is_none(),
            "soft-deleted bleibt gefiltert (nur das Status-Praedikat faellt weg)"
        );
    }

    #[test]
    fn historic_aggregate_cross_member_isolation() {
        let phase = sample_phase(12000);
        let member_x = Uuid::new_v4();
        let member_y = Uuid::new_v4();
        let e = sample_entry(phase.id, member_y, 5, RepaymentEntryStatus::PaidOut, false);
        assert!(aggregate_for_member_historic(&phase, &[e], member_x).is_none());
    }

    #[test]
    fn historic_aggregate_empty_returns_none() {
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        assert!(aggregate_for_member_historic(&phase, &[], member).is_none());
    }

    #[test]
    fn historic_trait_aggregate_ok_for_paid_out_only() {
        // Die DAOs werden auf dem aggregate-Pfad nicht angefasst -> keine
        // Erwartungen noetig.
        let r = build_historic_impl(MockTestPhaseDao::new(), MockTestEntryDao::new());
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 5, RepaymentEntryStatus::PaidOut, false);

        let ctx = r.aggregate(&phase, &[e], member).expect("Ok");
        assert_eq!(ctx.share_count, 5);
        assert_eq!(ctx.payout_amount, "600,00");
        assert_eq!(ctx.fiscal_year, 2025);
    }

    #[test]
    fn strict_trait_aggregate_still_errs_for_paid_out_only() {
        // Regressionsgate fuer den Live-Pfad auf Aggregations-Ebene:
        // identischer Input, strikte Impl -> weiterhin EntityNotFound.
        let r = build_impl(MockTestPhaseDao::new(), MockTestEntryDao::new());
        let phase = sample_phase(12000);
        let member = Uuid::new_v4();
        let e = sample_entry(phase.id, member, 5, RepaymentEntryStatus::PaidOut, false);

        let err = r.aggregate(&phase, &[e], member).unwrap_err();
        assert!(matches!(err, ServiceError::EntityNotFound(id) if id == member));
    }

    #[tokio::test]
    async fn historic_resolve_loads_and_aggregates() {
        let phase = sample_phase(12000);
        let phase_id = phase.id;
        let member_id = Uuid::new_v4();
        let entry = sample_entry(phase_id, member_id, 5, RepaymentEntryStatus::PaidOut, false);

        let mut phase_dao = MockTestPhaseDao::new();
        let phase_clone = phase.clone();
        phase_dao
            .expect_find_by_id()
            .returning(move |_id, _tx| Ok(Some(phase_clone.clone())));

        let mut entry_dao = MockTestEntryDao::new();
        let entries: Arc<[RepaymentEntryEntity]> = vec![entry].into();
        entry_dao
            .expect_find_by_phase_id()
            .returning(move |_pid, _tx| Ok(entries.clone()));

        let r = build_historic_impl(phase_dao, entry_dao);
        let ctx = r
            .resolve(phase_id, member_id, TestTransaction)
            .await
            .expect("Ok");
        assert_eq!(ctx.share_count, 5);
        assert_eq!(ctx.payout_amount, "600,00");
        assert_eq!(ctx.fiscal_year, 2025, "fiscal_year ist definiert");
    }

    #[tokio::test]
    async fn historic_resolve_phase_not_found() {
        // Belegt, dass load_phase_and_entries fuer beide Impls identisch
        // fehlerbehaftet ist (vgl. test_resolve_phase_not_found).
        let phase_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();

        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao.expect_find_by_id().returning(|_id, _tx| Ok(None));

        // entry_dao darf nicht aufgerufen werden (kein expect).
        let entry_dao = MockTestEntryDao::new();

        let r = build_historic_impl(phase_dao, entry_dao);
        let err = r
            .resolve(phase_id, member_id, TestTransaction)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ServiceError::EntityNotFound(id) if id == phase_id),
            "phase fehlt -> EntityNotFound(phase_id)"
        );
    }
}
