use rest_types::RackTO;

#[derive(Clone)]
pub struct Rack {
    pub items: Vec<RackTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for Rack {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}