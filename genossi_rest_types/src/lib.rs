use genossi_dao::application::ApplicationStatus;
use genossi_dao::member::{MemberStatus, PostalStatus, Salutation};
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SalutationTO {
    Herr,
    Frau,
    Firma,
}

impl From<&Salutation> for SalutationTO {
    fn from(s: &Salutation) -> Self {
        match s {
            Salutation::Herr => SalutationTO::Herr,
            Salutation::Frau => SalutationTO::Frau,
            Salutation::Firma => SalutationTO::Firma,
        }
    }
}

impl From<&SalutationTO> for Salutation {
    fn from(s: &SalutationTO) -> Self {
        match s {
            SalutationTO::Herr => Salutation::Herr,
            SalutationTO::Frau => Salutation::Frau,
            SalutationTO::Firma => Salutation::Firma,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub enum MemberStatusTO {
    #[default]
    Normal,
    FehlerhaftErfasst,
}

impl From<&MemberStatus> for MemberStatusTO {
    fn from(s: &MemberStatus) -> Self {
        match s {
            MemberStatus::Normal => MemberStatusTO::Normal,
            MemberStatus::FehlerhaftErfasst => MemberStatusTO::FehlerhaftErfasst,
        }
    }
}

impl From<&MemberStatusTO> for MemberStatus {
    fn from(s: &MemberStatusTO) -> Self {
        match s {
            MemberStatusTO::Normal => MemberStatus::Normal,
            MemberStatusTO::FehlerhaftErfasst => MemberStatus::FehlerhaftErfasst,
        }
    }
}

/// Quick 260625-e14: REST-Transfer-Objekt für den postalischen Status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
pub enum PostalStatusTO {
    #[default]
    Erreichbar,
    Unzustellbar,
}

impl From<&PostalStatus> for PostalStatusTO {
    fn from(s: &PostalStatus) -> Self {
        match s {
            PostalStatus::Erreichbar => PostalStatusTO::Erreichbar,
            PostalStatus::Unzustellbar => PostalStatusTO::Unzustellbar,
        }
    }
}

impl From<&PostalStatusTO> for PostalStatus {
    fn from(s: &PostalStatusTO) -> Self {
        match s {
            PostalStatusTO::Erreichbar => PostalStatus::Erreichbar,
            PostalStatusTO::Unzustellbar => PostalStatus::Unzustellbar,
        }
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[schema(example = "Dr.")]
    pub title: Option<String>,
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
    // Quick 260607-mw9: optional Kontoinhaber (Account Holder).
    // Wird im Auszahlungs-Anschreiben als Recipient-Adressblock verwendet,
    // wenn das Bankkonto auf einen anderen Namen läuft. None = Fallback auf
    // first_name + last_name. PII-Hinweis: Bewusst NICHT in MemberSlimTO.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Erika Mustermann")]
    pub account_holder: Option<String>,
    #[serde(default)]
    pub status: MemberStatusTO,
    // Quick 260625-e14: postalischer Status. #[serde(default)] sorgt für
    // Abwärtskompatibilität älterer Clients (fehlt -> Erreichbar).
    #[serde(default)]
    pub postal_status: PostalStatusTO,
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
            salutation: m.salutation.as_ref().map(SalutationTO::from),
            title: m.title.as_deref().map(String::from),
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
            account_holder: m.account_holder.as_deref().map(String::from),
            status: MemberStatusTO::from(&m.status),
            postal_status: PostalStatusTO::from(&m.postal_status),
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
            salutation: to.salutation.as_ref().map(Salutation::from),
            title: to.title.as_deref().map(Arc::from),
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
            account_holder: to.account_holder.as_deref().map(Arc::from),
            status: MemberStatus::from(&to.status),
            postal_status: PostalStatus::from(&to.postal_status),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

/// Reduzierte Darstellung eines Mitglieds fuer Empfaenger-Search (TRSF-06).
///
/// **PII-Leak-Guard:** Diese Struct hat EXAKT 6 Felder. KEIN
/// `impl From<&MemberTO> for MemberSlimTO` — sonst wuerden neue MemberTO-Felder
/// (email, bank_account, street, IBAN, current_shares, current_balance) durch
/// `MemberTO`-Erweiterungen unbemerkt in den Slim-Endpunkt durchrutschen.
/// Konversion EXKLUSIV via `From<&genossi_service::member::Member>` aus dem
/// Service-Layer — jedes neue PII-Feld muss explizit hier ergaenzt werden.
///
/// Pattern-Vorbild: `AttendanceMemberTO` (ATTN-01 Slim-DTO mit identischer
/// Whitelist-Disziplin und Pendant-Tests in `member_slim_to_tests`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberSlimTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = 42)]
    pub member_number: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Dr.")]
    pub title: Option<String>,
    #[schema(example = "Anna")]
    pub first_name: String,
    #[schema(example = "Schmidt")]
    pub last_name: String,
}

impl From<&genossi_service::member::Member> for MemberSlimTO {
    fn from(m: &genossi_service::member::Member) -> Self {
        Self {
            id: m.id,
            member_number: m.member_number,
            salutation: m.salutation.as_ref().map(SalutationTO::from),
            title: m.title.as_deref().map(String::from),
            first_name: m.first_name.to_string(),
            last_name: m.last_name.to_string(),
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
    Note,
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
            ActionType::Note => ActionTypeTO::Note,
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
            ActionTypeTO::Note => ActionType::Note,
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

// ============================================================================
// Phase 15 v1.2 — Membership-Adjust Request/Response TOs (D-15-10, D-15-11)
// ============================================================================
//
// Request-DTOs nutzen `iso8601_date_required` Serde (Pflichtfeld; nicht
// Optional). Response-DTO komponiert die Domain-Tupel-Return-Werte
// (action, member) als Single-Round-Trip-Payload fuer Frontend-Phase-18.

/// Request-Body fuer `POST /api/members/{id}/cancel` (CANC-01).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelMembershipRequestTO {
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub willensbekundung_date: time::Date,
}

/// Request-Body fuer `POST /api/members/{id}/increase-shares` (UPGD-01).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct IncreaseSharesRequestTO {
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub willensbekundung_date: time::Date,
    #[schema(example = 2)]
    pub shares: i32,
}

/// Response-Body fuer beide Phase-15-Endpoints (D-15-11).
///
/// Bundelt die neu erzeugte `MemberAction` mit dem aktualisierten `Member`,
/// damit das Frontend in einem einzigen Round-Trip die Detail-View
/// (exit_date / current_shares) refreshen kann — kein POST-then-GET.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MembershipAdjustResponseTO {
    pub action: MemberActionTO,
    pub member: MemberTO,
}

/// Request-Body fuer `POST /api/members/{id}/partial-repayment` (PART-01, D-16-15).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PartialRepaymentRequestTO {
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub willensbekundung_date: time::Date,
    /// Anzahl der zurueckgegebenen Anteile (1..current_shares; D-16-11/12 enforce
    /// strict `< current_shares`). Type ist `i32` konsistent mit
    /// `MemberEntity.current_shares` und `RepaymentEntryEntity.share_count_to_pay_out`
    /// (research finding #3 — CONTEXT Z. 12 sagt faelschlicherweise i64).
    #[schema(example = 2)]
    pub shares: i32,
}

/// Response-Body fuer `POST /api/members/{id}/partial-repayment` (D-16-16).
///
/// `phase` ist nur bei Auto-Anlegen befuellt (D-16-01 Variante B); ansonsten `None`
/// → wird aus dem JSON entfernt (`skip_serializing_if`). Frontend (Phase 18) zeigt
/// dann den Hinweis "Phase fuer FY YYYY wurde automatisch angelegt".
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PartialRepaymentResponseTO {
    pub entry: RepaymentEntryTO,
    pub member: MemberTO,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub phase: Option<RepaymentPhaseTO>,
}

/// Request-Body fuer `POST /api/members/{from_id}/transfer-shares` (TRSF-01).
///
/// `shares` muss im Bereich `1..=from.current_shares` liegen (Service-Validation).
/// `shares == from.current_shares` triggert Voll-Uebertrag mit Austritt-Cascade
/// (D-17-01). `to_member_id` darf NICHT gleich dem Path-Parameter `from_id`
/// sein (TRSF-07 / D-17-08, Self-Transfer-Block).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferSharesRequestTO {
    pub to_member_id: Uuid,
    /// Anzahl der zu uebertragenden Anteile (1..=from.current_shares).
    #[schema(example = 2)]
    pub shares: i32,
    /// Wirksamkeitsdatum des Uebertrags (TRSF-05 sofort wirksam, kein H1/H2).
    /// Muss in [today.year(), today.year()+1] liegen (Phase-15 D-15-05 Re-use).
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub transfer_date: time::Date,
}

/// Response-Body fuer `POST /api/members/{from_id}/transfer-shares` (C-17-CF-07).
///
/// `actions.len()` ist 2 (Teil-Uebertrag) oder 3 (Voll-Uebertrag inkl. Austritt).
/// `from` + `to` enthalten die aktualisierten Members nach Tx-Commit (inkl.
/// neuer `version` und ggf. `exit_date` bei Voll-Uebertrag), so dass das Frontend
/// (Phase 18) mit einem Round-Trip beide Member-Detail-Views refreshen kann.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferSharesResponseTO {
    pub actions: Vec<MemberActionTO>,
    pub from: MemberTO,
    pub to: MemberTO,
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UnsupportedFileTypeResponse {
    pub error: String,
    pub allowed_extensions: Vec<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationResultTO {
    pub member_number_gaps: Vec<i64>,
    pub unmatched_transfers: Vec<UnmatchedTransferTO>,
    pub shares_mismatches: Vec<SharesMismatchTO>,
    pub missing_entry_actions: Vec<MissingEntryActionTO>,
    pub exit_date_mismatches: Vec<ExitDateMismatchTO>,
    pub active_members_no_shares: Vec<ActiveMemberNoSharesTO>,
    pub duplicate_member_numbers: Vec<DuplicateMemberNumberTO>,
    pub exited_members_with_shares: Vec<ExitedMemberWithSharesTO>,
    pub migrated_flag_mismatches: Vec<MigratedFlagMismatchTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UnmatchedTransferTO {
    pub action_id: Uuid,
    pub member_id: Uuid,
    #[schema(example = 42)]
    pub member_number: i64,
    pub action_type: ActionTypeTO,
    pub transfer_member_id: Uuid,
    #[schema(example = 17)]
    pub transfer_member_number: i64,
    #[schema(example = -3)]
    pub shares_change: i32,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2024-05-01")]
    pub date: time::Date,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SharesMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub expected: i32,
    pub actual: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MissingEntryActionTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub actual_count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ExitDateMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub has_exit_date: bool,
    pub has_austritt_action: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ActiveMemberNoSharesTO {
    pub member_id: Uuid,
    pub member_number: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateMemberNumberTO {
    pub member_number: i64,
    pub member_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ExitedMemberWithSharesTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub current_shares: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MigratedFlagMismatchTO {
    pub member_id: Uuid,
    pub member_number: i64,
    pub flag_value: bool,
    pub computed_status: String,
}

impl From<&genossi_service::validation::ValidationResult> for ValidationResultTO {
    fn from(r: &genossi_service::validation::ValidationResult) -> Self {
        Self {
            member_number_gaps: r.member_number_gaps.to_vec(),
            unmatched_transfers: r
                .unmatched_transfers
                .iter()
                .map(UnmatchedTransferTO::from)
                .collect(),
            shares_mismatches: r
                .shares_mismatches
                .iter()
                .map(SharesMismatchTO::from)
                .collect(),
            missing_entry_actions: r
                .missing_entry_actions
                .iter()
                .map(MissingEntryActionTO::from)
                .collect(),
            exit_date_mismatches: r
                .exit_date_mismatches
                .iter()
                .map(ExitDateMismatchTO::from)
                .collect(),
            active_members_no_shares: r
                .active_members_no_shares
                .iter()
                .map(ActiveMemberNoSharesTO::from)
                .collect(),
            duplicate_member_numbers: r
                .duplicate_member_numbers
                .iter()
                .map(DuplicateMemberNumberTO::from)
                .collect(),
            exited_members_with_shares: r
                .exited_members_with_shares
                .iter()
                .map(ExitedMemberWithSharesTO::from)
                .collect(),
            migrated_flag_mismatches: r
                .migrated_flag_mismatches
                .iter()
                .map(MigratedFlagMismatchTO::from)
                .collect(),
        }
    }
}

impl From<&genossi_service::validation::UnmatchedTransfer> for UnmatchedTransferTO {
    fn from(t: &genossi_service::validation::UnmatchedTransfer) -> Self {
        Self {
            action_id: t.action_id,
            member_id: t.member_id,
            member_number: t.member_number,
            action_type: ActionTypeTO::from(&t.action_type),
            transfer_member_id: t.transfer_member_id,
            transfer_member_number: t.transfer_member_number,
            shares_change: t.shares_change,
            date: t.date,
        }
    }
}

impl From<&genossi_service::validation::SharesMismatch> for SharesMismatchTO {
    fn from(s: &genossi_service::validation::SharesMismatch) -> Self {
        Self {
            member_id: s.member_id,
            member_number: s.member_number,
            expected: s.expected,
            actual: s.actual,
        }
    }
}

impl From<&genossi_service::validation::MissingEntryAction> for MissingEntryActionTO {
    fn from(m: &genossi_service::validation::MissingEntryAction) -> Self {
        Self {
            member_id: m.member_id,
            member_number: m.member_number,
            actual_count: m.actual_count,
        }
    }
}

impl From<&genossi_service::validation::ExitDateMismatch> for ExitDateMismatchTO {
    fn from(e: &genossi_service::validation::ExitDateMismatch) -> Self {
        Self {
            member_id: e.member_id,
            member_number: e.member_number,
            has_exit_date: e.has_exit_date,
            has_austritt_action: e.has_austritt_action,
        }
    }
}

impl From<&genossi_service::validation::ActiveMemberNoShares> for ActiveMemberNoSharesTO {
    fn from(a: &genossi_service::validation::ActiveMemberNoShares) -> Self {
        Self {
            member_id: a.member_id,
            member_number: a.member_number,
        }
    }
}

impl From<&genossi_service::validation::DuplicateMemberNumber> for DuplicateMemberNumberTO {
    fn from(d: &genossi_service::validation::DuplicateMemberNumber) -> Self {
        Self {
            member_number: d.member_number,
            member_ids: d.member_ids.to_vec(),
        }
    }
}

impl From<&genossi_service::validation::ExitedMemberWithShares> for ExitedMemberWithSharesTO {
    fn from(e: &genossi_service::validation::ExitedMemberWithShares) -> Self {
        Self {
            member_id: e.member_id,
            member_number: e.member_number,
            current_shares: e.current_shares,
        }
    }
}

impl From<&genossi_service::validation::MigratedFlagMismatch> for MigratedFlagMismatchTO {
    fn from(m: &genossi_service::validation::MigratedFlagMismatch) -> Self {
        Self {
            member_id: m.member_id,
            member_number: m.member_number,
            flag_value: m.flag_value,
            computed_status: m.computed_status.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserPreferenceTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    #[schema(example = "member_list_columns")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    #[schema(example = r#"["member_number","last_name","first_name"]"#)]
    pub value: String,

    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::user_preference::UserPreference> for UserPreferenceTO {
    fn from(p: &genossi_service::user_preference::UserPreference) -> Self {
        Self {
            id: Some(p.id),
            key: Some(p.key.to_string()),
            value: p.value.to_string(),
            created: Some(p.created),
            version: Some(p.version),
        }
    }
}

impl From<&genossi_service::member_action::MigrationStatus> for MigrationStatusTO {
    fn from(s: &genossi_service::member_action::MigrationStatus) -> Self {
        Self {
            member_id: s.member_id,
            status: match s.status {
                genossi_service::member_action::MigrationState::Migrated => "migrated".to_string(),
                genossi_service::member_action::MigrationState::Pending => "pending".to_string(),
            },
            expected_shares: s.expected_shares,
            actual_shares: s.actual_shares,
            expected_action_count: s.expected_action_count,
            actual_action_count: s.actual_action_count,
        }
    }
}

// Application (membership declaration) types

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ApplicationStatusTO {
    Offen,
    Bestaetigt,
    Abgelehnt,
}

impl From<&ApplicationStatus> for ApplicationStatusTO {
    fn from(s: &ApplicationStatus) -> Self {
        match s {
            ApplicationStatus::Offen => ApplicationStatusTO::Offen,
            ApplicationStatus::Bestaetigt => ApplicationStatusTO::Bestaetigt,
            ApplicationStatus::Abgelehnt => ApplicationStatusTO::Abgelehnt,
        }
    }
}

impl From<&ApplicationStatusTO> for ApplicationStatus {
    fn from(s: &ApplicationStatusTO) -> Self {
        match s {
            ApplicationStatusTO::Offen => ApplicationStatus::Offen,
            ApplicationStatusTO::Bestaetigt => ApplicationStatus::Bestaetigt,
            ApplicationStatusTO::Abgelehnt => ApplicationStatus::Abgelehnt,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApplicationTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "Max")]
    pub first_name: String,
    #[schema(example = "Mustermann")]
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Dr.")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "max@example.com")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Musterstraße")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "42")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "12345")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Berlin")]
    pub city: Option<String>,
    #[schema(example = 1)]
    pub shares: i32,
    pub status: ApplicationStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Option<Uuid>,
}

impl From<&genossi_service::application::Application> for ApplicationTO {
    fn from(a: &genossi_service::application::Application) -> Self {
        Self {
            id: a.id,
            first_name: a.first_name.to_string(),
            last_name: a.last_name.to_string(),
            salutation: a.salutation.as_ref().map(SalutationTO::from),
            title: a.title.as_deref().map(|s| s.to_string()),
            email: a.email.as_deref().map(|s| s.to_string()),
            street: a.street.as_deref().map(|s| s.to_string()),
            house_number: a.house_number.as_deref().map(|s| s.to_string()),
            postal_code: a.postal_code.as_deref().map(|s| s.to_string()),
            city: a.city.as_deref().map(|s| s.to_string()),
            shares: a.shares,
            status: ApplicationStatusTO::from(&a.status),
            created: Some(a.created),
            deleted: a.deleted,
            version: Some(a.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicJoinRequest {
    #[schema(example = "Max")]
    pub first_name: String,
    #[schema(example = "Mustermann")]
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Dr.")]
    pub title: Option<String>,
    #[schema(example = "max@example.com")]
    pub email: String,
    #[schema(example = "Musterstraße")]
    pub street: String,
    #[schema(example = "42")]
    pub house_number: String,
    #[schema(example = "12345")]
    pub postal_code: String,
    #[schema(example = "Berlin")]
    pub city: String,
    #[schema(example = 1)]
    pub shares: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicJoinResponse {
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationFailureItem {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub errors: Vec<ValidationFailureItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminCreateApplicationRequest {
    #[schema(example = "Max")]
    pub first_name: String,
    #[schema(example = "Mustermann")]
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Dr.")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "max@example.com")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Musterstraße")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "42")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "12345")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Berlin")]
    pub city: Option<String>,
    #[schema(example = 1)]
    pub shares: i32,
    #[serde(default)]
    pub send_mail: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateApplicationRequest {
    #[schema(example = "Max")]
    pub first_name: String,
    #[schema(example = "Mustermann")]
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Dr.")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "max@example.com")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Musterstraße")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "42")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "12345")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Berlin")]
    pub city: Option<String>,
    #[schema(example = 1)]
    pub shares: i32,
    pub version: Uuid,
}

// Assembly (Generalversammlung) types

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AssemblyStatusTO {
    Preparation,
    Open,
    Closed,
}

impl From<&genossi_dao::assembly::AssemblyStatus> for AssemblyStatusTO {
    fn from(s: &genossi_dao::assembly::AssemblyStatus) -> Self {
        use genossi_dao::assembly::AssemblyStatus;
        match s {
            AssemblyStatus::Preparation => AssemblyStatusTO::Preparation,
            AssemblyStatus::Open => AssemblyStatusTO::Open,
            AssemblyStatus::Closed => AssemblyStatusTO::Closed,
        }
    }
}

impl From<&AssemblyStatusTO> for genossi_dao::assembly::AssemblyStatus {
    fn from(s: &AssemblyStatusTO) -> Self {
        use genossi_dao::assembly::AssemblyStatus;
        match s {
            AssemblyStatusTO::Preparation => AssemblyStatus::Preparation,
            AssemblyStatusTO::Open => AssemblyStatus::Open,
            AssemblyStatusTO::Closed => AssemblyStatus::Closed,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssemblyTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
    pub status: AssemblyStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub opened_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub closed_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::assembly::Assembly> for AssemblyTO {
    fn from(a: &genossi_service::assembly::Assembly) -> Self {
        Self {
            id: a.id,
            name: a.name.to_string(),
            date: Some(a.date),
            location: a.location.as_ref().map(|s| s.to_string()),
            status: AssemblyStatusTO::from(&a.status),
            opened_at: a.opened_at,
            closed_at: a.closed_at,
            created: Some(a.created),
            deleted: a.deleted,
            version: Some(a.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssemblyDetailTO {
    pub assembly: AssemblyTO,
    pub snapshot_member_count: u64,
}

impl From<&genossi_service::assembly::AssemblyDetail> for AssemblyDetailTO {
    fn from(d: &genossi_service::assembly::AssemblyDetail) -> Self {
        Self {
            assembly: AssemblyTO::from(&d.assembly),
            snapshot_member_count: d.snapshot_member_count,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAssemblyRequest {
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAssemblyRequest {
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
    pub version: Uuid,
}

// ============================================================================
// Phase 7: RepaymentPhase TOs (PHAS-01..PHAS-05)
// ============================================================================
//
// Mirrors `RepaymentPhase` from `genossi_service::repayment_phase` and follows
// the AssemblyTO pattern (Z. 1005-1141): bidirektionale Status-From-Impls,
// ISO8601-Datetime-Serde auf allen Optional-Timestamps, version als Option mit
// skip_if_none, Pflichtfelder `fiscal_year: i32` und `share_value: i64` (Cent).
//
// **KEIN `RepaymentPhaseDetailTO`** (Phase 7 hat keinen Snapshot — `get_*`
// liefert direkt `RepaymentPhase`).
// **KEIN `status`-Feld in `UpdateRepaymentPhaseRequest`** (D-02: Lifecycle-
// Transitionen gehen ausschließlich über `POST /open` und `POST /close`).

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum RepaymentPhaseStatusTO {
    Preparation,
    Open,
    Closed,
}

impl From<&genossi_dao::repayment_phase::RepaymentPhaseStatus> for RepaymentPhaseStatusTO {
    fn from(s: &genossi_dao::repayment_phase::RepaymentPhaseStatus) -> Self {
        use genossi_dao::repayment_phase::RepaymentPhaseStatus;
        match s {
            RepaymentPhaseStatus::Preparation => RepaymentPhaseStatusTO::Preparation,
            RepaymentPhaseStatus::Open => RepaymentPhaseStatusTO::Open,
            RepaymentPhaseStatus::Closed => RepaymentPhaseStatusTO::Closed,
        }
    }
}

impl From<&RepaymentPhaseStatusTO> for genossi_dao::repayment_phase::RepaymentPhaseStatus {
    fn from(s: &RepaymentPhaseStatusTO) -> Self {
        use genossi_dao::repayment_phase::RepaymentPhaseStatus;
        match s {
            RepaymentPhaseStatusTO::Preparation => RepaymentPhaseStatus::Preparation,
            RepaymentPhaseStatusTO::Open => RepaymentPhaseStatus::Open,
            RepaymentPhaseStatusTO::Closed => RepaymentPhaseStatus::Closed,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepaymentPhaseTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = 2026)]
    pub fiscal_year: i32,
    #[schema(example = 12000)]
    pub share_value: i64,
    pub status: RepaymentPhaseStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub opened_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub closed_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::repayment_phase::RepaymentPhase> for RepaymentPhaseTO {
    fn from(p: &genossi_service::repayment_phase::RepaymentPhase) -> Self {
        Self {
            id: p.id,
            fiscal_year: p.fiscal_year,
            share_value: p.share_value,
            status: RepaymentPhaseStatusTO::from(&p.status),
            opened_at: p.opened_at,
            closed_at: p.closed_at,
            created: Some(p.created),
            deleted: p.deleted,
            version: Some(p.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRepaymentPhaseRequest {
    #[schema(example = 2026)]
    pub fiscal_year: i32,
    #[schema(example = 12000)]
    pub share_value: i64,
}

/// Update body for `PUT /api/repayment-phase/{id}`.
///
/// **KEIN `status`-Feld** (D-02: Status-Übergänge laufen ausschließlich über
/// die dedizierten Action-Endpoints `POST /open` / `POST /close`).
/// `version` ist Pflicht (Optimistic Locking).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRepaymentPhaseRequest {
    #[schema(example = 2026)]
    pub fiscal_year: i32,
    #[schema(example = 12000)]
    pub share_value: i64,
    pub version: Uuid,
}

// ============================================================================
// Phase 8: RepaymentEntry TOs (ENTR-02..ENTR-06)
// ============================================================================
//
// Mirrors `RepaymentEntry` from `genossi_service::repayment_entry` and follows
// the RepaymentPhaseTO pattern (Z. 1144-1259): bidirektionale Status-From-Impls,
// ISO8601-Datetime-Serde auf Optional-Timestamps, version als Option mit
// skip_if_none.
//
// **PUT-Body (`UpdateRepaymentEntryRequest`)** ist Optional-Field-based
// (D-12): share_count_to_pay_out und status sind beide Option, nur `version`
// ist Pflicht. PaidOut als Target-Status liefert 409 — durchgesetzt im
// Service-Layer (Plan 03), nicht hier.
//
// **CloseConflictResponse** + **BatchFailureResponse** sind strukturierte
// 409-Body-Formalisierungen für die JSON-in-Arc<str>-Conflicts, die der
// Service-Layer aus Plan 03 (batch_toggle_status) und Plan 04
// (close_repayment_phase pending-validation) liefert.

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum RepaymentEntryStatusTO {
    Open,
    Contacted,
    PaidOut,
}

impl From<&genossi_dao::repayment_entry::RepaymentEntryStatus> for RepaymentEntryStatusTO {
    fn from(s: &genossi_dao::repayment_entry::RepaymentEntryStatus) -> Self {
        use genossi_dao::repayment_entry::RepaymentEntryStatus as S;
        match s {
            S::Open => RepaymentEntryStatusTO::Open,
            S::Contacted => RepaymentEntryStatusTO::Contacted,
            S::PaidOut => RepaymentEntryStatusTO::PaidOut,
        }
    }
}

impl From<&RepaymentEntryStatusTO> for genossi_dao::repayment_entry::RepaymentEntryStatus {
    fn from(s: &RepaymentEntryStatusTO) -> Self {
        use genossi_dao::repayment_entry::RepaymentEntryStatus as S;
        match s {
            RepaymentEntryStatusTO::Open => S::Open,
            RepaymentEntryStatusTO::Contacted => S::Contacted,
            RepaymentEntryStatusTO::PaidOut => S::PaidOut,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepaymentEntryTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub member_id: Uuid,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub phase_id: Uuid,
    #[schema(example = 5)]
    pub share_count_to_pay_out: i32,
    pub status: RepaymentEntryStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::repayment_entry::RepaymentEntry> for RepaymentEntryTO {
    fn from(e: &genossi_service::repayment_entry::RepaymentEntry) -> Self {
        Self {
            id: e.id,
            member_id: e.member_id,
            phase_id: e.phase_id,
            share_count_to_pay_out: e.share_count_to_pay_out,
            status: RepaymentEntryStatusTO::from(&e.status),
            created: Some(e.created),
            deleted: e.deleted,
            version: Some(e.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRepaymentEntryRequest {
    pub phase_id: Uuid,
    pub member_id: Uuid,
    #[schema(example = 5)]
    pub share_count_to_pay_out: i32,
}

/// Update body for `PUT /api/repayment-entry/{id}`.
///
/// Optional-Field-Pattern (D-12): Felder, die nicht im Body stehen, bleiben
/// unverändert. `version` ist Pflicht (Optimistic Locking).
/// Edit-Matrix (D-05/D-06/ENTR-04) wird im Service-Layer durchgesetzt:
/// - PaidOut als Target-Status → 409
/// - share_count_to_pay_out-Edit nur wenn entry.status ∈ {Open, Contacted}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRepaymentEntryRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub share_count_to_pay_out: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<RepaymentEntryStatusTO>,
    pub version: Uuid,
}

/// Request body for `POST /api/repayment-entry/batch-status`.
///
/// All-or-nothing-Semantik (D-08): erster Fehler → komplette Tx rollt
/// zurück + 409 mit strukturiertem `BatchFailureResponse`-Body.
/// PaidOut als `target_status` → 400 ValidationError (D-07).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchStatusRequest {
    pub entry_ids: Vec<Uuid>,
    pub target_status: RepaymentEntryStatusTO,
}

/// 409-Response-Body für `POST /api/repayment-phase/{id}/close` wenn pending
/// Entries existieren (PHAS-03 / D-15). Der Service-Layer (Plan 04) emittiert
/// genau dieses JSON-Schema im `ServiceError::Conflict(Arc<str>)`-Body; der
/// REST-Layer reicht den Body 1:1 als 409-Response durch.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CloseConflictResponse {
    pub error: String,
    pub pending_count: usize,
    /// Up to 20 member numbers; longer lists end with "+N weitere" as suffix entry.
    pub pending_member_numbers: Vec<String>,
}

/// Structured body for HTTP 409 batch-toggle conflicts (D-08, W-05).
///
/// Used by `POST /api/repayment-entry/batch-status` to report the FIRST
/// failing entry that triggered the all-or-nothing transaction rollback.
/// The Service-Layer (Plan 03) emittiert exakt dieses JSON-Schema im
/// `ServiceError::Conflict(Arc<str>)`-Body; der REST-Layer reicht den Body
/// 1:1 als 409-Response durch. Frontend kann dies direkt deserialisieren.
///
/// Scope: domain-level conflicts ONLY. Examples:
///   - Source status is not Open or Contacted (e.g. PaidOut → Contacted attempt)
///   - Future domain conflicts that signal "request semantically rejected"
///
/// NOT used for: missing or soft-deleted entry_ids. Those cases yield
/// HTTP 404 with the standard NotFound payload — aggregate-consistent with
/// `GET/PUT/DELETE /api/repayment-entry/{id}`. See Phase 08 Gap-Closure
/// Plan 09 / CR-02.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchFailureResponse {
    /// Zero-based index of the failing entry in the original BatchStatusRequest.entry_ids list.
    pub failure_index: usize,
    /// UUID of the failing entry (string form for JSON-portability).
    pub failure_id: String,
    /// Human-readable reason (e.g., "source status is 'PaidOut', expected Open or Contacted").
    pub failure_reason: String,
}

// ============================================================================
// Phase 2: Helper Token TOs (HLPR-01..HLPR-07)
// ============================================================================

/// Derived status from helper_token columns (D-02): no own status column.
/// Priority: revoked_at.is_some() => Revoked; else used_at.is_some() => Used; else Open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HelperTokenStatusTO {
    Open,
    Used,
    Revoked,
}

/// REST representation of a helper_token row.
/// Excludes `token_hash` (hash leakage prevention, D-06 audit-fields parallel).
///
/// ADR-2026-05-06: `code` (plain-text) and `qr_svg` (regenerated on demand
/// from `code`) are now optional fields. `Some` for tokens created after the
/// migration (admin can re-display the QR card); `None` for legacy rows
/// (frontend renders a "revoke + recreate" hint). The QR SVG is NEVER stored
/// — the REST handler regenerates it from `code` per request.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HelperTokenTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub assembly_id: Uuid,
    #[schema(example = "Anna")]
    pub memo: String,
    pub status: HelperTokenStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub used_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub revoked_at: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    /// ADR-2026-05-06: plain-text Crockford-Base32 code; `None` for legacy
    /// rows created before the migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "ABC1234567")]
    pub code: Option<String>,
    /// ADR-2026-05-06: QR-Code SVG regenerated on-demand from `code`. Always
    /// `None` when `code` is `None`. Not persisted in the DB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<svg xmlns=\"http://www.w3.org/2000/svg\">...</svg>")]
    pub qr_svg: Option<String>,
}

impl From<&genossi_dao::helper_token::HelperTokenEntity> for HelperTokenTO {
    fn from(entity: &genossi_dao::helper_token::HelperTokenEntity) -> Self {
        // D-02 status derivation: revoked dominates used.
        let status = if entity.revoked_at.is_some() {
            HelperTokenStatusTO::Revoked
        } else if entity.used_at.is_some() {
            HelperTokenStatusTO::Used
        } else {
            HelperTokenStatusTO::Open
        };
        // ADR-2026-05-06: this default From-impl does NOT attach code/qr_svg.
        // The handlers in genossi_rest::helper_token build their TOs inline
        // and regenerate qr_svg on demand from `code` (the SVG is not stored).
        HelperTokenTO {
            id: entity.id,
            assembly_id: entity.assembly_id,
            memo: entity.memo.to_string(),
            status,
            used_at: entity.used_at,
            revoked_at: entity.revoked_at,
            created: Some(entity.created),
            version: entity.version,
            code: None,
            qr_svg: None,
        }
    }
}

/// Response from POST /api/assembly/{assembly_id}/helper-tokens (D-21).
/// `code` (10 char Crockford) and `qr_svg` are returned ONCE — never persisted (D-11).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HelperTokenCreateResponseTO {
    pub token: HelperTokenTO,
    #[schema(example = "ABC1234567")]
    pub code: String,
    #[schema(example = "<svg xmlns=\"http://www.w3.org/2000/svg\">...</svg>")]
    pub qr_svg: String,
}

/// Request body for POST /api/assembly/{assembly_id}/helper-tokens (D-21).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateHelperTokenRequest {
    #[schema(example = "Anna")]
    pub memo: String,
}

/// Request body for POST /api/helper/redeem (D-22, public).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RedeemRequest {
    #[schema(example = "ABC1234567")]
    pub code: String,
}

/// Response body for successful POST /api/helper/redeem (D-22).
/// Cookie `app_session=<session_id>` is set in the Set-Cookie header; the body
/// only carries metadata for the helper-frontend state.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RedeemResponse {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub assembly_id: Uuid,
    /// ISO8601 timestamp when the session expires (24h after redeem; D-18).
    #[schema(example = "2026-05-04T10:00:00.000000000Z")]
    pub expires_at: String,
}

/// Response body for GET /api/helper/session — used by Frontend Auto-Redirect
/// (Phase 4 D-06). Returned only when a valid Helper-Session-Cookie is
/// present; 401 otherwise. PII-Whitelist: exactly 3 keys, no token-id, memo,
/// or member data (T-04-01 mitigation, parallel zu `AttendanceMemberTO`-Pattern).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HelperSessionTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub assembly_id: Uuid,
    #[schema(example = "GV 2026")]
    pub assembly_name: String,
    /// ISO8601 timestamp when the session expires (24h ab Redeem, D-18).
    #[schema(example = "2026-05-04T10:00:00.000000000Z")]
    pub expires_at: String,
}

#[cfg(test)]
mod helper_session_to_tests {
    use super::*;

    #[test]
    fn helper_session_to_serializes_exactly_three_keys() {
        let to = HelperSessionTO {
            assembly_id: Uuid::new_v4(),
            assembly_name: "GV 2026".to_string(),
            expires_at: "2026-05-04T10:00:00.000000000Z".to_string(),
        };
        let json = serde_json::to_value(&to).unwrap();
        let obj = json
            .as_object()
            .expect("HelperSessionTO must serialize as JSON object");
        assert_eq!(
            obj.len(),
            3,
            "HelperSessionTO must serialize exactly 3 keys (PII-Whitelist, T-04-01); got: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert!(obj.contains_key("assembly_id"), "missing key: assembly_id");
        assert!(
            obj.contains_key("assembly_name"),
            "missing key: assembly_name"
        );
        assert!(obj.contains_key("expires_at"), "missing key: expires_at");
    }
}

// Audit Log types

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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

impl From<&genossi_dao::audit_log::AuditLogEntry> for AuditLogEntryTO {
    fn from(e: &genossi_dao::audit_log::AuditLogEntry) -> Self {
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        Self {
            id: e.id,
            timestamp: e.timestamp.assume_utc().format(format).unwrap_or_default(),
            user_id: e.user_id.to_string(),
            process: e.process.to_string(),
            transaction_id: e.transaction_id,
            entity_type: e.entity_type.to_string(),
            entity_id: e.entity_id,
            action: e.action.to_string(),
            field_name: e.field_name.to_string(),
            old_value: e.old_value.as_ref().map(|s| s.to_string()),
            new_value: e.new_value.as_ref().map(|s| s.to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PagedAuditLogTO {
    pub entries: Vec<AuditLogEntryTO>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyResponseTO {
    pub valid: bool,
    pub total_entries: usize,
    pub broken_links: Vec<BrokenLinkTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BrokenLinkTO {
    pub entry_id: Uuid,
    pub expected_hash: String,
    pub actual_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TimestampResponseTO {
    pub id: Uuid,
    pub timestamp: String,
    pub audit_hash: String,
    pub audit_entry_count: i64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webdav_path: Option<String>,
}

impl From<&genossi_dao::audit_timestamp::AuditTimestampEntry> for TimestampResponseTO {
    fn from(e: &genossi_dao::audit_timestamp::AuditTimestampEntry) -> Self {
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        Self {
            id: e.id,
            timestamp: e.timestamp.assume_utc().format(format).unwrap_or_default(),
            audit_hash: e.audit_hash.to_string(),
            audit_entry_count: e.audit_entry_count,
            status: e.status.to_string(),
            webdav_path: e.webdav_path.as_ref().map(|s| s.to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TimestampVerifyResponseTO {
    pub token_valid: bool,
    pub hash_matches: bool,
    pub audit_log_consistent: bool,
    pub timestamp: String,
    pub audit_hash: String,
}

impl From<&genossi_service::timestamp::TimestampVerification> for TimestampVerifyResponseTO {
    fn from(v: &genossi_service::timestamp::TimestampVerification) -> Self {
        let format = &time::format_description::well_known::Iso8601::DEFAULT;
        Self {
            token_valid: v.token_valid,
            hash_matches: v.hash_matches,
            audit_log_consistent: v.audit_log_consistent,
            timestamp: v.timestamp.assume_utc().format(format).unwrap_or_default(),
            audit_hash: v.audit_hash.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TimestampCreateResponseTO {
    pub created: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<TimestampResponseTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionRevokeResponse {
    pub message: String,
    pub revoked_count: u64,
}

#[cfg(test)]
mod assembly_tests {
    use super::*;
    use genossi_dao::assembly::AssemblyStatus;

    #[test]
    fn test_assembly_status_to_serialize() {
        assert_eq!(
            serde_json::to_string(&AssemblyStatusTO::Preparation).unwrap(),
            "\"Preparation\""
        );
        assert_eq!(
            serde_json::to_string(&AssemblyStatusTO::Open).unwrap(),
            "\"Open\""
        );
        assert_eq!(
            serde_json::to_string(&AssemblyStatusTO::Closed).unwrap(),
            "\"Closed\""
        );
    }

    #[test]
    fn test_assembly_status_to_deserialize() {
        assert_eq!(
            serde_json::from_str::<AssemblyStatusTO>("\"Open\"").unwrap(),
            AssemblyStatusTO::Open
        );
        assert_eq!(
            serde_json::from_str::<AssemblyStatusTO>("\"Preparation\"").unwrap(),
            AssemblyStatusTO::Preparation
        );
        assert_eq!(
            serde_json::from_str::<AssemblyStatusTO>("\"Closed\"").unwrap(),
            AssemblyStatusTO::Closed
        );
    }

    #[test]
    fn test_assembly_to_optional_dates_default() {
        let json =
            r#"{"id":"123e4567-e89b-12d3-a456-426614174000","name":"GV","status":"Preparation"}"#;
        let parsed: AssemblyTO = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "GV");
        assert!(parsed.date.is_none());
        assert!(parsed.opened_at.is_none());
        assert!(parsed.closed_at.is_none());
        assert!(parsed.created.is_none());
        assert!(parsed.deleted.is_none());
        assert!(parsed.location.is_none());
        assert!(parsed.version.is_none());
        assert_eq!(parsed.status, AssemblyStatusTO::Preparation);
    }

    #[test]
    fn test_status_bidirectional_mapping() {
        // DAO -> TO -> DAO roundtrip for all three variants
        let preparation_back: AssemblyStatus =
            (&AssemblyStatusTO::from(&AssemblyStatus::Preparation)).into();
        assert_eq!(preparation_back, AssemblyStatus::Preparation);

        let open_back: AssemblyStatus = (&AssemblyStatusTO::from(&AssemblyStatus::Open)).into();
        assert_eq!(open_back, AssemblyStatus::Open);

        let closed_back: AssemblyStatus = (&AssemblyStatusTO::from(&AssemblyStatus::Closed)).into();
        assert_eq!(closed_back, AssemblyStatus::Closed);
    }

    #[test]
    fn test_assembly_detail_to_contains_count() {
        let assembly = AssemblyTO {
            id: Uuid::new_v4(),
            name: "GV 2026".to_string(),
            date: None,
            location: None,
            status: AssemblyStatusTO::Open,
            opened_at: None,
            closed_at: None,
            created: None,
            deleted: None,
            version: None,
        };
        let detail = AssemblyDetailTO {
            assembly,
            snapshot_member_count: 42,
        };
        assert_eq!(detail.snapshot_member_count, 42);
    }
}

#[cfg(test)]
mod assembly_request_tests {
    use super::*;

    #[test]
    fn test_create_assembly_request_minimal_json() {
        let json = r#"{"name":"GV 2026"}"#;
        let parsed: CreateAssemblyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "GV 2026");
        assert!(parsed.date.is_none());
        assert!(parsed.location.is_none());
    }

    #[test]
    fn test_create_assembly_request_full_json() {
        let json =
            r#"{"name":"GV","date":"2026-06-15T18:00:00.000000000Z","location":"Vereinsheim"}"#;
        let parsed: CreateAssemblyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "GV");
        assert!(parsed.date.is_some());
        assert_eq!(parsed.location.as_deref(), Some("Vereinsheim"));
    }

    #[test]
    fn test_update_assembly_request_requires_version() {
        let json = r#"{"name":"GV 2026","date":null,"location":null}"#;
        let result: Result<UpdateAssemblyRequest, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "UpdateAssemblyRequest must require version field"
        );
    }

    #[test]
    fn test_update_assembly_request_with_version() {
        let version = Uuid::new_v4();
        let json = format!(
            r#"{{"name":"GV","date":null,"location":null,"version":"{}"}}"#,
            version
        );
        let parsed: UpdateAssemblyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "GV");
        assert_eq!(parsed.version, version);
    }
}

#[cfg(test)]
mod repayment_phase_to_tests {
    use super::*;
    use genossi_dao::repayment_phase::RepaymentPhaseStatus;

    fn make_domain() -> genossi_service::repayment_phase::RepaymentPhase {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        genossi_service::repayment_phase::RepaymentPhase {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000,
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_repayment_phase_status_to_roundtrip() {
        // DAO -> TO -> DAO must produce the same enum value for all 3 variants.
        for status in [
            RepaymentPhaseStatus::Preparation,
            RepaymentPhaseStatus::Open,
            RepaymentPhaseStatus::Closed,
        ] {
            let to = RepaymentPhaseStatusTO::from(&status);
            let back: RepaymentPhaseStatus = (&to).into();
            assert_eq!(back, status, "roundtrip must preserve {:?}", status);
        }
    }

    #[test]
    fn test_repayment_phase_to_from_domain() {
        // Each of the 9 RepaymentPhaseTO fields must mirror the domain type
        // verbatim; `created` and `version` are wrapped in Some(...) per the
        // AssemblyTO precedent for optional-on-the-wire fields.
        let domain = make_domain();
        let to = RepaymentPhaseTO::from(&domain);
        assert_eq!(to.id, domain.id);
        assert_eq!(to.fiscal_year, domain.fiscal_year);
        assert_eq!(to.share_value, domain.share_value);
        assert_eq!(to.status, RepaymentPhaseStatusTO::Preparation);
        assert_eq!(to.opened_at, domain.opened_at);
        assert_eq!(to.closed_at, domain.closed_at);
        assert_eq!(to.created, Some(domain.created));
        assert_eq!(to.deleted, domain.deleted);
        assert_eq!(to.version, Some(domain.version));
    }

    #[test]
    fn test_create_repayment_phase_request_serde() {
        // Minimal JSON with exactly the two required fields must deserialize.
        let json = r#"{"fiscal_year":2026,"share_value":12000}"#;
        let parsed: CreateRepaymentPhaseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.fiscal_year, 2026);
        assert_eq!(parsed.share_value, 12000);

        // Re-serialize and verify the shape is structurally equivalent.
        let serialized = serde_json::to_string(&parsed).unwrap();
        let reparsed: CreateRepaymentPhaseRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(reparsed.fiscal_year, 2026);
        assert_eq!(reparsed.share_value, 12000);
    }

    #[test]
    fn test_update_repayment_phase_request_requires_version() {
        // `version: Uuid` is non-optional — JSON without it must fail to
        // deserialize. Guards the optimistic-locking contract on PUT.
        let json = r#"{"fiscal_year":2026,"share_value":12000}"#;
        let result: Result<UpdateRepaymentPhaseRequest, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "UpdateRepaymentPhaseRequest must require version field"
        );

        // With version, deserialization must succeed.
        let version = Uuid::new_v4();
        let json_with = format!(
            r#"{{"fiscal_year":2026,"share_value":12000,"version":"{}"}}"#,
            version
        );
        let parsed: UpdateRepaymentPhaseRequest = serde_json::from_str(&json_with).unwrap();
        assert_eq!(parsed.version, version);
        assert_eq!(parsed.fiscal_year, 2026);
        assert_eq!(parsed.share_value, 12000);
    }
}

#[cfg(test)]
mod repayment_entry_to_tests {
    use super::*;

    fn make_domain_entry() -> genossi_service::repayment_entry::RepaymentEntry {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 31).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        genossi_service::repayment_entry::RepaymentEntry {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 5,
            status: genossi_dao::repayment_entry::RepaymentEntryStatus::Open,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_repayment_entry_status_to_roundtrip() {
        use genossi_dao::repayment_entry::RepaymentEntryStatus as S;
        for status in [S::Open, S::Contacted, S::PaidOut] {
            let to = RepaymentEntryStatusTO::from(&status);
            let back: S = (&to).into();
            assert_eq!(back, status, "roundtrip must preserve {:?}", status);
        }
    }

    #[test]
    fn test_repayment_entry_to_from_domain() {
        // Each of the 8 RepaymentEntryTO fields must mirror the domain type
        // verbatim; `created` and `version` are wrapped in Some(...) per the
        // RepaymentPhaseTO precedent for optional-on-the-wire fields.
        let domain = make_domain_entry();
        let to = RepaymentEntryTO::from(&domain);
        assert_eq!(to.id, domain.id);
        assert_eq!(to.member_id, domain.member_id);
        assert_eq!(to.phase_id, domain.phase_id);
        assert_eq!(to.share_count_to_pay_out, 5);
        assert!(matches!(to.status, RepaymentEntryStatusTO::Open));
        assert_eq!(to.created, Some(domain.created));
        assert_eq!(to.deleted, domain.deleted);
        assert_eq!(to.version, Some(domain.version));
    }

    #[test]
    fn test_create_repayment_entry_request_serde() {
        let phase_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let req = CreateRepaymentEntryRequest {
            phase_id,
            member_id,
            share_count_to_pay_out: 3,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: CreateRepaymentEntryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.share_count_to_pay_out, 3);
        assert_eq!(back.phase_id, phase_id);
        assert_eq!(back.member_id, member_id);
    }

    #[test]
    fn test_update_repayment_entry_request_optional_fields() {
        // share_count_to_pay_out + status sind Optional; version ist Pflicht.
        // JSON ohne version muss fehlschlagen.
        let json_no_version = r#"{"share_count_to_pay_out":3}"#;
        let result: Result<UpdateRepaymentEntryRequest, _> = serde_json::from_str(json_no_version);
        assert!(
            result.is_err(),
            "UpdateRepaymentEntryRequest must require version field"
        );

        // Nur version → share_count_to_pay_out + status bleiben None.
        let json = r#"{"version":"00000000-0000-0000-0000-000000000000"}"#;
        let req: UpdateRepaymentEntryRequest = serde_json::from_str(json).unwrap();
        assert!(req.share_count_to_pay_out.is_none());
        assert!(req.status.is_none());
        assert_eq!(req.version, Uuid::nil());
    }

    #[test]
    fn test_batch_status_request_serde() {
        let req = BatchStatusRequest {
            entry_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            target_status: RepaymentEntryStatusTO::Contacted,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: BatchStatusRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_ids.len(), 2);
        assert!(matches!(
            back.target_status,
            RepaymentEntryStatusTO::Contacted
        ));
    }

    #[test]
    fn test_close_conflict_response_serializes_with_pending_numbers() {
        let resp = CloseConflictResponse {
            error: "Cannot close phase: 3 entries are not paid out and not deleted.".into(),
            pending_count: 3,
            pending_member_numbers: vec!["1".into(), "5".into(), "+1 weitere".into()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"pending_count\":3"));
        assert!(json.contains("\"pending_member_numbers\""));
        assert!(json.contains("weitere"));
    }

    #[test]
    fn test_batch_failure_response_serde() {
        // W-05: structured 409-body for batch-toggle failures
        let failing_id = Uuid::new_v4();
        let resp = BatchFailureResponse {
            failure_index: 1,
            failure_id: failing_id.to_string(),
            failure_reason: "source status is 'PaidOut', expected Open or Contacted".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"failure_index\":1"));
        assert!(json.contains("\"failure_id\""));
        assert!(json.contains("\"failure_reason\""));
        let back: BatchFailureResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.failure_index, 1);
        assert_eq!(back.failure_id, failing_id.to_string());
    }
}

#[cfg(test)]
mod helper_token_to_tests {
    use super::*;
    use std::sync::Arc;
    use time::PrimitiveDateTime;

    fn dummy_entity_open() -> genossi_dao::helper_token::HelperTokenEntity {
        let now = time::OffsetDateTime::now_utc();
        let now_pdt = PrimitiveDateTime::new(now.date(), now.time());
        genossi_dao::helper_token::HelperTokenEntity {
            id: Uuid::nil(),
            assembly_id: Uuid::nil(),
            memo: Arc::from("Anna"),
            token_hash: Arc::from("dummy-hash-not-leaked"),
            // ADR-2026-05-06: dummy plain-text code; the From<&Entity> impl
            // does not expose it on HelperTokenTO (the REST handler attaches
            // code/qr_svg explicitly via the service layer).
            code: Some(Arc::from("ABC1234567")),
            created: now_pdt,
            used_at: None,
            session_id: None,
            revoked_at: None,
            deleted: None,
            version: Uuid::nil(),
        }
    }

    #[test]
    fn test_status_open_when_neither_used_nor_revoked() {
        let entity = dummy_entity_open();
        let to = HelperTokenTO::from(&entity);
        assert_eq!(to.status, HelperTokenStatusTO::Open);
    }

    #[test]
    fn test_status_used_when_used_at_some() {
        let mut entity = dummy_entity_open();
        let now = time::OffsetDateTime::now_utc();
        entity.used_at = Some(PrimitiveDateTime::new(now.date(), now.time()));
        let to = HelperTokenTO::from(&entity);
        assert_eq!(to.status, HelperTokenStatusTO::Used);
    }

    #[test]
    fn test_status_revoked_dominates_used() {
        // D-02 priority: revoked_at.is_some() => Revoked, even if used_at.is_some()
        // (Real-world: never both, but defensive — revoked always wins.)
        let mut entity = dummy_entity_open();
        let now = time::OffsetDateTime::now_utc();
        entity.used_at = Some(PrimitiveDateTime::new(now.date(), now.time()));
        entity.revoked_at = Some(PrimitiveDateTime::new(now.date(), now.time()));
        let to = HelperTokenTO::from(&entity);
        assert_eq!(to.status, HelperTokenStatusTO::Revoked);
    }

    #[test]
    fn test_to_does_not_expose_token_hash() {
        // Defensive serialization-test: D-06 parallel — TO must NOT contain a
        // `token_hash` field (no leak path through OpenAPI / JSON-response).
        let entity = dummy_entity_open();
        let to = HelperTokenTO::from(&entity);
        let json = serde_json::to_string(&to).unwrap();
        assert!(
            !json.contains("token_hash"),
            "JSON must not contain token_hash; got: {}",
            json
        );
        assert!(
            !json.contains("dummy-hash-not-leaked"),
            "JSON must not leak the hash payload"
        );
    }

    #[test]
    fn test_create_response_has_one_time_secrets() {
        // HelperTokenCreateResponseTO carries `code` and `qr_svg` once (D-21).
        let entity = dummy_entity_open();
        let token_to = HelperTokenTO::from(&entity);
        let resp = HelperTokenCreateResponseTO {
            token: token_to,
            code: "ABC1234567".to_string(),
            qr_svg: "<svg/>".to_string(),
        };
        assert_eq!(resp.code.len(), 10);
        assert!(resp.qr_svg.starts_with("<svg"));
    }

    #[test]
    fn test_redeem_request_minimal_json() {
        let json = r#"{"code":"ABC1234567"}"#;
        let parsed: RedeemRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.code, "ABC1234567");
    }

    #[test]
    fn test_redeem_response_carries_assembly_and_expiry() {
        let assembly_id = Uuid::new_v4();
        let resp = RedeemResponse {
            assembly_id,
            expires_at: "2026-05-04T10:00:00.000000000Z".to_string(),
        };
        assert_eq!(resp.assembly_id, assembly_id);
        assert!(resp.expires_at.contains("2026"));
    }

    #[test]
    fn test_create_helper_token_request_json() {
        let json = r#"{"memo":"Anna"}"#;
        let parsed: CreateHelperTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.memo, "Anna");
    }
}

// ============================================================================
// Attendance TOs (Phase 3 Plan 04 -- D-24, D-26, D-28, ATTN-01, ATTN-02, ASSY-04)
// ============================================================================

/// **Reduced helper-view of a member (D-24, ATTN-01)** -- DSGVO-compliant projection.
///
/// **PII-Leak-Guard (Pitfall 6 in 03-RESEARCH.md):** This struct has EXACTLY
/// 7 fields (member_number, first_name, last_name, salutation, title,
/// is_present, member_id).
///
/// **VERBOTEN:** Inserting an `impl From<&MemberTO> for AttendanceMemberTO`
/// would silently propagate new MemberTO fields (e.g. future `iban` /
/// `email` / `bank_account`) and violate ATTN-01. Conversion runs
/// EXCLUSIVELY through `From<&genossi_dao::attendance::AttendanceMemberRow>`
/// -- an explicit 7-field DTO from the DAO layer with the same whitelist.
///
/// Plan 06 (REST E2E tests) verifies the guard with a whitelist+blacklist
/// iteration on the JSON response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceMemberTO {
    /// Mitgliedsnummer (ATTN-01).
    pub member_number: i64,
    /// Vorname (ATTN-01).
    pub first_name: String,
    /// Nachname (ATTN-01).
    pub last_name: String,
    /// Anrede ("Herr"/"Frau"/"Firma" or null).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<String>,
    /// Akademischer Titel.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    /// Aktueller Anwesenheits-Status (ATTN-03/04).
    pub is_present: bool,
    /// Member-ID -- frontend needs this for PUT/DELETE requests on
    /// `/api/attendance/{aid}/{mid}`. Kein PII (UUID).
    pub member_id: Uuid,
}

impl From<&genossi_dao::attendance::AttendanceMemberRow> for AttendanceMemberTO {
    fn from(r: &genossi_dao::attendance::AttendanceMemberRow) -> Self {
        Self {
            member_number: r.member_number,
            first_name: r.first_name.to_string(),
            last_name: r.last_name.to_string(),
            salutation: r.salutation.as_deref().map(String::from),
            title: r.title.as_deref().map(String::from),
            is_present: r.is_present,
            member_id: r.member_id,
        }
    }
}

/// Live counter (ASSY-04). `{present, total}` for the
/// `X von Y aktiven Mitgliedern` display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceStatsTO {
    pub present: u64,
    pub total: u64,
}

impl From<&genossi_service::attendance::AttendanceStats> for AttendanceStatsTO {
    fn from(s: &genossi_service::attendance::AttendanceStats) -> Self {
        Self {
            present: s.present,
            total: s.total,
        }
    }
}

#[cfg(test)]
mod attendance_to_tests {
    use super::*;

    #[test]
    fn test_attendance_member_to_serializes_exactly_seven_keys() {
        let to = AttendanceMemberTO {
            member_number: 42,
            first_name: "Max".to_string(),
            last_name: "Mueller".to_string(),
            salutation: Some("Herr".to_string()),
            title: Some("Dr.".to_string()),
            is_present: true,
            member_id: uuid::Uuid::new_v4(),
        };
        let json = serde_json::to_value(&to).unwrap();
        let keys: std::collections::HashSet<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let allowed: std::collections::HashSet<&str> = [
            "member_number",
            "first_name",
            "last_name",
            "salutation",
            "title",
            "is_present",
            "member_id",
        ]
        .iter()
        .copied()
        .collect();
        assert_eq!(
            keys, allowed,
            "AttendanceMemberTO must serialize exactly 7 fields, got: {:?}",
            keys
        );
    }

    #[test]
    fn test_attendance_member_to_with_none_optionals_skips_them() {
        let to = AttendanceMemberTO {
            member_number: 42,
            first_name: "Max".to_string(),
            last_name: "Mueller".to_string(),
            salutation: None,
            title: None,
            is_present: false,
            member_id: uuid::Uuid::new_v4(),
        };
        let json = serde_json::to_value(&to).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("salutation"));
        assert!(!obj.contains_key("title"));
    }

    #[test]
    fn test_attendance_member_to_does_not_contain_pii_keys() {
        let to = AttendanceMemberTO {
            member_number: 42,
            first_name: "Max".to_string(),
            last_name: "Mueller".to_string(),
            salutation: Some("Herr".to_string()),
            title: Some("Dr.".to_string()),
            is_present: true,
            member_id: uuid::Uuid::new_v4(),
        };
        let json = serde_json::to_value(&to).unwrap();
        for forbidden in [
            "email",
            "iban",
            "bank_account",
            "street",
            "house_number",
            "postal_code",
            "city",
            "comment",
            "join_date",
            "exit_date",
            "birth_date",
            "phone",
        ] {
            assert!(
                json.get(forbidden).is_none(),
                "PII-Leak: AttendanceMemberTO serialized forbidden field '{}'",
                forbidden
            );
        }
    }

    #[test]
    fn test_attendance_member_to_from_attendance_member_row() {
        use std::sync::Arc;
        let row = genossi_dao::attendance::AttendanceMemberRow {
            member_id: uuid::Uuid::new_v4(),
            member_number: 100,
            first_name: Arc::from("Maxi"),
            last_name: Arc::from("Mueller"),
            salutation: Some(Arc::from("Frau")),
            title: None,
            is_present: true,
        };
        let to = AttendanceMemberTO::from(&row);
        assert_eq!(to.member_number, 100);
        assert_eq!(to.first_name, "Maxi");
        assert_eq!(to.last_name, "Mueller");
        assert_eq!(to.salutation, Some("Frau".to_string()));
        assert_eq!(to.title, None);
        assert!(to.is_present);
        assert_eq!(to.member_id, row.member_id);
    }

    #[test]
    fn test_attendance_stats_to_serializes_present_total() {
        let stats = AttendanceStatsTO {
            present: 3,
            total: 10,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["present"], 3);
        assert_eq!(json["total"], 10);
    }

    #[test]
    fn test_attendance_stats_to_from_service_stats() {
        let stats = genossi_service::attendance::AttendanceStats {
            present: 7,
            total: 25,
        };
        let to = AttendanceStatsTO::from(&stats);
        assert_eq!(to.present, 7);
        assert_eq!(to.total, 25);
    }
}

#[cfg(test)]
mod member_slim_to_tests {
    //! Phase 14 Plan 04 — Tests for `MemberSlimTO` (TRSF-06 Slim-DTO with PII guard).
    //!
    //! Verifies (1) the From<&Member> conversion populates exactly the 6 allowed
    //! fields, (2) JSON serialization contains no PII fields (email, bank_account,
    //! street, IBAN, current_shares), and (3) Option fields are skipped when None.
    use super::*;
    use std::sync::Arc;

    fn sample_service_member() -> genossi_service::member::Member {
        genossi_service::member::Member {
            id: uuid::Uuid::new_v4(),
            member_number: 42,
            first_name: Arc::from("Anna"),
            last_name: Arc::from("Schmidt"),
            salutation: Some(Salutation::Frau),
            title: Some(Arc::from("Dr.")),
            email: Some(Arc::from("anna@example.com")),
            company: None,
            comment: None,
            street: Some(Arc::from("Musterstraße")),
            house_number: Some(Arc::from("12")),
            postal_code: Some(Arc::from("12345")),
            city: Some(Arc::from("Berlin")),
            join_date: time::Date::from_calendar_date(2024, time::Month::January, 15).unwrap(),
            shares_at_joining: 1,
            current_shares: 3,
            current_balance: 15000,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: Some(Arc::from("DE89370400440532013000")),
            account_holder: Some(Arc::from("Erika Mustermann")),
            status: MemberStatus::Normal,
            postal_status: PostalStatus::Erreichbar,
            created: {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            },
            deleted: None,
            version: uuid::Uuid::new_v4(),
        }
    }

    #[test]
    fn test_member_slim_to_from_member_populates_six_fields() {
        let m = sample_service_member();
        let slim = MemberSlimTO::from(&m);

        assert_eq!(slim.id, m.id);
        assert_eq!(slim.member_number, 42);
        assert_eq!(slim.salutation, Some(SalutationTO::Frau));
        assert_eq!(slim.title.as_deref(), Some("Dr."));
        assert_eq!(slim.first_name, "Anna");
        assert_eq!(slim.last_name, "Schmidt");
    }

    #[test]
    fn test_member_slim_to_serializes_no_pii_fields() {
        let m = sample_service_member();
        let slim = MemberSlimTO::from(&m);
        let json = serde_json::to_value(&slim).expect("serialize MemberSlimTO");

        // PII-Leak-Guard: these fields MUST NEVER appear in the JSON output.
        let obj = json.as_object().expect("MemberSlimTO serializes to object");
        assert!(!obj.contains_key("email"), "email leaked into MemberSlimTO");
        assert!(
            !obj.contains_key("bank_account"),
            "bank_account leaked into MemberSlimTO"
        );
        // Quick 260607-mw9: account_holder darf NIE in MemberSlimTO landen
        // (PII-Whitelist intakt — Helfer sehen nur Mitgliedsnummer/Name/Titel).
        assert!(
            !obj.contains_key("account_holder"),
            "account_holder leaked into MemberSlimTO"
        );
        assert!(!obj.contains_key("iban"), "iban leaked into MemberSlimTO");
        assert!(
            !obj.contains_key("street"),
            "street leaked into MemberSlimTO"
        );
        assert!(
            !obj.contains_key("current_shares"),
            "current_shares leaked into MemberSlimTO"
        );
        assert!(
            !obj.contains_key("current_balance"),
            "current_balance leaked into MemberSlimTO"
        );
        assert!(
            !obj.contains_key("postal_code"),
            "postal_code leaked into MemberSlimTO"
        );
        assert!(!obj.contains_key("city"), "city leaked into MemberSlimTO");
    }

    #[test]
    fn test_member_slim_to_serializes_exactly_six_keys_when_all_present() {
        let m = sample_service_member();
        let slim = MemberSlimTO::from(&m);
        let json = serde_json::to_value(&slim).expect("serialize MemberSlimTO");
        let obj = json.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "first_name",
                "id",
                "last_name",
                "member_number",
                "salutation",
                "title",
            ]
        );
    }

    #[test]
    fn test_member_slim_to_skips_none_optional_fields() {
        let mut m = sample_service_member();
        m.salutation = None;
        m.title = None;
        let slim = MemberSlimTO::from(&m);
        let json = serde_json::to_value(&slim).expect("serialize MemberSlimTO");
        let obj = json.as_object().expect("object");
        // 4 fields remaining after Option<Salutation> + Option<title> are skipped.
        assert_eq!(
            obj.len(),
            4,
            "salutation+title None should be skipped, leaving 4 fields"
        );
        assert!(!obj.contains_key("salutation"));
        assert!(!obj.contains_key("title"));
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("member_number"));
        assert!(obj.contains_key("first_name"));
        assert!(obj.contains_key("last_name"));
    }

    /// Quick 260607-mw9: MemberTO serializes account_holder when Some.
    #[test]
    fn test_member_to_serializes_account_holder_when_some() {
        let mut m = sample_service_member();
        m.account_holder = Some(Arc::from("Erika Mustermann"));
        let to = MemberTO::from(&m);
        assert_eq!(to.account_holder.as_deref(), Some("Erika Mustermann"));
        let json = serde_json::to_value(&to).expect("serialize MemberTO");
        let obj = json.as_object().expect("MemberTO serializes to object");
        assert_eq!(
            obj.get("account_holder").and_then(|v| v.as_str()),
            Some("Erika Mustermann")
        );
    }

    /// Quick 260607-mw9: MemberTO omits account_holder when None
    /// (skip_serializing_if), so JSON wire output stays compact.
    #[test]
    fn test_member_to_omits_account_holder_when_none() {
        let mut m = sample_service_member();
        m.account_holder = None;
        let to = MemberTO::from(&m);
        assert_eq!(to.account_holder, None);
        let json = serde_json::to_value(&to).expect("serialize MemberTO");
        let obj = json.as_object().expect("MemberTO serializes to object");
        assert!(
            !obj.contains_key("account_holder"),
            "MemberTO with account_holder=None must skip the field in JSON"
        );
    }

    /// Quick 260607-mw9: roundtrip Member → MemberTO → Member preserves
    /// account_holder value.
    #[test]
    fn test_member_to_account_holder_roundtrip() {
        let mut m = sample_service_member();
        m.account_holder = Some(Arc::from("Firma XY GmbH"));
        let to = MemberTO::from(&m);
        let back = genossi_service::member::Member::from(&to);
        assert_eq!(back.account_holder.as_deref(), Some("Firma XY GmbH"));
    }

    /// Quick 260625-e14: postal_status survives Member -> MemberTO -> Member.
    #[test]
    fn test_member_to_postal_status_roundtrip() {
        let mut m = sample_service_member();
        m.postal_status = PostalStatus::Unzustellbar;
        let to = MemberTO::from(&m);
        assert_eq!(to.postal_status, PostalStatusTO::Unzustellbar);
        let back = genossi_service::member::Member::from(&to);
        assert_eq!(back.postal_status, PostalStatus::Unzustellbar);
    }

    /// Quick 260625-e14: missing postal_status in JSON defaults to Erreichbar
    /// (backward compatibility for older clients).
    #[test]
    fn test_member_to_postal_status_defaults_when_absent() {
        let json = r#"{"member_number":1,"first_name":"A","last_name":"B","join_date":"2024-01-15","shares_at_joining":1,"current_shares":1,"current_balance":0}"#;
        let to: MemberTO = serde_json::from_str(json).unwrap();
        assert_eq!(to.postal_status, PostalStatusTO::Erreichbar);
    }
}
