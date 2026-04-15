/// Macro that performs a DAO create and logs all non-None fields to the audit log.
///
/// Expects `self` to have `audit_log_dao` and `uuid_service` fields.
#[macro_export]
macro_rules! audited_create {
    ($self:expr, $dao:expr, $entity:expr, $process:expr, $user_id:expr, $tx:expr) => {{
        use genossi_dao::audit_log::AuditLogDao;

        // Perform the DAO create
        $dao.create($entity, $process, $tx.clone()).await?;

        // Get the current latest hash
        let prev_hash = $self
            .audit_log_dao
            .get_latest_hash($tx.clone())
            .await?
            .unwrap_or_default();

        // Build audit entries for all non-None fields
        let entries = $crate::audit_log::build_create_entries(
            $entity,
            $user_id,
            $process,
            &prev_hash,
            &mut || uuid::Uuid::new_v4(),
        );

        // Write audit entries
        if !entries.is_empty() {
            $self
                .audit_log_dao
                .create_entries(&entries, $tx.clone())
                .await?;
        }
    }};
}

/// Macro that loads the old entity, performs a DAO update, and logs only changed fields.
///
/// Expects `self` to have `audit_log_dao` and `uuid_service` fields.
#[macro_export]
macro_rules! audited_update {
    ($self:expr, $dao:expr, $entity_id:expr, $new_entity:expr, $process:expr, $user_id:expr, $tx:expr) => {{
        use genossi_dao::audit_log::AuditLogDao;

        // Load the old entity
        let old = $dao
            .find_by_id($entity_id, $tx.clone())
            .await?
            .ok_or(genossi_service::ServiceError::EntityNotFound($entity_id))?;

        // Perform the DAO update
        $dao.update($new_entity, $process, $tx.clone()).await?;

        // Get the current latest hash
        let prev_hash = $self
            .audit_log_dao
            .get_latest_hash($tx.clone())
            .await?
            .unwrap_or_default();

        // Build audit entries for changed fields only
        let entries = $crate::audit_log::build_update_entries(
            &old,
            $new_entity,
            $user_id,
            $process,
            &prev_hash,
            &mut || uuid::Uuid::new_v4(),
        );

        // Write audit entries
        if !entries.is_empty() {
            $self
                .audit_log_dao
                .create_entries(&entries, $tx.clone())
                .await?;
        }
    }};
}

/// Macro that loads the entity, sets deleted timestamp, performs update, and logs all fields as delete.
///
/// Expects `self` to have `audit_log_dao` and `uuid_service` fields.
#[macro_export]
macro_rules! audited_delete {
    ($self:expr, $dao:expr, $entity_id:expr, $process:expr, $user_id:expr, $tx:expr) => {{
        use genossi_dao::audit_log::AuditLogDao;

        // Load the entity
        let mut entity = $dao
            .find_by_id($entity_id, $tx.clone())
            .await?
            .ok_or(genossi_service::ServiceError::EntityNotFound($entity_id))?;

        // Set the deleted timestamp
        let now = time::OffsetDateTime::now_utc();
        entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        // Perform the DAO update (soft delete)
        $dao.update(&entity, $process, $tx.clone()).await?;

        // Get the current latest hash
        let prev_hash = $self
            .audit_log_dao
            .get_latest_hash($tx.clone())
            .await?
            .unwrap_or_default();

        // Build audit entries for all fields (delete action)
        let entries = $crate::audit_log::build_delete_entries(
            &entity,
            $user_id,
            $process,
            &prev_hash,
            &mut || uuid::Uuid::new_v4(),
        );

        // Write audit entries
        if !entries.is_empty() {
            $self
                .audit_log_dao
                .create_entries(&entries, $tx.clone())
                .await?;
        }
    }};
}
