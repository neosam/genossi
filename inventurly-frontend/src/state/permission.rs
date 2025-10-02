use rest_types::PermissionTO;

#[derive(Clone)]
pub struct Permission {
    pub items: Vec<PermissionTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for Permission {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}