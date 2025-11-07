use serde::{Deserialize, Serialize};
use uuid::Uuid;
use time::PrimitiveDateTime;

// Custom serialization module for ISO8601 datetime format
mod iso8601_datetime {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::PrimitiveDateTime;
    use time::format_description::well_known::Iso8601;

    pub fn serialize<S>(
        datetime: &Option<PrimitiveDateTime>, 
        serializer: S
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match datetime {
            Some(dt) => {
                let formatted = dt.assume_utc().format(&Iso8601::DEFAULT)
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
            Some(s) => {
                PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            None => Ok(None),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub age: i32,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// Custom Price type for monetary values
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct Price {
    cents: i64,
}

impl Price {
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }
    
    pub fn to_cents(&self) -> i64 {
        self.cents
    }
    
    pub fn to_euros(&self) -> f64 {
        self.cents as f64 / 100.0
    }
}

impl Serialize for Price {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.cents)
    }
}

impl<'de> Deserialize<'de> for Price {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let cents = i64::deserialize(deserializer)?;
        Ok(Self { cents })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProductTO {
    pub id: Option<Uuid>,
    pub ean: String,
    pub name: String,
    pub short_name: String,
    pub sales_unit: String,
    pub requires_weighing: bool,
    pub price: Price,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// Authentication types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserTO {
    pub username: String,
    pub roles: Vec<String>,
    pub privileges: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// Duplicate Detection types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateDetectionConfigTO {
    pub similarity_threshold: f64,
    pub exact_match_weight: f64,
    pub word_order_weight: f64,
    pub levenshtein_weight: f64,
    pub jaro_winkler_weight: f64,
    pub category_aware: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlgorithmScoresTO {
    pub exact_match: f64,
    pub word_order: f64,
    pub levenshtein: f64,
    pub jaro_winkler: f64,
    pub category_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MatchConfidenceTO {
    VeryHigh,
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateMatchTO {
    pub product: ProductTO,
    pub similarity_score: f64,
    pub algorithm_scores: AlgorithmScoresTO,
    pub confidence: MatchConfidenceTO,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateDetectionResultTO {
    pub checked_product: ProductTO,
    pub matches: Vec<DuplicateMatchTO>,
    pub config: DuplicateDetectionConfigTO,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckDuplicateRequestTO {
    pub name: String,
    pub sales_unit: String,
    pub requires_weighing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RackTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductRackTO {
    pub product_id: Uuid,
    pub rack_id: Uuid,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddProductToRackRequestTO {
    pub product_id: Uuid,
    pub rack_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub weight_grams: i64,
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

// Inventur types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventurTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub start_date: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub end_date: Option<PrimitiveDateTime>,
    pub status: String,  // "draft", "active", "completed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventurMeasurementTO {
    pub id: Option<Uuid>,
    pub inventur_id: Uuid,
    pub product_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_grams: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured_by: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub measured_at: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeInventurStatusRequestTO {
    pub status: String,
}

