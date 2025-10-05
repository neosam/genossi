use rest_types::ProductRackTO;

#[derive(Clone)]
pub struct ProductRack {
    pub items: Vec<ProductRackTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for ProductRack {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}