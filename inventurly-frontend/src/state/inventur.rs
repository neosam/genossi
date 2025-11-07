use rest_types::{InventurTO, InventurMeasurementTO};
use time::PrimitiveDateTime;

#[derive(Clone)]
pub struct Inventur {
    pub items: Vec<InventurTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub filter_date_from: Option<PrimitiveDateTime>,
    pub filter_date_to: Option<PrimitiveDateTime>,
}

impl Default for Inventur {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
            filter_date_from: None,
            filter_date_to: None,
        }
    }
}

#[derive(Clone)]
pub struct InventurMeasurement {
    pub items: Vec<InventurMeasurementTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for InventurMeasurement {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}
