use rest_types::InventurCustomEntryTO;
use uuid::Uuid;

#[derive(Clone)]
pub struct CustomEntry {
    pub items: Vec<InventurCustomEntryTO>,
    pub loading: bool,
    pub error: Option<String>,
    // Filters
    pub filter_query: String,
    pub filter_has_ean: Option<bool>, // None=all, Some(true)=has EAN, Some(false)=no EAN
    pub filter_rack_ids: Vec<Uuid>,
    pub filter_measured_by: Vec<String>,
    pub filter_review_state: Option<String>, // None=all, Some("unreviewed"), Some("reviewed")
}

impl Default for CustomEntry {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
            filter_query: String::new(),
            filter_has_ean: None,
            filter_rack_ids: vec![],
            filter_measured_by: vec![],
            filter_review_state: None,
        }
    }
}
