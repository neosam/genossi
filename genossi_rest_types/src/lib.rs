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
