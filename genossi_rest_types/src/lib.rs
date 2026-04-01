use genossi_dao::member_action::ActionType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// Custom serialization module for ISO8601 datetime format
pub mod iso8601_datetime {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::format_description::well_known::Iso8601;
    use time::PrimitiveDateTime;

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

// Custom serialization module for ISO8601 date format (date only, no time)
pub mod iso8601_date {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Date;

    pub fn serialize<S>(date: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => {
                let format =
                    time::format_description::parse("[year]-[month]-[day]").unwrap();
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
                let format =
                    time::format_description::parse("[year]-[month]-[day]").unwrap();
                Date::parse(&s, &format)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            None => Ok(None),
        }
    }
}

// Required date serialization (non-optional)
pub mod iso8601_date_required {
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = 42)]
    pub member_number: i64,
    #[schema(example = "Max")]
    pub first_name: String,
    #[schema(example = "Mustermann")]
    pub last_name: String,
    #[schema(example = "max@example.com")]
    pub email: Option<String>,
    #[schema(example = "Muster GmbH")]
    pub company: Option<String>,
    pub comment: Option<String>,
    #[schema(example = "Musterstraße")]
    pub street: Option<String>,
    #[schema(example = "1a")]
    pub house_number: Option<String>,
    #[schema(example = "12345")]
    pub postal_code: Option<String>,
    #[schema(example = "Berlin")]
    pub city: Option<String>,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2024-01-15")]
    pub join_date: time::Date,
    #[schema(example = 1)]
    pub shares_at_joining: i32,
    #[schema(example = 3)]
    pub current_shares: i32,
    #[schema(example = 15000)]
    pub current_balance: i64,
    #[serde(default)]
    #[schema(example = 0)]
    pub action_count: i32,
    #[serde(default)]
    pub migrated: bool,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_date::serialize",
        deserialize_with = "iso8601_date::deserialize",
        default
    )]
    #[schema(example = "2025-06-30")]
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

