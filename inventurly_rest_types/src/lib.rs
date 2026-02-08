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
#[derive(Clone, Debug, Copy, PartialEq, Eq, ToSchema, Default)]
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "4006381333931")]
    pub deposit_ean: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 3)]
    pub rack_count: Option<i64>,
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
            deposit_ean: product.deposit_ean.as_ref().map(|s| s.to_string()),
            created: Some(product.created),
            deleted: product.deleted,
            version: Some(product.version),
            rack_count: None,
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
            deposit_ean: to.deposit_ean.as_ref().map(|s| Arc::from(s.as_str())),
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
    #[schema(example = 1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
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
            sort_order: Some(product_rack.sort_order),
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
            sort_order: to.sort_order.unwrap_or(0),
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReorderProductsInRackRequestTO {
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    /// List of product IDs in desired order
    pub product_order: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SetProductPositionRequestTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub product_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    #[schema(example = 3)]
    pub position: i32,
}

/// Container-Rack relationship (junction between containers and racks)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ContainerRackTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub container_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    #[schema(example = 1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
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
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Uuid>,
}

impl From<&inventurly_service::container_rack::ContainerRack> for ContainerRackTO {
    fn from(container_rack: &inventurly_service::container_rack::ContainerRack) -> Self {
        Self {
            container_id: container_rack.container_id,
            rack_id: container_rack.rack_id,
            sort_order: Some(container_rack.sort_order),
            created: Some(container_rack.created),
            deleted: container_rack.deleted,
            version: Some(container_rack.version),
        }
    }
}

impl From<&ContainerRackTO> for inventurly_service::container_rack::ContainerRack {
    fn from(to: &ContainerRackTO) -> Self {
        Self {
            container_id: to.container_id,
            rack_id: to.rack_id,
            sort_order: to.sort_order.unwrap_or(0),
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
pub struct AddContainerToRackRequestTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub container_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReorderContainersInRackRequestTO {
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    /// List of container IDs in desired order
    pub container_order: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SetContainerPositionRequestTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub container_id: Uuid,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub rack_id: Uuid,
    #[schema(example = 3)]
    pub position: i32,
}

/// Inventur (Inventory counting session)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InventurTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "Q1 2025 Inventory Count")]
    pub name: String,
    #[schema(example = "Annual inventory count for Q1")]
    pub description: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-01T08:00:00Z")]
    pub start_date: Option<time::PrimitiveDateTime>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-31T18:00:00Z")]
    pub end_date: Option<time::PrimitiveDateTime>,
    #[schema(example = "draft")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "admin")]
    pub created_by: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2024-12-15T10:30:00Z")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "abc123xyz789...")]
    pub token: Option<String>,
}

impl From<&inventurly_service::inventur::Inventur> for InventurTO {
    fn from(inventur: &inventurly_service::inventur::Inventur) -> Self {
        Self {
            id: Some(inventur.id),
            name: inventur.name.to_string(),
            description: inventur.description.to_string(),
            start_date: Some(inventur.start_date),
            end_date: inventur.end_date,
            status: inventur.status.to_string(),
            created_by: Some(inventur.created_by.to_string()),
            created: Some(inventur.created),
            deleted: inventur.deleted,
            version: Some(inventur.version),
            token: inventur.token.as_ref().map(|t| t.to_string()),
        }
    }
}

impl From<&InventurTO> for inventurly_service::inventur::Inventur {
    fn from(to: &InventurTO) -> Self {
        use std::sync::Arc;
        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            name: Arc::from(to.name.as_str()),
            description: Arc::from(to.description.as_str()),
            start_date: to.start_date.unwrap_or(now_primitive),
            end_date: to.end_date,
            status: Arc::from(to.status.as_str()),
            // created_by will be overwritten by the service layer from auth context
            created_by: Arc::from(to.created_by.as_deref().unwrap_or("UNKNOWN")),
            created: to.created.unwrap_or(now_primitive),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
            token: to.token.as_ref().map(|t| Arc::from(t.as_str())),
        }
    }
}

/// InventurMeasurement (Individual product measurement in an inventur)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InventurMeasurementTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub inventur_id: Uuid,
    #[schema(example = "789e0123-e89b-12d3-a456-426614174000")]
    pub product_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "abc1234-e89b-12d3-a456-426614174000")]
    pub rack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "def5678-e89b-12d3-a456-426614174000")]
    pub container_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 50)]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 1500)]
    pub weight_grams: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "counter1")]
    pub measured_by: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-15T14:30:00Z")]
    pub measured_at: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Counted in storage area A")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "unreviewed")]
    pub review_state: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-15T14:30:00Z")]
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

