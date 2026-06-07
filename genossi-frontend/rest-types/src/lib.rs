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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalutationTO {
    Herr,
    Frau,
    Firma,
}

impl SalutationTO {
    pub fn all() -> &'static [SalutationTO] {
        &[
            SalutationTO::Herr,
            SalutationTO::Frau,
            SalutationTO::Firma,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SalutationTO::Herr => "Herr",
            SalutationTO::Frau => "Frau",
            SalutationTO::Firma => "Firma",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Herr" => Some(SalutationTO::Herr),
            "Frau" => Some(SalutationTO::Frau),
            "Firma" => Some(SalutationTO::Firma),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemberStatusTO {
    Normal,
    FehlerhaftErfasst,
}

impl MemberStatusTO {
    pub fn all() -> &'static [MemberStatusTO] {
        &[MemberStatusTO::Normal, MemberStatusTO::FehlerhaftErfasst]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemberStatusTO::Normal => "Normal",
            MemberStatusTO::FehlerhaftErfasst => "FehlerhaftErfasst",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Normal" => Some(MemberStatusTO::Normal),
            "FehlerhaftErfasst" => Some(MemberStatusTO::FehlerhaftErfasst),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MemberStatusTO::Normal => "Normal",
            MemberStatusTO::FehlerhaftErfasst => "Fehlerhaft erfasst",
        }
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, MemberStatusTO::Normal)
    }
}

impl Default for MemberStatusTO {
    fn default() -> Self {
        MemberStatusTO::Normal
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberTO {
    pub id: Option<Uuid>,
    pub member_number: i64,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    pub title: Option<String>,
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
    #[serde(default)]
    pub status: MemberStatusTO,
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
        if !self.status.is_normal() {
            return false;
        }
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
    Note,
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
            ActionTypeTO::Note,
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

    pub fn needs_shares_input(&self) -> bool {
        matches!(
            self,
            ActionTypeTO::Aufstockung
                | ActionTypeTO::Verkauf
                | ActionTypeTO::UebertragungEmpfang
                | ActionTypeTO::UebertragungAbgabe
        )
    }

    pub fn negates_shares(&self) -> bool {
        matches!(
            self,
            ActionTypeTO::Verkauf | ActionTypeTO::UebertragungAbgabe
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
            ActionTypeTO::Note => "Note",
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
            "Note" => Some(ActionTypeTO::Note),
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
        default,
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// ─── Phase 18 ─── Membership-Adjust DTOs (Phase 15/16/17 Request/Response-Shapes) ────
// Frontend-Kopie von genossi_rest_types/src/lib.rs OHNE utoipa::ToSchema + OHNE iso8601_date_required.
// time::Date Default-Serde (Feature `serde-human-readable`) liefert `YYYY-MM-DD`
// (matched Backend-iso8601_date_required-Format).
// Landmine L-2 Mitigation.

/// Phase 14 D-14-12 — DSGVO-konformer Slim-TO fuer Transfer-Recipients
/// (whitelist 7 Felder, keine Email/PII).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberSlimTO {
    pub id: Uuid,
    pub member_number: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
}

/// Phase 15 D-15-11 — Cancel-Membership-Request-Body fuer `POST /api/members/{id}/cancel`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelMembershipRequestTO {
    pub willensbekundung_date: time::Date,
}

/// Phase 15 D-15-15 — Increase-Shares-Request-Body fuer `POST /api/members/{id}/increase-shares`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncreaseSharesRequestTO {
    pub willensbekundung_date: time::Date,
    pub shares: i32,
}

/// Phase 15 D-15-11 / D-15-15 — gemeinsame Response-Shape fuer Cancel + Increase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MembershipAdjustResponseTO {
    pub action: MemberActionTO,
    pub member: MemberTO,
}

/// Phase 16 D-16-16 — Partial-Repayment-Request-Body fuer `POST /api/members/{id}/partial-repayment`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialRepaymentRequestTO {
    pub willensbekundung_date: time::Date,
    pub shares: i32,
}

/// Phase 16 D-16-16 — Partial-Repayment-Response. `phase` ist `Some(...)` wenn Auto-Anlegen passierte.
/// `entry` und `phase` als `serde_json::Value` (Zero-Coupling) — Frontend braucht nur
/// `entry.id` und `phase.fiscal_year`/`phase.id`. Die vollstaendigen Repayment-TOs leben in
/// `genossi-frontend/src/api.rs` (historisch dort, nicht in rest-types).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialRepaymentResponseTO {
    pub entry: serde_json::Value,
    pub member: MemberTO,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phase: Option<serde_json::Value>,
}

/// Phase 17 C-17-CF-07 — Transfer-Shares-Request-Body fuer `POST /api/members/{from_id}/transfer-shares`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferSharesRequestTO {
    pub to_member_id: Uuid,
    pub shares: i32,
    pub transfer_date: time::Date,
}

/// Phase 17 C-17-CF-07 — Transfer-Shares-Response. 2 actions bei Teil-Uebertrag, 3 bei Voll-Uebertrag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferSharesResponseTO {
    pub actions: Vec<MemberActionTO>,
    pub from: MemberTO,
    pub to: MemberTO,
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
            salutation: None,
            title: None,
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

// ── Communication timeline types ──────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommunicationDirection {
    Inbound,
    Outbound,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InboundStatusTO {
    pub done: bool,
    pub replied: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommunicationEntryTO {
    pub direction: CommunicationDirection,
    pub date: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbox_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbound_status: Option<InboundStatusTO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound_status: Option<String>,
}

// Audit Log types

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntryTO {
    pub id: Uuid,
    pub timestamp: String,
    pub user_id: String,
    pub process: String,
    pub transaction_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub field_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagedAuditLogTO {
    pub entries: Vec<AuditLogEntryTO>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyResponseTO {
    pub valid: bool,
    pub total_entries: usize,
    pub broken_links: Vec<BrokenLinkTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrokenLinkTO {
    pub entry_id: Uuid,
    pub expected_hash: String,
    pub actual_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimestampResponseTO {
    pub id: Uuid,
    pub timestamp: String,
    pub audit_hash: String,
    pub audit_entry_count: i64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webdav_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimestampVerifyResponseTO {
    pub token_valid: bool,
    pub hash_matches: bool,
    pub audit_log_consistent: bool,
    pub timestamp: String,
    pub audit_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRevokeResponse {
    pub message: String,
    pub revoked_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimestampCreateResponseTO {
    pub created: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<TimestampResponseTO>,
}
