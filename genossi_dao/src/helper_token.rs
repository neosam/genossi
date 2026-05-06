use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use uuid::Uuid;

use crate::DaoError;

/// HelperTokenEntity — D-01: mirrors the helper_token table.
///
/// Lifecycle status (Open/Used/Revoked) is **derived** from the columns
/// `used_at` and `revoked_at` (D-02), no separate status column.
///
/// `token_hash` holds SHA256(code) (D-11). It is the canonical input to the
/// atomic redeem path and stays the only column the Auditable impl is allowed
/// to leak (it does NOT — D-06 excludes it).
///
/// `code` (Option<Arc<str>>) is the plain-text Crockford-Base32 code,
/// persisted per ADR-2026-05-06 so the Vorstand can re-display the QR card
/// at any time. Pre-update rows have `code = None`; the frontend renders a
/// "revoke + recreate" hint for those. The Auditable impl excludes `code`
/// (see `audit_fields()` below) to avoid creating a parallel code store in
/// the audit hash chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperTokenEntity {
    pub id: Uuid,
    pub assembly_id: Uuid,
    pub memo: Arc<str>,
    pub token_hash: Arc<str>,
    /// ADR-2026-05-06: plain-text code persisted for re-display. NULL for
    /// rows created before the migration — frontend treats those as "revoke
    /// and recreate to obtain a QR card".
    pub code: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub used_at: Option<time::PrimitiveDateTime>,
    pub session_id: Option<Arc<str>>,
    pub revoked_at: Option<time::PrimitiveDateTime>,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl crate::auditable::Auditable for HelperTokenEntity {
    fn entity_type() -> &'static str {
        "helper_token"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    // SECURITY: `code` is plain-text persistence per Vorstand-decision 2026-05-06;
    // deliberately NOT audited (would create a parallel code store in audit_log).
    // The audit hash chain stays free of redeemable material — the helper_token
    // row in the live DB is the single, authoritative store of the plain-text
    // code. `token_hash` (D-06) remains excluded for the same reason.
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        // WR-08: do NOT use `unwrap_or_default()` — a silent empty string in
        // the audit log is forensically useless. Log the failure and substitute
        // a sentinel so any breakage is at least visible.
        let format_dt = |dt: &time::PrimitiveDateTime| {
            dt.assume_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|err| {
                    tracing::error!(
                        error = ?err,
                        entity = "helper_token",
                        "Failed to format datetime for audit field"
                    );
                    "<invalid datetime>".to_string()
                })
        };
        // D-06: NO token_hash (no pre-image leakage in audit log).
        // ADR-2026-05-06: NO code (no parallel plain-text store in audit log).
        // Includes assembly_id, memo, used_at, session_id, revoked_at —
        // sufficient for forensic review.
        vec![
            ("assembly_id", Some(self.assembly_id.to_string())),
            ("memo", Some(self.memo.to_string())),
            ("used_at", self.used_at.as_ref().map(format_dt)),
            (
                "session_id",
                self.session_id.as_ref().map(|s| s.to_string()),
            ),
            ("revoked_at", self.revoked_at.as_ref().map(format_dt)),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait HelperTokenDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction)
        -> Result<Arc<[HelperTokenEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &HelperTokenEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &HelperTokenEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[HelperTokenEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<HelperTokenEntity> = all_entities
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
    ) -> Result<Option<HelperTokenEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    /// Atomic one-time-use redeem (D-25, RESEARCH §Pattern 1).
    ///
    /// Implementations MUST use a single SQL statement of the form:
    ///   `UPDATE helper_token SET used_at = ?
    ///    WHERE token_hash = ?
    ///      AND used_at IS NULL
    ///      AND revoked_at IS NULL
    ///      AND deleted IS NULL
    ///    RETURNING id, assembly_id`
    ///
    /// Returns `Some((token_id, assembly_id))` on success, `None` if 0 rows
    /// matched. The caller MUST run `lookup_status()` after a `None` result
    /// to discriminate between 404 (unknown), 410 (used), 403 (revoked).
    async fn atomic_redeem(
        &self,
        token_hash: &str,
        used_at: time::PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<Option<(Uuid, Uuid)>, DaoError>;

    /// Sets `session_id` on a redeemed token; called immediately after the
    /// session row has been created (Pitfall 3: two-step UPDATE inside the
    /// same TX as `atomic_redeem`).
    async fn set_session_id(
        &self,
        token_id: Uuid,
        session_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Differential status-lookup for a 0-row redeem (D-24).
    ///
    /// Returns `Some((used_at, revoked_at))` if a row exists for this
    /// `token_hash` (regardless of state), `None` if the `token_hash` was
    /// unknown (or the row was soft-deleted).
    async fn lookup_status(
        &self,
        token_hash: &str,
        tx: Self::Transaction,
    ) -> Result<
        Option<(
            Option<time::PrimitiveDateTime>,
            Option<time::PrimitiveDateTime>,
        )>,
        DaoError,
    >;

    /// Listing for the Vorstand UI (D-21). Filters `deleted IS NULL` and
    /// returns rows ordered by `created DESC`.
    async fn all_for_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[HelperTokenEntity]>, DaoError>;

    /// D-12: Cascade-Discovery for AssemblyServiceImpl::close_assembly (Phase 3).
    /// Returns all currently-bound helper-session ids for the given assembly.
    /// Filters out null session_ids (revoked or never-redeemed tokens) and
    /// soft-deleted token rows. Order is implementation-defined but stable
    /// within a single SQLite snapshot.
    async fn list_session_ids_for_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Vec<Arc<str>>, DaoError>;

    /// Phase 4 Plan 01 (D-06): Reverse-lookup the assembly_id for a given
    /// helper-session id. Returns `Ok(Some(assembly_id))` if a non-deleted
    /// helper_token row carries this `session_id`, `Ok(None)` if no such row
    /// exists. Used by the public `/api/helper/session` and `/api/helper/logout`
    /// endpoints to confirm that a generic `app_session` cookie is in fact a
    /// Helper-Session (not an admin/OIDC session) before honouring it.
    async fn find_assembly_id_for_session(
        &self,
        session_id: &str,
        tx: Self::Transaction,
    ) -> Result<Option<Uuid>, DaoError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;

    fn make_token() -> HelperTokenEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 3).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        HelperTokenEntity {
            id: Uuid::new_v4(),
            assembly_id: Uuid::new_v4(),
            memo: Arc::from("Anna"),
            token_hash: Arc::from("deadbeefdeadbeefdeadbeefdeadbeef"),
            code: Some(Arc::from("ABC1234567")),
            created: datetime,
            used_at: None,
            session_id: None,
            revoked_at: None,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_entity_type_is_helper_token() {
        assert_eq!(HelperTokenEntity::entity_type(), "helper_token");
        let entity = make_token();
        assert_eq!(entity.entity_id(), entity.id);
    }

    #[test]
    fn test_auditable_fields_excludes_token_hash() {
        let entity = make_token();
        let fields = entity.audit_fields();

        assert_eq!(
            fields.len(),
            5,
            "audit_fields must contain exactly 5 entries (D-06)"
        );

        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            field_names,
            vec![
                "assembly_id",
                "memo",
                "used_at",
                "session_id",
                "revoked_at"
            ]
        );

        // D-06 explicit guard: token_hash MUST NOT appear in the audit log
        // (no pre-image leakage of the SHA256 hash).
        assert!(
            !field_names.contains(&"token_hash"),
            "D-06: token_hash must be excluded from audit_fields"
        );

        // ADR-2026-05-06 explicit guard: code MUST NOT appear in the audit
        // log either — the helper_token row is the only allowed store of
        // the plain-text code (no parallel store in audit_log).
        assert!(
            !field_names.contains(&"code"),
            "ADR-2026-05-06: code must be excluded from audit_fields"
        );

        // Auditable convention — id/version/created/deleted are lifecycle
        // metadata and never go into audit_fields.
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_audit_fields_capture_does_not_include_code_change() {
        // ADR-2026-05-06: even if `code` is rotated/cleared, the audit log
        // must remain free of plain-text material. Diff between two entities
        // that differ only in `code` must produce zero changes.
        let entity_a = make_token();
        let mut entity_b = entity_a.clone();
        entity_b.code = Some(Arc::from("ZZZZZZZZZZ"));

        let changes = entity_a.diff(&entity_b);
        assert!(
            changes.is_empty(),
            "code-only mutation must not produce audit diff entries; got {:?}",
            changes
        );
    }

    #[test]
    fn test_audit_fields_capture_used_at_and_revoked_at_changes() {
        // D-08 leaves redeem/revoke unaudited in Phase 2, but the diff must
        // still detect lifecycle column changes if a future phase opts in.
        let now = make_token().created;
        let mut old = make_token();
        old.used_at = None;
        old.revoked_at = None;
        old.code = None;

        let mut new = old.clone();
        new.used_at = Some(now);
        new.revoked_at = Some(now);
        new.session_id = Some(Arc::from("session-xyz"));

        let changes = old.diff(&new);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert!(names.contains(&"used_at"));
        assert!(names.contains(&"revoked_at"));
        assert!(names.contains(&"session_id"));
        assert_eq!(changes.len(), 3);
    }
}
