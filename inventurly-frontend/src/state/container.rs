use rest_types::ContainerTO;

#[derive(Clone)]
pub struct Container {
    pub items: Vec<ContainerTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}
