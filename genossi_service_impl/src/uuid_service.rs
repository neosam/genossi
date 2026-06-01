use async_trait::async_trait;
use genossi_service::uuid_service::UuidService;
use uuid::Uuid;

#[derive(Clone)]
pub struct UuidServiceImpl;

impl Default for UuidServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl UuidServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UuidService for UuidServiceImpl {
    async fn new_v4(&self) -> Uuid {
        Uuid::new_v4()
    }
}
