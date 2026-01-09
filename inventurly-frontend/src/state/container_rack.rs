use rest_types::ContainerRackTO;

#[derive(Clone)]
pub struct ContainerRack {
    pub items: Vec<ContainerRackTO>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for ContainerRack {
    fn default() -> Self {
        Self {
            items: vec![],
            loading: false,
            error: None,
        }
    }
}
