//! v1.2 Mitgliedschaft-Anpassungen — Service-Trait (Foundation fuer Phase 15-17).
//!
//! Phase 15 definiert `cancel_membership` + `increase_shares`. Phase 16 ergaenzt
//! `partial_repayment`, Phase 17 ergaenzt `transfer_shares` (D-15-13 inkrementelles Wachsen).

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::member::Member;
use crate::member_action::MemberAction;
use crate::permission::Authentication;
use crate::ServiceError;

/// Service-Trait fuer v1.2-Mitgliedschaft-Anpassungen (PERM-01: admin-only).
///
/// Beide Phase-15-Methoden geben `(MemberAction, Member)` zurueck, damit das Frontend
/// nach dem Commit ohne zusaetzlichen GET-Round-Trip rendern kann (D-15-11).
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MembershipAdjustService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Kuendigt eine Mitgliedschaft via `MemberAction::Austritt` (CANC-01..05).
    async fn cancel_membership(
        &self,
        member_id: Uuid,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError>;

    /// Stockt die Anteile eines aktiven Mitglieds atomar auf (UPGD-01..04).
    async fn increase_shares(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberAction, Member), ServiceError>;
}
