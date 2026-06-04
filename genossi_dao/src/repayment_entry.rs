use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

/// Status-Lifecycle eines RepaymentEntry-Eintrags.
///
/// **D-05:** Alle drei Varianten existieren von Anfang an (verhindert Phase-9-
/// DB-Schema-Migration). Phase 8 toggled nur `Open ↔ Contacted` (D-06 bidirektional).
/// `PaidOut` ist Phase-9-Zielzustand (PAYO-04, einseitig final).
///
/// **D-01-Konvention (analog Phase 7):** Statusstrings in Englisch; Frontend i18n.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RepaymentEntryStatus {
    #[default]
    Open,
    Contacted,
    PaidOut,
}

impl RepaymentEntryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepaymentEntryStatus::Open => "Open",
            RepaymentEntryStatus::Contacted => "Contacted",
            RepaymentEntryStatus::PaidOut => "PaidOut",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Open" => Ok(RepaymentEntryStatus::Open),
            "Contacted" => Ok(RepaymentEntryStatus::Contacted),
            "PaidOut" => Ok(RepaymentEntryStatus::PaidOut),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown repayment entry status: {}",
                s
            )))),
        }
    }
}

/// Auditpflichtige RepaymentEntry-Entity.
///
/// **Audit-Felder (frozen):** `member_id`, `phase_id`, `share_count_to_pay_out`, `status`
/// in genau dieser Reihenfolge (siehe `Auditable`-Impl). Reihenfolge-Änderung würde
/// die Hash-Chain historischer Audit-Einträge brechen (Phase-7-Plan-01-Lektion).
///
/// **Metadaten-Konvention:** `id`, `version`, `created`, `deleted` sind NICHT in
/// `audit_fields` enthalten (Auditable-Konvention).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentEntryEntity {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phase_id: Uuid,
    pub share_count_to_pay_out: i32,
    pub status: RepaymentEntryStatus,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl crate::auditable::Auditable for RepaymentEntryEntity {
    fn entity_type() -> &'static str {
        "repayment_entry"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        // FROZEN ORDER (Hash-Chain-Konsistenz, Phase-7-Lektion):
        // member_id, phase_id, share_count_to_pay_out, status
        vec![
            ("member_id", Some(self.member_id.to_string())),
            ("phase_id", Some(self.phase_id.to_string())),
            (
                "share_count_to_pay_out",
                Some(self.share_count_to_pay_out.to_string()),
            ),
            ("status", Some(self.status.as_str().to_string())),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait RepaymentEntryDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &RepaymentEntryEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &RepaymentEntryEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<RepaymentEntryEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<RepaymentEntryEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    /// Liefert alle aktiven Eintraege einer Phase (`deleted IS NULL`).
    /// Wird in Plan 03 fuer Listing (`GET /api/repayment-entry?phase_id=`) und
    /// in Plan 04 fuer die Close-Validation (PHAS-03, D-13) verwendet.
    async fn find_by_phase_id(
        &self,
        phase_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let filtered: Vec<RepaymentEntryEntity> = all_entities
            .iter()
            .filter(|e| e.phase_id == phase_id && e.deleted.is_none())
            .cloned()
            .collect();
        Ok(filtered.into())
    }

    /// Liefert alle aktiven Eintraege einer Member-Phase-Kombination.
    ///
    /// Foundation fuer Phase-16-Sum-Check + Auto-Fill-Skip-Pattern
    /// (PITFALLS Kat 1). SQLite-Impl ueberschreibt mit SQL-WHERE-Klausel
    /// zur Performance-Skalierung; Default-Impl filtert in-memory ueber
    /// `dump_all`.
    ///
    /// **Mockall-Hinweis:** `#[automock]` ueberschreibt Default-Impls,
    /// daher muessen Service-Unit-Tests `.expect_find_by_member_and_phase()`
    /// explizit setzen.
    async fn find_by_member_and_phase(
        &self,
        member_id: Uuid,
        phase_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let filtered: Vec<RepaymentEntryEntity> = all_entities
            .iter()
            .filter(|e| e.member_id == member_id && e.phase_id == phase_id && e.deleted.is_none())
            .cloned()
            .collect();
        Ok(filtered.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;

    fn make_repayment_entry() -> RepaymentEntryEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 5,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_repayment_entry_status_roundtrip() {
        for variant in [
            RepaymentEntryStatus::Open,
            RepaymentEntryStatus::Contacted,
            RepaymentEntryStatus::PaidOut,
        ] {
            let s = variant.as_str();
            let parsed = RepaymentEntryStatus::from_str(s).expect("roundtrip parses");
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_repayment_entry_status_strings_are_english() {
        // D-05 / D-01-Konvention (analog Phase 7): Statusstrings auf Englisch;
        // Frontend uebersetzt via i18n.
        assert_eq!(RepaymentEntryStatus::Open.as_str(), "Open");
        assert_eq!(RepaymentEntryStatus::Contacted.as_str(), "Contacted");
        assert_eq!(RepaymentEntryStatus::PaidOut.as_str(), "PaidOut");
        // Round-trip via die literalen englischen Strings muss erfolgreich sein.
        assert_eq!(
            RepaymentEntryStatus::from_str("Open").unwrap(),
            RepaymentEntryStatus::Open
        );
        assert_eq!(
            RepaymentEntryStatus::from_str("Contacted").unwrap(),
            RepaymentEntryStatus::Contacted
        );
        assert_eq!(
            RepaymentEntryStatus::from_str("PaidOut").unwrap(),
            RepaymentEntryStatus::PaidOut
        );
    }

    #[test]
    fn test_repayment_entry_status_invalid_string() {
        // Deutsche Strings duerfen NICHT parsen (D-05, analog Phase 7 T-07-01-05).
        let err = RepaymentEntryStatus::from_str("offen");
        assert!(matches!(err, Err(DaoError::ParseError(_))));

        let err2 = RepaymentEntryStatus::from_str("angeschrieben");
        assert!(matches!(err2, Err(DaoError::ParseError(_))));

        let err3 = RepaymentEntryStatus::from_str("ausgezahlt");
        assert!(matches!(err3, Err(DaoError::ParseError(_))));

        let err4 = RepaymentEntryStatus::from_str("");
        assert!(matches!(err4, Err(DaoError::ParseError(_))));
    }

    #[test]
    fn test_repayment_entry_status_default_is_open() {
        // D-05: Eintragsstatus startet immer in Open. Konsistenz mit Migration-Default.
        assert_eq!(RepaymentEntryStatus::default(), RepaymentEntryStatus::Open);
    }

    #[test]
    fn test_auditable_entity_type_is_repayment_entry() {
        assert_eq!(RepaymentEntryEntity::entity_type(), "repayment_entry");
    }

    #[test]
    fn test_auditable_fields_count_and_excludes_metadata() {
        // FROZEN-Test: Reihenfolge der audit_fields ist Hash-Chain-relevant
        // und darf sich NICHT aendern. Aenderungen wuerden historische
        // Audit-Eintraege brechen (Phase-7-Plan-01-Lektion).
        let entity = make_repayment_entry();
        let fields = entity.audit_fields();
        assert_eq!(
            fields.len(),
            4,
            "audit_fields must contain exactly 4 entries \
             (member_id, phase_id, share_count_to_pay_out, status)"
        );

        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            field_names,
            vec!["member_id", "phase_id", "share_count_to_pay_out", "status"],
            "audit_fields order is FROZEN — changing it breaks audit hash-chain history"
        );

        // Lifecycle / metadata fields MUSST NICHT in audit_fields sein
        // (Auditable-Konvention — siehe RepaymentPhaseEntity-Vorlage Z. 67-93):
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_status_change() {
        let old = make_repayment_entry();
        let mut new = old.clone();
        new.status = RepaymentEntryStatus::Contacted;

        let changes = old.diff(&new);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert_eq!(
            changes.len(),
            1,
            "exactly one field changed (status: Open -> Contacted)"
        );
        assert_eq!(names, vec!["status"]);
    }

    #[test]
    fn test_auditable_audit_fields_member_id_first_phase_id_second() {
        // Frozen-Order-Detail: member_id ist Index 0, phase_id ist Index 1.
        let entity = make_repayment_entry();
        let fields = entity.audit_fields();
        assert_eq!(fields[0].0, "member_id");
        assert_eq!(fields[0].1, Some(entity.member_id.to_string()));
        assert_eq!(fields[1].0, "phase_id");
        assert_eq!(fields[1].1, Some(entity.phase_id.to_string()));
        assert_eq!(fields[2].0, "share_count_to_pay_out");
        assert_eq!(fields[3].0, "status");
    }

    /// Hand-rolled Test-Stub fuer die Default-Impl-Verifikation.
    ///
    /// `#[automock]` ueberschreibt Default-Impls (Pitfall 2 / Phase-3-Plan-03-Lektion),
    /// deshalb implementieren wir das Trait minimal selbst. Nur `dump_all` ist
    /// non-trivial; `create`/`update` sind unimplemented (werden vom Default-Impl
    /// nicht aufgerufen).
    struct TestRepaymentEntryDao {
        entries: Vec<RepaymentEntryEntity>,
    }

    #[async_trait]
    impl RepaymentEntryDao for TestRepaymentEntryDao {
        type Transaction = crate::MockTransaction;

        async fn dump_all(
            &self,
            _tx: Self::Transaction,
        ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
            Ok(self.entries.clone().into())
        }

        async fn create(
            &self,
            _entity: &RepaymentEntryEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            unimplemented!("not used by default-impl test")
        }

        async fn update(
            &self,
            _entity: &RepaymentEntryEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            unimplemented!("not used by default-impl test")
        }
    }

    #[tokio::test]
    async fn test_find_by_member_and_phase_default_impl_filters_correctly() {
        // Foundation fuer Phase-16-Sum-Check + Auto-Fill-Skip-Pattern
        // (PITFALLS Kat 1). Default-Impl filtert in-memory via dump_all
        // auf (member_id, phase_id) AND deleted IS NULL.
        let member_a = Uuid::new_v4();
        let member_b = Uuid::new_v4();
        let phase_x = Uuid::new_v4();
        let phase_y = Uuid::new_v4();

        let date = time::Date::from_calendar_date(2026, time::Month::May, 30).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);

        // 4 Eintraege:
        // e1: (member_A, phase_X, deleted=None) -> MATCH
        // e2: (member_A, phase_Y, deleted=None) -> phase differs -> exclude
        // e3: (member_B, phase_X, deleted=None) -> member differs -> exclude
        // e4: (member_A, phase_X, deleted=Some(...)) -> deleted -> exclude
        let e1 = RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: member_a,
            phase_id: phase_x,
            share_count_to_pay_out: 2,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        };
        let e2 = RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: member_a,
            phase_id: phase_y,
            share_count_to_pay_out: 1,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        };
        let e3 = RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: member_b,
            phase_id: phase_x,
            share_count_to_pay_out: 3,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        };
        let e4 = RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id: member_a,
            phase_id: phase_x,
            share_count_to_pay_out: 1,
            status: RepaymentEntryStatus::Open,
            created: datetime,
            deleted: Some(datetime),
            version: Uuid::new_v4(),
        };

        let dao = TestRepaymentEntryDao {
            entries: vec![e1.clone(), e2, e3, e4],
        };

        let mock_tx = crate::MockTransaction::new();
        let result = dao
            .find_by_member_and_phase(member_a, phase_x, mock_tx)
            .await
            .expect("default-impl must succeed");

        assert_eq!(
            result.len(),
            1,
            "exactly one entry matches (member_A, phase_X) and is not deleted"
        );
        assert_eq!(result[0].id, e1.id, "the surviving entry must be e1");
        assert_eq!(result[0].member_id, member_a);
        assert_eq!(result[0].phase_id, phase_x);
        assert!(result[0].deleted.is_none());

        // Empty-input edge case
        let empty_dao = TestRepaymentEntryDao { entries: vec![] };
        let empty_tx = crate::MockTransaction::new();
        let empty_result = empty_dao
            .find_by_member_and_phase(member_a, phase_x, empty_tx)
            .await
            .expect("empty dump_all -> empty result");
        assert_eq!(empty_result.len(), 0);
    }
}