impl From<&inventurly_service::inventur_measurement::InventurMeasurement> for InventurMeasurementTO {
    fn from(measurement: &inventurly_service::inventur_measurement::InventurMeasurement) -> Self {
        Self {
            id: Some(measurement.id),
            inventur_id: measurement.inventur_id,
            product_id: measurement.product_id,
            rack_id: measurement.rack_id,
            container_id: measurement.container_id,
            count: measurement.count,
            weight_grams: measurement.weight_grams,
            measured_by: Some(measurement.measured_by.to_string()),
            measured_at: Some(measurement.measured_at),
            notes: measurement.notes.as_ref().map(|n| n.to_string()),
            review_state: Some(measurement.review_state.to_string()),
            created: Some(measurement.created),
            deleted: measurement.deleted,
            version: Some(measurement.version),
        }
    }
}

impl From<&InventurMeasurementTO> for inventurly_service::inventur_measurement::InventurMeasurement {
    fn from(to: &InventurMeasurementTO) -> Self {
        use std::sync::Arc;
        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            inventur_id: to.inventur_id,
            product_id: to.product_id,
            rack_id: to.rack_id,
            container_id: to.container_id,
            count: to.count,
            weight_grams: to.weight_grams,
            // measured_by will be overwritten by the service layer from auth context
            measured_by: Arc::from(to.measured_by.as_deref().unwrap_or("UNKNOWN")),
            measured_at: to.measured_at.unwrap_or(now_primitive),
            notes: to.notes.as_ref().map(|n| Arc::from(n.as_str())),
            review_state: Arc::from(to.review_state.as_deref().unwrap_or("unreviewed")),
            created: to.created.unwrap_or(now_primitive),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InventurCustomEntryTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Option<Uuid>,
    #[schema(example = "456e7890-e89b-12d3-a456-426614174000")]
    pub inventur_id: Uuid,
    #[schema(example = "Unknown Product XYZ")]
    pub custom_product_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "abc1234-e89b-12d3-a456-426614174000")]
    pub rack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "def5678-e89b-12d3-a456-426614174000")]
    pub container_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 25)]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 2500)]
    pub weight_grams: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "counter1")]
    pub measured_by: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-15T14:30:00Z")]
    pub measured_at: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Product not in system, needs investigation")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "4006381333931")]
    pub ean: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "unreviewed")]
    pub review_state: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    #[schema(example = "2025-01-15T14:30:00Z")]
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

impl From<&inventurly_service::inventur_custom_entry::InventurCustomEntry> for InventurCustomEntryTO {
    fn from(entry: &inventurly_service::inventur_custom_entry::InventurCustomEntry) -> Self {
        Self {
            id: Some(entry.id),
            inventur_id: entry.inventur_id,
            custom_product_name: entry.custom_product_name.to_string(),
            rack_id: entry.rack_id,
            container_id: entry.container_id,
            count: entry.count,
            weight_grams: entry.weight_grams,
            measured_by: Some(entry.measured_by.to_string()),
            measured_at: Some(entry.measured_at),
            notes: entry.notes.as_ref().map(|n| n.to_string()),
            ean: entry.ean.as_ref().map(|e| e.to_string()),
            review_state: Some(entry.review_state.to_string()),
            created: Some(entry.created),
            deleted: entry.deleted,
            version: Some(entry.version),
        }
    }
}

