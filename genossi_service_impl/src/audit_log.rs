use genossi_dao::audit_log::AuditLogEntry;
use genossi_dao::auditable::{AuditFieldChange, Auditable};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

pub fn compute_entry_hash(
    timestamp: &str,
    user_id: &str,
    process: &str,
    transaction_id: &str,
    entity_type: &str,
    entity_id: &str,
    action: &str,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    prev_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(timestamp);
    hasher.update("|");
    hasher.update(user_id);
    hasher.update("|");
    hasher.update(process);
    hasher.update("|");
    hasher.update(transaction_id);
    hasher.update("|");
    hasher.update(entity_type);
    hasher.update("|");
    hasher.update(entity_id);
    hasher.update("|");
    hasher.update(action);
    hasher.update("|");
    hasher.update(field_name);
    hasher.update("|");
    hasher.update(old_value);
    hasher.update("|");
    hasher.update(new_value);
    hasher.update("|");
    hasher.update(prev_hash);
    format!("{:x}", hasher.finalize())
}

fn value_to_hash_str(v: &Option<Arc<str>>) -> String {
    match v {
        Some(s) => s.to_string(),
        None => String::new(),
    }
}

pub fn build_audit_entries<E: Auditable>(
    action: &str,
    changes: &[AuditFieldChange],
    entity: &E,
    user_id: &str,
    process: &str,
    prev_hash: &str,
    uuid_fn: &mut dyn FnMut() -> Uuid,
) -> Vec<AuditLogEntry> {
    let now = time::OffsetDateTime::now_utc();
    let timestamp = time::PrimitiveDateTime::new(now.date(), now.time());
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    let timestamp_str = timestamp.assume_utc().format(format).unwrap_or_default();
    let transaction_id = uuid_fn();
    let entity_type = E::entity_type();
    let entity_id = entity.entity_id();

    let mut sorted_changes: Vec<&AuditFieldChange> = changes.iter().collect();
    sorted_changes.sort_by_key(|c| c.field_name);

    let mut entries = Vec::new();
    let mut current_prev_hash = prev_hash.to_string();

    for change in sorted_changes {
        let old_value: Option<Arc<str>> = change.old_value.as_deref().map(Arc::from);
        let new_value: Option<Arc<str>> = change.new_value.as_deref().map(Arc::from);

        let entry_hash = compute_entry_hash(
            &timestamp_str,
            user_id,
            process,
            &transaction_id.to_string(),
            entity_type,
            &entity_id.to_string(),
            action,
            change.field_name,
            &value_to_hash_str(&old_value),
            &value_to_hash_str(&new_value),
            &current_prev_hash,
        );

        entries.push(AuditLogEntry {
            id: uuid_fn(),
            timestamp,
            user_id: Arc::from(user_id),
            process: Arc::from(process),
            transaction_id,
            entity_type: Arc::from(entity_type),
            entity_id,
            action: Arc::from(action),
            field_name: Arc::from(change.field_name),
            old_value,
            new_value,
            prev_hash: Arc::from(current_prev_hash.as_str()),
            entry_hash: Arc::from(entry_hash.as_str()),
        });

        current_prev_hash = entry_hash;
    }

    entries
}

pub fn build_create_entries<E: Auditable>(
    entity: &E,
    user_id: &str,
    process: &str,
    prev_hash: &str,
    uuid_fn: &mut dyn FnMut() -> Uuid,
) -> Vec<AuditLogEntry> {
    let fields = entity.audit_fields();
    let changes: Vec<AuditFieldChange> = fields
        .into_iter()
        .filter(|(_, v)| v.is_some())
        .map(|(name, value)| AuditFieldChange {
            field_name: name,
            old_value: None,
            new_value: value,
        })
        .collect();
    build_audit_entries(
        "create", &changes, entity, user_id, process, prev_hash, uuid_fn,
    )
}

pub fn build_snapshot_entries<E: Auditable>(
    entity: &E,
    user_id: &str,
    process: &str,
    prev_hash: &str,
    uuid_fn: &mut dyn FnMut() -> Uuid,
) -> Vec<AuditLogEntry> {
    let fields = entity.audit_fields();
    let changes: Vec<AuditFieldChange> = fields
        .into_iter()
        .map(|(name, value)| AuditFieldChange {
            field_name: name,
            old_value: None,
            new_value: value,
        })
        .collect();
    build_audit_entries(
        "snapshot", &changes, entity, user_id, process, prev_hash, uuid_fn,
    )
}

