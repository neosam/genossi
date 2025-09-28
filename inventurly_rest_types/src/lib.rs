use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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