impl From<&InventurCustomEntryTO> for inventurly_service::inventur_custom_entry::InventurCustomEntry {
    fn from(to: &InventurCustomEntryTO) -> Self {
        use std::sync::Arc;
        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        Self {
            id: to.id.unwrap_or_else(Uuid::nil),
            inventur_id: to.inventur_id,
            custom_product_name: Arc::from(to.custom_product_name.as_str()),
            rack_id: to.rack_id,
            container_id: to.container_id,
            count: to.count,
            weight_grams: to.weight_grams,
            // measured_by will be overwritten by the service layer from auth context
            measured_by: Arc::from(to.measured_by.as_deref().unwrap_or("UNKNOWN")),
            measured_at: to.measured_at.unwrap_or(now_primitive),
            notes: to.notes.as_ref().map(|n| Arc::from(n.as_str())),
            ean: to.ean.as_ref().map(|e| Arc::from(e.as_str())),
            review_state: Arc::from(to.review_state.as_deref().unwrap_or("unreviewed")),
            created: to.created.unwrap_or(now_primitive),
            deleted: to.deleted,
            version: to.version.unwrap_or_else(Uuid::nil),
        }
    }
}

/// Request body for changing inventur status
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ChangeInventurStatusRequestTO {
    #[schema(example = "active")]
    pub status: String,
}

/// Rack information with ID and name for report items
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RackMeasuredTO {
    /// Rack UUID
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    /// Rack name
    #[schema(example = "Regal A")]
    pub name: String,
}

impl From<&inventurly_service::inventur_report::RackMeasured> for RackMeasuredTO {
    fn from(rack: &inventurly_service::inventur_report::RackMeasured) -> Self {
        Self {
            id: rack.id,
            name: rack.name.to_string(),
        }
    }
}

/// Aggregated product data for an inventur report
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InventurProductReportItemTO {
    /// Product UUID (None for custom entries without linked product)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<Uuid>,
    /// Product EAN code
    #[schema(example = "4260474470041")]
    pub ean: String,
    /// Full product name
    #[schema(example = "Macadamia süss salzig")]
    pub product_name: String,
    /// Short product name
    #[schema(example = "Macadamia süss")]
    pub short_name: String,
    /// Total count of units measured (summed across all measurements)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 15)]
    pub total_count: Option<i64>,
    /// Total weight in grams measured (summed across all measurements)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 2850)]
    pub total_weight_grams: Option<i64>,
    /// Number of individual measurements for this product
    #[schema(example = 3)]
    pub measurement_count: usize,
    /// List of racks where this product was measured (with ID and name)
    pub racks_measured: Vec<RackMeasuredTO>,
    /// Unit price in cents (None if product not found in database)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 539)]
    pub price_cents: Option<i64>,
    /// Calculated total value in cents based on count/weight (None if can't calculate)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 8085)]
    pub total_value_cents: Option<i64>,
}

impl From<&inventurly_service::inventur_report::InventurProductReportItem>
    for InventurProductReportItemTO
{
    fn from(item: &inventurly_service::inventur_report::InventurProductReportItem) -> Self {
        Self {
            product_id: item.product_id,
            ean: item.ean.to_string(),
            product_name: item.product_name.to_string(),
            short_name: item.short_name.to_string(),
            total_count: item.total_count,
            total_weight_grams: item.total_weight_grams,
            measurement_count: item.measurement_count,
            racks_measured: item.racks_measured.iter().map(RackMeasuredTO::from).collect(),
            price_cents: item.price_cents,
            total_value_cents: item.total_value_cents,
        }
    }
}

/// Statistics summary for an inventur
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InventurStatisticsTO {
    /// Total monetary value in cents
    #[schema(example = 125000)]
    pub total_value_cents: i64,
    /// Total number of measurements + custom entries
    #[schema(example = 150)]
    pub total_entries: usize,
    /// Number of distinct products with at least one positive entry
    #[schema(example = 45)]
    pub products_with_entries: usize,
}

impl From<&inventurly_service::inventur_report::InventurStatistics> for InventurStatisticsTO {
    fn from(stats: &inventurly_service::inventur_report::InventurStatistics) -> Self {
        Self {
            total_value_cents: stats.total_value_cents,
            total_entries: stats.total_entries,
            products_with_entries: stats.products_with_entries,
        }
    }
}
