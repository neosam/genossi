#[cfg(test)]
mod date_tests {
    use time::{Date, Month, Weekday};

    #[test]
    fn test_date_creation() {
        let date = Date::from_calendar_date(2024, Month::January, 1).unwrap();
        
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 1);
        assert_eq!(date.weekday(), Weekday::Monday);
    }

    #[test]
    fn test_date_arithmetic() {
        let start_date = Date::from_calendar_date(2024, Month::January, 1).unwrap();
        let duration = time::Duration::days(6);
        let end_date = start_date + duration;

        assert_eq!(end_date.day(), 7);
        assert_eq!(end_date.weekday(), Weekday::Sunday);
    }

    #[test]
    fn test_date_formatting() {
        let date = Date::from_calendar_date(2024, Month::April, 15).unwrap();
        
        // Test that date can be formatted
        let formatted = format!("{}-{:02}-{:02}", date.year(), date.month() as u8, date.day());
        assert_eq!(formatted, "2024-04-15");
    }

    #[test]
    fn test_date_weekday_calculation() {
        // Test that weekday calculation works correctly
        let monday = Date::from_calendar_date(2024, Month::April, 1).unwrap();
        
        // Add days to test each weekday
        for i in 0..7 {
            let test_date = monday + time::Duration::days(i);
            let weekday = test_date.weekday();
            
            // Verify weekday progression
            match i {
                0 => assert_eq!(weekday, Weekday::Monday),
                1 => assert_eq!(weekday, Weekday::Tuesday),
                2 => assert_eq!(weekday, Weekday::Wednesday),
                3 => assert_eq!(weekday, Weekday::Thursday),
                4 => assert_eq!(weekday, Weekday::Friday),
                5 => assert_eq!(weekday, Weekday::Saturday),
                6 => assert_eq!(weekday, Weekday::Sunday),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_date_year_boundary() {
        let last_day_2023 = Date::from_calendar_date(2023, Month::December, 31).unwrap();
        let first_day_2024 = Date::from_calendar_date(2024, Month::January, 1).unwrap();

        // Test year boundary crossing
        assert_eq!(last_day_2023.year(), 2023);
        assert_eq!(first_day_2024.year(), 2024);
        
        let duration = first_day_2024 - last_day_2023;
        assert_eq!(duration.whole_days(), 1);
    }

    #[test]
    fn test_date_month_boundaries() {
        // Test end of February in leap year
        let feb_28_2024 = Date::from_calendar_date(2024, Month::February, 28).unwrap();
        let feb_29_2024 = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        let mar_1_2024 = Date::from_calendar_date(2024, Month::March, 1).unwrap();

        assert_eq!(feb_28_2024.month(), Month::February);
        assert_eq!(feb_29_2024.month(), Month::February);
        assert_eq!(mar_1_2024.month(), Month::March);

        let duration = mar_1_2024 - feb_29_2024;
        assert_eq!(duration.whole_days(), 1);
    }

    #[test]
    fn test_date_invalid_dates() {
        // Test that invalid dates return errors
        let invalid_date = Date::from_calendar_date(2024, Month::February, 30);
        assert!(invalid_date.is_err());

        let invalid_month = Date::from_calendar_date(2024, Month::November, 31);
        assert!(invalid_month.is_err());
    }

    #[test]
    fn test_date_ordinal() {
        let jan_1 = Date::from_calendar_date(2024, Month::January, 1).unwrap();
        let dec_31 = Date::from_calendar_date(2024, Month::December, 31).unwrap();

        // 2024 is a leap year, so it has 366 days
        assert_eq!(jan_1.ordinal(), 1);
        assert_eq!(dec_31.ordinal(), 366);
    }
}