use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditFieldChange {
    pub field_name: &'static str,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

pub trait Auditable {
    fn entity_type() -> &'static str;
    fn entity_id(&self) -> Uuid;
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)>;

    fn diff(&self, other: &Self) -> Vec<AuditFieldChange> {
        let old_fields = self.audit_fields();
        let new_fields = other.audit_fields();
        old_fields
            .into_iter()
            .zip(new_fields)
            .filter_map(|((name, old_val), (_, new_val))| {
                if old_val != new_val {
                    Some(AuditFieldChange {
                        field_name: name,
                        old_value: old_val,
                        new_value: new_val,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                ("name", Some(self.name.clone())),
                ("email", self.email.clone()),
            ]
        }
    }

    #[test]
    fn test_diff_with_changes() {
        let id = Uuid::new_v4();
        let old = TestEntity {
            id,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let new = TestEntity {
            id,
            name: "Bob".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let changes = old.diff(&new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "name");
        assert_eq!(changes[0].old_value, Some("Alice".to_string()));
        assert_eq!(changes[0].new_value, Some("Bob".to_string()));
    }

    #[test]
    fn test_diff_no_changes() {
        let id = Uuid::new_v4();
        let old = TestEntity {
            id,
            name: "Alice".to_string(),
            email: None,
        };
        let new = TestEntity {
            id,
            name: "Alice".to_string(),
            email: None,
        };
        let changes = old.diff(&new);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_diff_option_none_to_some() {
        let id = Uuid::new_v4();
        let old = TestEntity {
            id,
            name: "Alice".to_string(),
            email: None,
        };
        let new = TestEntity {
            id,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let changes = old.diff(&new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "email");
        assert!(changes[0].old_value.is_none());
        assert_eq!(changes[0].new_value, Some("alice@example.com".to_string()));
    }

    #[test]
    fn test_diff_option_some_to_none() {
        let id = Uuid::new_v4();
        let old = TestEntity {
            id,
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
        };
        let new = TestEntity {
            id,
            name: "Alice".to_string(),
            email: None,
        };
        let changes = old.diff(&new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "email");
        assert_eq!(changes[0].old_value, Some("alice@example.com".to_string()));
        assert!(changes[0].new_value.is_none());
    }
}
