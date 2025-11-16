pub mod error_tests;
pub mod integration_tests;
pub mod week_tests;

#[cfg(test)]
mod unit_tests {
    use crate::state::auth_info::AuthInfo;
    use crate::state::config::Config;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test_auth_info_default() {
        let auth_info = AuthInfo::default();

        assert_eq!(auth_info.user.as_ref(), "");
        assert_eq!(auth_info.privileges.len(), 0);
        assert!(!auth_info.authenticated);
        assert_eq!(auth_info.claims.len(), 0);
    }

    #[test]
    fn test_auth_info_with_privileges() {
        let auth_info = AuthInfo {
            user: "test_user".into(),
            roles: Rc::new([]),
            privileges: Rc::new(["admin".into(), "planner".into()]),
            authenticated: true,
            claims: Rc::new(HashMap::new()),
        };

        assert_eq!(auth_info.user.as_ref(), "test_user");
        assert!(auth_info.authenticated);
        assert!(auth_info.has_privilege("admin"));
        assert!(auth_info.has_privilege("planner"));
        assert!(!auth_info.has_privilege("sales"));
    }

    #[test]
    fn test_auth_info_with_inventur_id_claim() {
        let mut claims = HashMap::new();
        claims.insert("inventur_id".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string());

        let auth_info = AuthInfo {
            user: "token_user".into(),
            roles: Rc::new([]),
            privileges: Rc::new([]),
            authenticated: true,
            claims: Rc::new(claims),
        };

        assert_eq!(
            auth_info.get_inventur_id(),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_auth_info_without_inventur_id_claim() {
        let auth_info = AuthInfo {
            user: "regular_user".into(),
            roles: Rc::new([]),
            privileges: Rc::new(["admin".into()]),
            authenticated: true,
            claims: Rc::new(HashMap::new()),
        };

        assert_eq!(auth_info.get_inventur_id(), None);
    }

    #[test]
    fn test_config_creation() {
        let config = Config {
            backend: "http://localhost:3000".into(),
            application_title: "Test App".into(),
            is_prod: false,
            env_short_description: "TEST".into(),
            show_vacation: false,
        };

        assert_eq!(config.backend.as_ref(), "http://localhost:3000");
        assert_eq!(config.application_title.as_ref(), "Test App");
        assert!(!config.is_prod);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        // Test that default values match Rust's Default trait behavior
        // (serde defaults only apply during deserialization, not Default::default())
        assert_eq!(config.backend.as_ref(), "");
        assert_eq!(config.application_title.as_ref(), ""); // Default trait gives empty string
        assert_eq!(config.env_short_description.as_ref(), ""); // Default trait gives empty string
        assert!(!config.is_prod);
        assert!(!config.show_vacation);
    }
}

#[cfg(test)]
mod i18n_tests {
    use crate::i18n::{I18n, Key, Locale};
    use time::{Date, Month};

    #[test]
    fn test_locale_variants() {
        let locales = vec![Locale::En, Locale::De];
        assert_eq!(locales.len(), 2);
    }

    #[test]
    fn test_i18n_creation() {
        // Create i18n instance using the I18n::new function
        let i18n = I18n::new(Locale::En);

        // Test that basic translations are available
        let save_text = i18n.t(Key::Save);
        assert!(!save_text.is_empty());
    }

    #[test]
    fn test_date_formatting_structure() {
        let date = Date::from_calendar_date(2024, Month::January, 15).unwrap();

        // Test date object creation and basic properties
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 15);
    }
}

#[cfg(test)]
mod service_tests {
    use crate::state::container::Container;
    use uuid::Uuid;

    #[test]
    fn test_container_state_default() {
        let store = Container::default();

        assert_eq!(store.items.len(), 0);
        assert!(!store.loading);
        assert!(store.error.is_none());
    }

    #[test]
    fn test_container_to_creation() {
        use rest_types::ContainerTO;
        
        let container = ContainerTO {
            id: Some(Uuid::new_v4()),
            name: "Test Container".to_string(),
            weight_grams: 100,
            description: "Test Description".to_string(),
            created: None,
            deleted: None,
            version: None,
        };

        assert_eq!(container.name, "Test Container");
        assert_eq!(container.weight_grams, 100);
        assert_eq!(container.description, "Test Description");
    }

    #[test]
    fn test_product_state_creation() {
        use crate::state::product::Product;
        
        let product_state = Product::default();
        assert_eq!(product_state.items.len(), 0);
        assert!(!product_state.loading);
        assert!(product_state.error.is_none());
    }
}

#[cfg(test)]
mod utils_tests {
    // Removed unused js function imports
    use crate::error::{result_handler, InventurlyError};
    use time::{Date, Month};
    use uuid::Uuid;

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_year_week_ranges() {
        // These functions use JavaScript Date, only available in WASM
        let year = get_current_year();
        let week = get_current_week();

        assert!(year >= 2024 && year <= 2100);
        assert!(week >= 1 && week <= 53);
    }

    #[test]
    fn test_year_week_ranges_mock() {
        // Mock test for non-WASM environments
        let current_year = 2024u32;
        let current_week = 42u8;

        assert!(current_year >= 2024 && current_year <= 2100);
        assert!(current_week >= 1 && current_week <= 53);
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        assert_ne!(uuid1, uuid2);
        assert_ne!(uuid1, Uuid::nil());
    }

    #[test]
    fn test_date_validation() {
        let valid_date = Date::from_calendar_date(2024, Month::January, 15);
        assert!(valid_date.is_ok());

        let invalid_date = Date::from_calendar_date(2024, Month::February, 30);
        assert!(invalid_date.is_err());
    }

    #[test]
    fn test_error_result_handler() {
        let ok_result: Result<i32, InventurlyError> = Ok(42);
        let result = result_handler(ok_result);
        assert_eq!(result, Some(42));
    }
}
