use rest_types::ProductTO;

#[derive(Clone)]
pub struct Product {
    pub items: Vec<ProductTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for Product {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}