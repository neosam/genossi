#[cfg(test)]
mod integration_tests {
    use crate::i18n::{I18n, Key, Locale};
    use crate::state::{AuthInfo, Config};
    use std::rc::Rc;
    use time::{Date, Month};

    #[test]
    fn test_authenticated_user_workflow() {
        // Test complete user authentication flow
        let auth_info = AuthInfo {
            user: "manager@company.com".into(),
            privileges: Rc::new([
                "shift_planner".into(),
                "employee_manager".into(),
                "report_viewer".into(),
            ]),
            authenticated: true,
        };

        // User should be able to access shift planning
        assert!(auth_info.has_privilege("shift_planner"));
        assert!(auth_info.authenticated);

        // User should be able to manage employees
        assert!(auth_info.has_privilege("employee_manager"));

        // User should be able to view reports
        assert!(auth_info.has_privilege("report_viewer"));

        // User should NOT be able to access admin functions
        assert!(!auth_info.has_privilege("super_admin"));
    }

    #[test]
    fn test_date_formatting_with_i18n() {
        // Test date formatting with different locales
        let date = Date::from_calendar_date(2024, Month::April, 15).unwrap();

        let i18n_en = I18n::new(Locale::En);
        let i18n_de = I18n::new(Locale::De);

        // Format dates in different locales
        let date_en = i18n_en.format_date(date);
        let date_de = i18n_de.format_date(date);

        // All should be non-empty and different formats
        assert!(!date_en.is_empty());
        assert!(!date_de.is_empty());

        // German format should use dots
        assert!(date_de.contains('.'));
        // English format should use dashes
        assert!(date_en.contains('-'));
    }

    #[test]
    fn test_config_management_workflow() {
        // Test configuration management workflow
        let config = Config {
            backend: "http://localhost:3000".into(),
            application_title: "Inventurly".into(),
            is_prod: false,
            env_short_description: "DEV".into(),
            show_vacation: false,
        };

        // Test configuration values
        assert_eq!(config.backend.as_ref(), "http://localhost:3000");
        assert_eq!(config.application_title.as_ref(), "Inventurly");
        assert!(!config.is_prod);
        assert_eq!(config.env_short_description.as_ref(), "DEV");

        // Test production configuration
        let prod_config = Config {
            backend: "https://api.inventurly.com".into(),
            application_title: "Inventurly".into(),
            is_prod: true,
            env_short_description: "PROD".into(),
            show_vacation: false,
        };

        assert!(prod_config.is_prod);
        assert!(prod_config.backend.starts_with("https://"));
    }

    #[test]
    fn test_multilingual_application_flow() {
        // Test application workflow with different languages
        let config = Config {
            backend: "http://localhost:3000".into(),
            application_title: "Inventurly".into(),
            is_prod: false,
            env_short_description: "DEV".into(),
            show_vacation: false,
        };

        let auth_info = AuthInfo {
            user: "german.user@company.de".into(),
            privileges: Rc::new(["inventory_manager".into()]),
            authenticated: true,
        };

        // Test with German locale
        let i18n = I18n::new(Locale::De);

        // Basic UI elements should be translated
        let save_text = i18n.t(Key::Save);
        let cancel_text = i18n.t(Key::Cancel);
        let products_text = i18n.t(Key::Products);

        assert!(!save_text.is_empty());
        assert!(!cancel_text.is_empty());
        assert!(!products_text.is_empty());

        // Test date formatting for German locale
        let date = Date::from_calendar_date(2024, Month::March, 15).unwrap();
        let formatted_date = i18n.format_date(date);

        assert!(!formatted_date.is_empty());
        assert!(formatted_date.contains("15")); // Day should be present
        assert!(formatted_date.contains(".")); // German format uses dots

        // User should still be able to access inventory management regardless of locale
        assert!(auth_info.has_privilege("inventory_manager"));
        assert!(!config.is_prod); // Development environment
    }

