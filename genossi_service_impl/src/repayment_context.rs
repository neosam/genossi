//! Phase 13 D-13-04 / D-13-10: Impl von RepaymentContextResolver.
//!
//! TDD RED-Phase: Tests existieren, Implementation noch nicht.

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
        phase_dao
            .expect_find_by_id()
            .returning(|_id, _tx| Ok(None));

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
}