pub fn build_update_entries<E: Auditable>(
    old: &E,
    new: &E,
    user_id: &str,
    process: &str,
    prev_hash: &str,
    uuid_fn: &mut dyn FnMut() -> Uuid,
) -> Vec<AuditLogEntry> {
    let changes = old.diff(new);
    if changes.is_empty() {
        return Vec::new();
    }
    build_audit_entries(
        "update", &changes, new, user_id, process, prev_hash, uuid_fn,
    )
}

pub fn build_delete_entries<E: Auditable>(
    entity: &E,
    user_id: &str,
    process: &str,
    prev_hash: &str,
    uuid_fn: &mut dyn FnMut() -> Uuid,
) -> Vec<AuditLogEntry> {
    let fields = entity.audit_fields();
    let changes: Vec<AuditFieldChange> = fields
        .into_iter()
        .map(|(name, value)| AuditFieldChange {
            field_name: name,
            old_value: value,
            new_value: None,
        })
        .collect();
    build_audit_entries(
        "delete", &changes, entity, user_id, process, prev_hash, uuid_fn,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    pub entry_id: Uuid,
    pub expected_hash: String,
    pub actual_hash: String,
}

pub fn verify_chain(entries: &[AuditLogEntry]) -> Vec<BrokenLink> {
    let mut broken = Vec::new();
    let mut expected_prev_hash = String::new();

    for entry in entries {
        if entry.prev_hash.as_ref() != expected_prev_hash {
            broken.push(BrokenLink {
                entry_id: entry.id,
                expected_hash: expected_prev_hash.clone(),
                actual_hash: entry.prev_hash.to_string(),
            });
        }

        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        let timestamp_str = entry
            .timestamp
            .assume_utc()
            .format(format)
            .unwrap_or_default();

        let computed = compute_entry_hash(
            &timestamp_str,
            &entry.user_id,
            &entry.process,
            &entry.transaction_id.to_string(),
            &entry.entity_type,
            &entry.entity_id.to_string(),
            &entry.action,
            &entry.field_name,
            &value_to_hash_str(&entry.old_value),
            &value_to_hash_str(&entry.new_value),
            &entry.prev_hash,
        );

        if computed != entry.entry_hash.as_ref() {
            broken.push(BrokenLink {
                entry_id: entry.id,
                expected_hash: computed,
                actual_hash: entry.entry_hash.to_string(),
            });
        }

        expected_prev_hash = entry.entry_hash.to_string();
    }

    broken
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::auditable::AuditFieldChange;

    struct TestEntity {
        id: Uuid,
        name: String,
        email: Option<String>,
    }

    impl Auditable for TestEntity {
        fn entity_type() -> &'static str {
            "test"
        }
        fn entity_id(&self) -> Uuid {
            self.id
        }
        fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
            vec![
                ("email", self.email.clone()),
                ("name", Some(self.name.clone())),
            ]
        }
    }

    fn fixed_uuid(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    #[test]
    fn test_hash_computation_is_deterministic() {
        let h1 = compute_entry_hash("t", "u", "p", "tx", "et", "eid", "a", "f", "o", "n", "ph");
        let h2 = compute_entry_hash("t", "u", "p", "tx", "et", "eid", "a", "f", "o", "n", "ph");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_changes_with_different_input() {
        let h1 = compute_entry_hash("t", "u", "p", "tx", "et", "eid", "a", "f", "o", "n", "ph");
        let h2 = compute_entry_hash("t", "u", "p", "tx", "et", "eid", "a", "f", "X", "n", "ph");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_build_create_entries() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let mut counter: u8 = 10;
        let entries = build_create_entries(&entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        assert_eq!(entries.len(), 2);
        // Sorted alphabetically: email, name
        assert_eq!(entries[0].field_name.as_ref(), "email");
        assert_eq!(entries[1].field_name.as_ref(), "name");
        assert_eq!(entries[0].action.as_ref(), "create");
        assert!(entries[0].old_value.is_none());
        assert_eq!(entries[0].new_value.as_deref(), Some("alice@example.com"));
        // Chain: first entry prev_hash is empty, second links to first
        assert_eq!(entries[0].prev_hash.as_ref(), "");
        assert_eq!(
            entries[1].prev_hash.as_ref(),
            entries[0].entry_hash.as_ref()
        );
        // Same transaction_id
        assert_eq!(entries[0].transaction_id, entries[1].transaction_id);
    }

    #[test]
    fn test_build_create_entries_skips_none_fields() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: None,
        };
        let mut counter: u8 = 10;
        let entries = build_create_entries(&entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].field_name.as_ref(), "name");
    }

    #[test]
    fn test_build_update_entries_only_changed() {
        let old = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let new = TestEntity {
            id: fixed_uuid(1),
            name: "Bob".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let mut counter: u8 = 10;
        let entries = build_update_entries(&old, &new, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].field_name.as_ref(), "name");
        assert_eq!(entries[0].old_value.as_deref(), Some("Alice"));
        assert_eq!(entries[0].new_value.as_deref(), Some("Bob"));
    }

    #[test]
    fn test_build_update_entries_no_changes() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: None,
        };
        let mut counter: u8 = 10;
        let entries = build_update_entries(&entity, &entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        assert!(entries.is_empty());
    }

    #[test]
    fn test_build_delete_entries() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let mut counter: u8 = 10;
        let entries = build_delete_entries(&entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        // Delete logs all fields (including None ones)
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action.as_ref(), "delete");
        // All new_values should be None
        for entry in &entries {
            assert!(entry.new_value.is_none());
        }
    }

    #[test]
    fn test_verify_chain_intact() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("a@b.com".to_string()),
        };
        let mut counter: u8 = 10;
        let entries = build_create_entries(&entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        let broken = verify_chain(&entries);
        assert!(broken.is_empty());
    }

    #[test]
    fn test_verify_chain_detects_tampering() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("a@b.com".to_string()),
        };
        let mut counter: u8 = 10;
        let mut entries = build_create_entries(&entity, "user1", "svc", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        // Tamper with the first entry's new_value
        entries[0].new_value = Some(Arc::from("tampered@evil.com"));
        let broken = verify_chain(&entries);
        assert!(!broken.is_empty());
    }

    #[test]
    fn test_verify_chain_empty() {
        let broken = verify_chain(&[]);
        assert!(broken.is_empty());
    }

    #[test]
    fn test_entries_sorted_alphabetically() {
        let changes = vec![
            AuditFieldChange {
                field_name: "zebra",
                old_value: Some("a".to_string()),
                new_value: Some("b".to_string()),
            },
            AuditFieldChange {
                field_name: "alpha",
                old_value: Some("c".to_string()),
                new_value: Some("d".to_string()),
            },
        ];
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Test".to_string(),
            email: None,
        };
        let mut counter: u8 = 10;
        let entries = build_audit_entries("update", &changes, &entity, "u", "p", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        assert_eq!(entries[0].field_name.as_ref(), "alpha");
        assert_eq!(entries[1].field_name.as_ref(), "zebra");
    }

    #[test]
    fn test_build_snapshot_entries_includes_none_fields() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: None,
        };
        let mut counter: u8 = 10;
        let entries = build_snapshot_entries(&entity, "SYSTEM", "audit-snapshot", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        // Snapshot logs ALL fields including None
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action.as_ref(), "snapshot");
        // email is None but still logged
        let email_entry = entries
            .iter()
            .find(|e| e.field_name.as_ref() == "email")
            .unwrap();
        assert!(email_entry.old_value.is_none());
        assert!(email_entry.new_value.is_none());
        // name has a value
        let name_entry = entries
            .iter()
            .find(|e| e.field_name.as_ref() == "name")
            .unwrap();
        assert!(name_entry.old_value.is_none());
        assert_eq!(name_entry.new_value.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_verify_chain_with_snapshot() {
        let entity = TestEntity {
            id: fixed_uuid(1),
            name: "Alice".to_string(),
            email: Some("a@b.com".to_string()),
        };
        let mut counter: u8 = 10;
        let entries = build_snapshot_entries(&entity, "SYSTEM", "audit-snapshot", "", &mut || {
            counter += 1;
            fixed_uuid(counter)
        });
        let broken = verify_chain(&entries);
        assert!(broken.is_empty());
    }
}