    #[test]
    fn test_production_vs_development_workflow() {
        // Test differences between production and development configurations
        let dev_config = Config {
            backend: "http://localhost:3000".into(),
            application_title: "Inventurly DEV".into(),
            is_prod: false,
            env_short_description: "DEV".into(),
            show_vacation: false,
        };

        let prod_config = Config {
            backend: "https://api.inventurly.com".into(),
            application_title: "Inventurly".into(),
            is_prod: true,
            env_short_description: "PROD".into(),
            show_vacation: false,
        };

        // Development should use HTTP
        assert!(!dev_config.is_prod);
        assert!(dev_config.backend.starts_with("http://"));

        // Production should use HTTPS
        assert!(prod_config.is_prod);
        assert!(prod_config.backend.starts_with("https://"));

        // Both should support the same core functionality
        let auth_info = AuthInfo {
            user: "production.user@company.com".into(),
            privileges: Rc::new(["inventory_manager".into()]),
            authenticated: true,
        };

        assert!(auth_info.has_privilege("inventory_manager"));
    }

    #[test]
    fn test_date_boundary_edge_cases() {
        // Test date calculations around year boundaries
        let last_day_2023 = Date::from_calendar_date(2023, Month::December, 31).unwrap();
        let first_day_2024 = Date::from_calendar_date(2024, Month::January, 1).unwrap();

        // The dates should be consecutive
        let duration = first_day_2024 - last_day_2023;
        assert_eq!(duration.whole_days(), 1);

        // Test with I18n formatting
        let i18n_en = I18n::new(Locale::En);
        let i18n_de = I18n::new(Locale::De);
        
        let formatted_2023_en = i18n_en.format_date(last_day_2023);
        let formatted_2024_en = i18n_en.format_date(first_day_2024);
        let formatted_2023_de = i18n_de.format_date(last_day_2023);
        let formatted_2024_de = i18n_de.format_date(first_day_2024);

        assert!(formatted_2023_en.contains("2023"));
        assert!(formatted_2024_en.contains("2024"));
        assert!(formatted_2023_de.contains("2023"));
        assert!(formatted_2024_de.contains("2024"));
    }

    #[test]
    fn test_error_recovery_workflow() {
        use crate::error::{result_handler, InventurlyError};

        // Simulate a workflow that can recover from errors
        fn attempt_operation(should_fail: bool) -> Result<String, InventurlyError> {
            if should_fail {
                // Create a real error using invalid date
                let invalid_date = time::Date::from_calendar_date(2024, time::Month::February, 30);
                match invalid_date {
                    Err(time_error) => Err(InventurlyError::TimeComponentRange(time_error)),
                    Ok(_) => Ok("unexpected".to_string()),
                }
            } else {
                Ok("Operation successful".to_string())
            }
        }

        // First attempt fails
        let first_attempt = attempt_operation(true);
        let first_result = result_handler(first_attempt);
        assert_eq!(first_result, None);

        // Retry succeeds
        let second_attempt = attempt_operation(false);
        let second_result = result_handler(second_attempt);
        assert_eq!(second_result, Some("Operation successful".to_string()));
    }

    #[test]
    fn test_user_privilege_escalation_prevention() {
        // Test that privilege checks prevent unauthorized access
        let regular_user = AuthInfo {
            user: "regular.user@company.com".into(),
            privileges: Rc::new(["report_viewer".into()]),
            authenticated: true,
        };

        let admin_user = AuthInfo {
            user: "admin@company.com".into(),
            privileges: Rc::new([
                "report_viewer".into(),
                "inventory_manager".into(),
                "product_manager".into(),
                "admin".into(),
            ]),
            authenticated: true,
        };

        // Regular user should have limited access
        assert!(regular_user.has_privilege("report_viewer"));
        assert!(!regular_user.has_privilege("inventory_manager"));
        assert!(!regular_user.has_privilege("product_manager"));
        assert!(!regular_user.has_privilege("admin"));

        // Admin user should have full access
        assert!(admin_user.has_privilege("report_viewer"));
        assert!(admin_user.has_privilege("inventory_manager"));
        assert!(admin_user.has_privilege("product_manager"));
        assert!(admin_user.has_privilege("admin"));

        // Neither should have non-existent privileges
        assert!(!regular_user.has_privilege("super_secret_privilege"));
        assert!(!admin_user.has_privilege("non_existent_privilege"));
    }
}
