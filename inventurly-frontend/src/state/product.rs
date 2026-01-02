use rest_types::ProductTO;

#[derive(Clone)]
pub struct Product {
    pub items: Vec<ProductTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub search_results: Vec<ProductTO>,
    pub search_loading: bool,
    pub search_query: String,
    pub filter_query: String,
    pub filter_sales_units: Vec<String>,
    pub filter_requires_weighing: Option<bool>,
    pub filter_price_min: Option<i64>,
    pub filter_price_max: Option<i64>,
    pub filter_rack_assignment: Option<bool>, // None=all, Some(true)=assigned, Some(false)=unassigned
}

impl Default for Product {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
            search_results: vec![],
            search_loading: false,
            search_query: String::new(),
            filter_query: String::new(),
            filter_sales_units: vec![],
            filter_requires_weighing: None,
            filter_price_min: None,
            filter_price_max: None,
            filter_rack_assignment: None,
        }
    }
}
