use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// Custom serialization module for ISO8601 datetime format
mod iso8601_datetime {
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PersonTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = 30)]
    pub age: i32,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T12:45:00Z")]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::person::Person> for PersonTO {
    fn from(person: &inventurly_service::person::Person) -> Self {
        Self {
            id: Some(person.id),
            name: person.name.to_string(),
            age: person.age,
            created: Some(person.created),
            deleted: person.deleted,
            version: Some(person.version),
        }
    }
}

impl From<&PersonTO> for inventurly_service::person::Person {
    fn from(to: &PersonTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            name: Arc::from(to.name.as_str()),
            age: to.age,
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

// Custom Price type for monetary values
#[derive(Clone, Debug, Copy, PartialEq, Eq, ToSchema)]
#[schema(example = 599)]
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

impl From<inventurly_service::product::Price> for Price {
    fn from(price: inventurly_service::product::Price) -> Self {
        Self::from_cents(price.to_cents())
    }
}

impl From<Price> for inventurly_service::product::Price {
    fn from(price: Price) -> Self {
        inventurly_service::product::Price::from_cents(price.cents)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProductTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "4260474470041")]
    pub ean: String,
    #[schema(example = "Macadamia süss salzig")]
    pub name: String,
    #[schema(example = "Macadamia süss")]
    pub short_name: String,
    #[schema(example = "130g")]
    pub sales_unit: String,
    #[schema(example = false)]
    pub requires_weighing: bool,
    #[schema(example = 539)]
    pub price: Price,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T12:45:00Z")]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::product::Product> for ProductTO {
    fn from(product: &inventurly_service::product::Product) -> Self {
        Self {
            id: Some(product.id),
            ean: product.ean.to_string(),
            name: product.name.to_string(),
            short_name: product.short_name.to_string(),
            sales_unit: product.sales_unit.to_string(),
            requires_weighing: product.requires_weighing,
            price: product.price.into(),
            created: Some(product.created),
            deleted: product.deleted,
            version: Some(product.version),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RackTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "Rack A")]
    pub name: String,
    #[schema(example = "Storage rack for products")]
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T12:45:00Z")]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::rack::Rack> for RackTO {
    fn from(rack: &inventurly_service::rack::Rack) -> Self {
        Self {
            id: Some(rack.id),
            name: rack.name.to_string(),
            description: rack.description.to_string(),
            created: Some(rack.created),
            deleted: rack.deleted,
            version: Some(rack.version),
        }
    }
}

impl From<&RackTO> for inventurly_service::rack::Rack {
    fn from(to: &RackTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            name: Arc::from(to.name.as_str()),
            description: Arc::from(to.description.as_str()),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

impl From<&ProductTO> for inventurly_service::product::Product {
    fn from(to: &ProductTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            ean: Arc::from(to.ean.as_str()),
            name: Arc::from(to.name.as_str()),
            short_name: Arc::from(to.short_name.as_str()),
            sales_unit: Arc::from(to.sales_unit.as_str()),
            requires_weighing: to.requires_weighing,
            price: to.price.into(),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

// Duplicate Detection API Types

/// Configuration for duplicate detection algorithms
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateDetectionConfigTO {
    /// Minimum similarity threshold for considering products as duplicates (0.0 to 1.0). Default: 0.55
    #[schema(example = 0.55)]
    pub similarity_threshold: f64,
    /// Weight for case-insensitive exact match (0.0 to 1.0). Reduced because exact matches are rare
    #[schema(example = 0.3)]
    pub exact_match_weight: f64,
    /// Weight for word order variations (0.0 to 1.0). Important for German text
    #[schema(example = 0.4)]
    pub word_order_weight: f64,
    /// Weight for Levenshtein distance similarity (0.0 to 1.0). Good for typos
    #[schema(example = 0.2)]
    pub levenshtein_weight: f64,
    /// Weight for Jaro-Winkler similarity (0.0 to 1.0). Supplementary algorithm
    #[schema(example = 0.1)]
    pub jaro_winkler_weight: f64,
    /// Enable category-aware matching (consider sales_unit, etc.)
    #[schema(example = true)]
    pub category_aware: bool,
}

impl From<inventurly_service::duplicate_detection::DuplicateDetectionConfig>
    for DuplicateDetectionConfigTO
{
    fn from(config: inventurly_service::duplicate_detection::DuplicateDetectionConfig) -> Self {
        Self {
            similarity_threshold: config.similarity_threshold,
            exact_match_weight: config.exact_match_weight,
            word_order_weight: config.word_order_weight,
            levenshtein_weight: config.levenshtein_weight,
            jaro_winkler_weight: config.jaro_winkler_weight,
            category_aware: config.category_aware,
        }
    }
}

impl From<DuplicateDetectionConfigTO>
    for inventurly_service::duplicate_detection::DuplicateDetectionConfig
{
    fn from(to: DuplicateDetectionConfigTO) -> Self {
        Self {
            similarity_threshold: to.similarity_threshold,
            exact_match_weight: to.exact_match_weight,
            word_order_weight: to.word_order_weight,
            levenshtein_weight: to.levenshtein_weight,
            jaro_winkler_weight: to.jaro_winkler_weight,
            category_aware: to.category_aware,
        }
    }
}

/// Detailed scores from individual similarity algorithms
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AlgorithmScoresTO {
    /// Case-insensitive exact match score (0.0 or 1.0). Usually 0.0 for real-world data
    #[schema(example = 0.0)]
    pub exact_match: f64,
    /// Word order variation score (0.0 to 1.0). High for German word reordering
    #[schema(example = 1.0)]
    pub word_order: f64,
    /// Levenshtein distance similarity (0.0 to 1.0). Good for typo detection
    #[schema(example = 0.65)]
    pub levenshtein: f64,
    /// Jaro-Winkler similarity (0.0 to 1.0). Supplementary algorithm
    #[schema(example = 0.72)]
    pub jaro_winkler: f64,
    /// Category compatibility score (0.0 to 1.0). 1.0 for same category
    #[schema(example = 1.0)]
    pub category_score: f64,
}

impl From<inventurly_service::duplicate_detection::AlgorithmScores> for AlgorithmScoresTO {
    fn from(scores: inventurly_service::duplicate_detection::AlgorithmScores) -> Self {
        Self {
            exact_match: scores.exact_match,
            word_order: scores.word_order,
            levenshtein: scores.levenshtein,
            jaro_winkler: scores.jaro_winkler,
            category_score: scores.category_score,
        }
    }
}

/// Confidence level of duplicate match
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub enum MatchConfidenceTO {
    /// Very high confidence (>= 0.95)
    VeryHigh,
    /// High confidence (>= 0.85)
    High,
    /// Medium confidence (>= 0.7)
    Medium,
    /// Low confidence (>= threshold)
    Low,
}

impl From<inventurly_service::duplicate_detection::MatchConfidence> for MatchConfidenceTO {
    fn from(confidence: inventurly_service::duplicate_detection::MatchConfidence) -> Self {
        match confidence {
            inventurly_service::duplicate_detection::MatchConfidence::VeryHigh => Self::VeryHigh,
            inventurly_service::duplicate_detection::MatchConfidence::High => Self::High,
            inventurly_service::duplicate_detection::MatchConfidence::Medium => Self::Medium,
            inventurly_service::duplicate_detection::MatchConfidence::Low => Self::Low,
        }
    }
}

/// Result of duplicate detection for a single product
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateMatchTO {
    /// The potentially duplicate product
    pub product: ProductTO,
    /// Overall similarity score (0.0 to 1.0). Typical range: 0.55-0.75 for good duplicates
    #[schema(example = 0.62)]
    pub similarity_score: f64,
    /// Breakdown of individual algorithm scores
    pub algorithm_scores: AlgorithmScoresTO,
    /// Confidence level of the match
    pub confidence: MatchConfidenceTO,
}

impl From<inventurly_service::duplicate_detection::DuplicateMatch> for DuplicateMatchTO {
    fn from(duplicate_match: inventurly_service::duplicate_detection::DuplicateMatch) -> Self {
        Self {
            product: ProductTO::from(&duplicate_match.product),
            similarity_score: duplicate_match.similarity_score,
            algorithm_scores: AlgorithmScoresTO::from(duplicate_match.algorithm_scores),
            confidence: MatchConfidenceTO::from(duplicate_match.confidence),
        }
    }
}

/// Result of duplicate detection operation
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateDetectionResultTO {
    /// Input product that was checked
    pub checked_product: ProductTO,
    /// List of potential duplicate matches
    pub matches: Vec<DuplicateMatchTO>,
    /// Configuration used for detection
    pub config: DuplicateDetectionConfigTO,
}

impl From<inventurly_service::duplicate_detection::DuplicateDetectionResult>
    for DuplicateDetectionResultTO
{
    fn from(result: inventurly_service::duplicate_detection::DuplicateDetectionResult) -> Self {
        Self {
            checked_product: ProductTO::from(&result.checked_product),
            matches: result
                .matches
                .into_iter()
                .map(DuplicateMatchTO::from)
                .collect(),
            config: DuplicateDetectionConfigTO::from(result.config),
        }
    }
}

/// Request body for checking potential duplicates
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckDuplicateRequestTO {
    /// Product name to check for duplicates
    #[schema(example = "Macadamia süß salzig")]
    pub name: String,
    /// Sales unit (e.g., "130g", "1kg")
    #[schema(example = "130g")]
    pub sales_unit: String,
    /// Whether this product requires weighing
    #[schema(example = false)]
    pub requires_weighing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProductRackTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub product_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T12:45:00Z")]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::product_rack::ProductRack> for ProductRackTO {
    fn from(product_rack: &inventurly_service::product_rack::ProductRack) -> Self {
        Self {
            product_id: product_rack.product_id,
            rack_id: product_rack.rack_id,
            created: Some(product_rack.created),
            deleted: product_rack.deleted,
            version: Some(product_rack.version),
        }
    }
}

impl From<&ProductRackTO> for inventurly_service::product_rack::ProductRack {
    fn from(to: &ProductRackTO) -> Self {
        Self {
            product_id: to.product_id,
            rack_id: to.rack_id,
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(uuid::Uuid::nil),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContainerTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "Storage Container A")]
    pub name: String,
    #[schema(example = 1500)]
    pub weight_grams: i64,
    #[schema(example = "Large storage container for products")]
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-01-15T12:45:00Z")]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::container::Container> for ContainerTO {
    fn from(container: &inventurly_service::container::Container) -> Self {
        Self {
            id: Some(container.id),
            name: container.name.to_string(),
            weight_grams: container.weight_grams,
            description: container.description.to_string(),
            created: Some(container.created),
            deleted: container.deleted,
            version: Some(container.version),
        }
    }
}

impl From<&ContainerTO> for inventurly_service::container::Container {
    fn from(to: &ContainerTO) -> Self {
        use std::sync::Arc;
        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            name: Arc::from(to.name.as_str()),
            weight_grams: to.weight_grams,
            description: Arc::from(to.description.as_str()),
            created: to.created.unwrap_or_else(|| {
                let now = time::OffsetDateTime::now_utc();
                time::PrimitiveDateTime::new(now.date(), now.time())
            }),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

/// Request body for adding a product to a rack
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AddProductToRackRequestTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub product_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
}
