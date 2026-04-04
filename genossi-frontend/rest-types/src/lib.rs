use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

// Custom serialization module for ISO8601 datetime format
mod iso8601_datetime {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::PrimitiveDateTime;
    use time::format_description::well_known::Iso8601;

    pub fn serialize<S>(
        datetime: &Option<PrimitiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match datetime {
            Some(dt) => {
                let formatted = dt
                    .assume_utc()
                    .format(&Iso8601::DEFAULT)
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PrimitiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

mod iso8601_date {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Date;

    pub fn serialize<S>(date: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => {
                let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
                let formatted = d.format(&format).map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => {
                let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
                Date::parse(&s, &format)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            None => Ok(None),
        }
    }
}

mod iso8601_date_required {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Date;

    pub fn serialize<S>(date: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
        let formatted = date.format(&format).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
        Date::parse(&s, &format).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserTO {
    pub username: String,
    pub roles: Vec<String>,
    pub privileges: Vec<String>,
    pub claims: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberTO {
    pub id: Option<Uuid>,
    pub member_number: i64,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub comment: Option<String>,
    pub street: Option<String>,
    pub house_number: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    pub join_date: time::Date,
    pub shares_at_joining: i32,
    pub current_shares: i32,
    pub current_balance: i64,
    #[serde(default)]
    pub action_count: i32,
    #[serde(default)]
    pub migrated: bool,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_date::serialize",
        deserialize_with = "iso8601_date::deserialize",
        default
    )]
    pub exit_date: Option<time::Date>,
    pub bank_account: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl MemberTO {
    pub fn is_active(&self, reference_date: &time::Date) -> bool {
        if self.join_date > *reference_date {
            return false;
        }
        match self.exit_date {
            Some(exit) => exit > *reference_date,
            None => true,
        }
    }

    pub fn exited_in_year(&self, reference_date: &time::Date) -> bool {
        self.exit_date
            .map(|d| d.year() == reference_date.year())
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionTypeTO {
    Eintritt,
    Austritt,
    Todesfall,
    Aufstockung,
    Verkauf,
    UebertragungEmpfang,
    UebertragungAbgabe,
}

impl ActionTypeTO {
    pub fn all() -> &'static [ActionTypeTO] {
        &[
            ActionTypeTO::Eintritt,
            ActionTypeTO::Austritt,
            ActionTypeTO::Todesfall,
            ActionTypeTO::Aufstockung,
            ActionTypeTO::Verkauf,
            ActionTypeTO::UebertragungEmpfang,
            ActionTypeTO::UebertragungAbgabe,
        ]
    }

    pub fn is_status_action(&self) -> bool {
        matches!(
            self,
            ActionTypeTO::Eintritt | ActionTypeTO::Austritt | ActionTypeTO::Todesfall
        )
    }

    pub fn is_transfer(&self) -> bool {
        matches!(
            self,
            ActionTypeTO::UebertragungEmpfang | ActionTypeTO::UebertragungAbgabe
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ActionTypeTO::Eintritt => "Eintritt",
            ActionTypeTO::Austritt => "Austritt",
            ActionTypeTO::Todesfall => "Todesfall",
            ActionTypeTO::Aufstockung => "Aufstockung",
            ActionTypeTO::Verkauf => "Verkauf",
            ActionTypeTO::UebertragungEmpfang => "UebertragungEmpfang",
            ActionTypeTO::UebertragungAbgabe => "UebertragungAbgabe",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Eintritt" => Some(ActionTypeTO::Eintritt),
            "Austritt" => Some(ActionTypeTO::Austritt),
            "Todesfall" => Some(ActionTypeTO::Todesfall),
            "Aufstockung" => Some(ActionTypeTO::Aufstockung),
            "Verkauf" => Some(ActionTypeTO::Verkauf),
            "UebertragungEmpfang" => Some(ActionTypeTO::UebertragungEmpfang),
            "UebertragungAbgabe" => Some(ActionTypeTO::UebertragungAbgabe),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberActionTO {
    pub id: Option<Uuid>,
    pub member_id: Uuid,
    pub action_type: ActionTypeTO,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    pub date: time::Date,
    pub shares_change: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_member_id: Option<Uuid>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_date::serialize",
        deserialize_with = "iso8601_date::deserialize",
        default
    )]
    pub effective_date: Option<time::Date>,
    pub comment: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentTypeTO {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
}

impl DocumentTypeTO {
    pub fn all() -> &'static [DocumentTypeTO] {
        &[
            DocumentTypeTO::JoinDeclaration,
            DocumentTypeTO::JoinConfirmation,
            DocumentTypeTO::ShareIncrease,
            DocumentTypeTO::Other,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentTypeTO::JoinDeclaration => "join_declaration",
            DocumentTypeTO::JoinConfirmation => "join_confirmation",
            DocumentTypeTO::ShareIncrease => "share_increase",
            DocumentTypeTO::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_declaration" => Some(DocumentTypeTO::JoinDeclaration),
            "join_confirmation" => Some(DocumentTypeTO::JoinConfirmation),
            "share_increase" => Some(DocumentTypeTO::ShareIncrease),
            "other" => Some(DocumentTypeTO::Other),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberDocumentTO {
    pub id: Option<Uuid>,
    pub member_id: Uuid,
    pub document_type: String,
    pub description: Option<String>,
    pub file_name: String,
    pub mime_type: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationStatusTO {
    pub member_id: Uuid,
    pub status: String,
    pub expected_shares: i32,
    pub actual_shares: i32,
    pub expected_action_count: i32,
    pub actual_action_count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResultTO {
    pub member_number_gaps: Vec<i64>,
    pub unmatched_transfers: Vec<UnmatchedTransferTO>,
    #[serde(default)]
    pub shares_mismatches: Vec<SharesMismatchTO>,
    #[serde(default)]
    pub missing_entry_actions: Vec<MissingEntryActionTO>,
    #[serde(default)]
    pub exit_date_mismatches: Vec<ExitDateMismatchTO>,
    #[serde(default)]
    pub active_members_no_shares: Vec<ActiveMemberNoSharesTO>,
    #[serde(default)]
    pub duplicate_member_numbers: Vec<DuplicateMemberNumberTO>,
    #[serde(default)]
    pub exited_members_with_shares: Vec<ExitedMemberWithSharesTO>,
    #[serde(default)]
    pub migrated_flag_mismatches: Vec<MigratedFlagMismatchTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnmatchedTransferTO {
    pub action_id: Uuid,
    pub member_id: Uuid,
    pub member_number: i64,
    pub action_type: ActionTypeTO,
    pub transfer_member_id: Uuid,
    pub transfer_member_number: i64,
    pub shares_change: i32,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    pub date: time::Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharesMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub expected: i32,
    pub actual: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissingEntryActionTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub actual_count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitDateMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub has_exit_date: bool,
    pub has_austritt_action: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveMemberNoSharesTO {
    pub member_id: Uuid,
    pub member_number: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicateMemberNumberTO {
    pub member_number: i64,
    pub member_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitedMemberWithSharesTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub current_shares: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigratedFlagMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub flag_value: bool,
    pub computed_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPreferenceTO {
    pub id: Option<Uuid>,
    pub key: Option<String>,
    pub value: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month};

    fn make_member(join_date: Date, exit_date: Option<Date>) -> MemberTO {
        MemberTO {
            id: None,
            member_number: 1,
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date,
            bank_account: None,
            created: None,
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn test_is_active_no_exit_date() {
        let ref_date = Date::from_calendar_date(2026, Month::April, 1).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            None,
        );
        assert!(member.is_active(&ref_date));
    }

    #[test]
    fn test_is_active_exit_date_in_future() {
        let ref_date = Date::from_calendar_date(2026, Month::April, 1).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2027, Month::January, 1).unwrap()),
        );
        assert!(member.is_active(&ref_date));
    }

    #[test]
    fn test_is_active_exit_date_in_past() {
        let ref_date = Date::from_calendar_date(2026, Month::April, 1).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2026, Month::March, 1).unwrap()),
        );
        assert!(!member.is_active(&ref_date));
    }

    #[test]
    fn test_is_active_join_date_in_future() {
        let ref_date = Date::from_calendar_date(2026, Month::April, 1).unwrap();
        let member = make_member(
            Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            None,
        );
        assert!(!member.is_active(&ref_date));
    }

    #[test]
    fn test_is_active_exit_date_equals_reference() {
        let ref_date = Date::from_calendar_date(2026, Month::April, 1).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2026, Month::April, 1).unwrap()),
        );
        assert!(!member.is_active(&ref_date));
    }

    #[test]
    fn test_exited_in_year_matching_year() {
        let ref_date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2026, Month::December, 31).unwrap()),
        );
        assert!(member.exited_in_year(&ref_date));
    }

    #[test]
    fn test_exited_in_year_different_year() {
        let ref_date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2025, Month::March, 1).unwrap()),
        );
        assert!(!member.exited_in_year(&ref_date));
    }

    #[test]
    fn test_exited_in_year_no_exit_date() {
        let ref_date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            None,
        );
        assert!(!member.exited_in_year(&ref_date));
    }

    #[test]
    fn test_exited_in_year_and_still_active() {
        // Member exits Dec 31, 2026 — still active on June 15, 2026 but exited_in_year matches
        let ref_date = Date::from_calendar_date(2026, Month::June, 15).unwrap();
        let member = make_member(
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Some(Date::from_calendar_date(2026, Month::December, 31).unwrap()),
        );
        assert!(member.is_active(&ref_date));
        assert!(member.exited_in_year(&ref_date));
    }
}