impl From<&genossi_service::member::Member> for MemberTO {
    fn from(m: &genossi_service::member::Member) -> Self {
        Self {
            id: Some(m.id),
            member_number: m.member_number,
            first_name: m.first_name.to_string(),
            last_name: m.last_name.to_string(),
            email: m.email.as_deref().map(String::from),
            company: m.company.as_deref().map(String::from),
            comment: m.comment.as_deref().map(String::from),
            street: m.street.as_deref().map(String::from),
            house_number: m.house_number.as_deref().map(String::from),
            postal_code: m.postal_code.as_deref().map(String::from),
            city: m.city.as_deref().map(String::from),
            join_date: m.join_date,
            shares_at_joining: m.shares_at_joining,
            current_shares: m.current_shares,
            current_balance: m.current_balance,
            action_count: m.action_count,
            migrated: m.migrated,
            exit_date: m.exit_date,
            bank_account: m.bank_account.as_deref().map(String::from),
            created: Some(m.created),
            deleted: m.deleted,
            version: Some(m.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberImportErrorTO {
    #[schema(example = 7)]
    pub row: usize,
    #[schema(example = "Invalid date in 'Beitritt'")]
    pub error: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberImportResultTO {
    #[schema(example = 42)]
    pub imported: usize,
    #[schema(example = 5)]
    pub updated: usize,
    #[schema(example = 2)]
    pub skipped: usize,
    pub errors: Vec<MemberImportErrorTO>,
}

impl From<genossi_service::member_import::MemberImportResult> for MemberImportResultTO {
    fn from(r: genossi_service::member_import::MemberImportResult) -> Self {
        Self {
            imported: r.imported,
            updated: r.updated,
            skipped: r.skipped,
            errors: r
                .errors
                .into_iter()
                .map(|e| MemberImportErrorTO {
                    row: e.row,
                    error: e.error,
                })
                .collect(),
        }
    }
}

impl From<&MemberTO> for genossi_service::member::Member {
    fn from(to: &MemberTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            member_number: to.member_number,
            first_name: Arc::from(to.first_name.as_str()),
            last_name: Arc::from(to.last_name.as_str()),
            email: to.email.as_deref().map(Arc::from),
            company: to.company.as_deref().map(Arc::from),
            comment: to.comment.as_deref().map(Arc::from),
            street: to.street.as_deref().map(Arc::from),
            house_number: to.house_number.as_deref().map(Arc::from),
            postal_code: to.postal_code.as_deref().map(Arc::from),
            city: to.city.as_deref().map(Arc::from),
            join_date: to.join_date,
            shares_at_joining: to.shares_at_joining,
            current_shares: to.current_shares,
            current_balance: to.current_balance,
            action_count: to.action_count,
            migrated: to.migrated,
            exit_date: to.exit_date,
            bank_account: to.bank_account.as_deref().map(Arc::from),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ActionTypeTO {
    Eintritt,
    Austritt,
    Todesfall,
    Aufstockung,
    Verkauf,
    UebertragungEmpfang,
    UebertragungAbgabe,
}

impl From<&ActionType> for ActionTypeTO {
    fn from(at: &ActionType) -> Self {
        match at {
            ActionType::Eintritt => ActionTypeTO::Eintritt,
            ActionType::Austritt => ActionTypeTO::Austritt,
            ActionType::Todesfall => ActionTypeTO::Todesfall,
            ActionType::Aufstockung => ActionTypeTO::Aufstockung,
            ActionType::Verkauf => ActionTypeTO::Verkauf,
            ActionType::UebertragungEmpfang => ActionTypeTO::UebertragungEmpfang,
            ActionType::UebertragungAbgabe => ActionTypeTO::UebertragungAbgabe,
        }
    }
}

impl From<&ActionTypeTO> for ActionType {
    fn from(at: &ActionTypeTO) -> Self {
        match at {
            ActionTypeTO::Eintritt => ActionType::Eintritt,
            ActionTypeTO::Austritt => ActionType::Austritt,
            ActionTypeTO::Todesfall => ActionType::Todesfall,
            ActionTypeTO::Aufstockung => ActionType::Aufstockung,
            ActionTypeTO::Verkauf => ActionType::Verkauf,
            ActionTypeTO::UebertragungEmpfang => ActionType::UebertragungEmpfang,
            ActionTypeTO::UebertragungAbgabe => ActionType::UebertragungAbgabe,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberActionTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174001")]
    pub member_id: Uuid,
    pub action_type: ActionTypeTO,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2024-03-15")]
    pub date: time::Date,
    #[schema(example = 3)]
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

impl From<&genossi_service::member_action::MemberAction> for MemberActionTO {
    fn from(a: &genossi_service::member_action::MemberAction) -> Self {
        Self {
            id: Some(a.id),
            member_id: a.member_id,
            action_type: ActionTypeTO::from(&a.action_type),
            date: a.date,
            shares_change: a.shares_change,
            transfer_member_id: a.transfer_member_id,
            effective_date: a.effective_date,
            comment: a.comment.as_deref().map(String::from),
            created: Some(a.created),
            deleted: a.deleted,
            version: Some(a.version),
        }
    }
}

impl From<&MemberActionTO> for genossi_service::member_action::MemberAction {
    fn from(to: &MemberActionTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            member_id: to.member_id,
            action_type: ActionType::from(&to.action_type),
            date: to.date,
            shares_change: to.shares_change,
            transfer_member_id: to.transfer_member_id,
            effective_date: to.effective_date,
            comment: to.comment.as_deref().map(Arc::from),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MigrationStatusTO {
    pub member_id: Uuid,
    pub status: String,
    #[schema(example = 5)]
    pub expected_shares: i32,
    #[schema(example = 5)]
    pub actual_shares: i32,
    #[schema(example = 2)]
    pub expected_action_count: i32,
    #[schema(example = 2)]
    pub actual_action_count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MemberDocumentTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174001")]
    pub member_id: Uuid,
    #[schema(example = "join_declaration")]
    pub document_type: String,
    pub description: Option<String>,
    #[schema(example = "beitrittserklaerung.pdf")]
    pub file_name: String,
    #[schema(example = "application/pdf")]
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

impl From<&genossi_service::member_document::MemberDocument> for MemberDocumentTO {
    fn from(d: &genossi_service::member_document::MemberDocument) -> Self {
        Self {
            id: Some(d.id),
            member_id: d.member_id,
            document_type: d.document_type.as_str().to_string(),
            description: d.description.as_deref().map(String::from),
            file_name: d.file_name.to_string(),
            mime_type: d.mime_type.to_string(),
            created: Some(d.created),
            deleted: d.deleted,
            version: Some(d.version),
        }
    }
}

impl From<&genossi_service::member_action::MigrationStatus> for MigrationStatusTO {
    fn from(s: &genossi_service::member_action::MigrationStatus) -> Self {
        Self {
            member_id: s.member_id,
            status: match s.status {
                genossi_service::member_action::MigrationState::Migrated => {
                    "migrated".to_string()
                }
                genossi_service::member_action::MigrationState::Pending => "pending".to_string(),
            },
            expected_shares: s.expected_shares,
            actual_shares: s.actual_shares,
            expected_action_count: s.expected_action_count,
            actual_action_count: s.actual_action_count,
        }
    }
}
