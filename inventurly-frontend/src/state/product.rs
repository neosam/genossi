use rest_types::ProductTO;

#[derive(Clone)]
pub struct Product {
    pub items: Vec<ProductTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub search_results: Vec<ProductTO>,
    pub search_loading: bool,
    pub search_query: String,
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
        }
    }
}
