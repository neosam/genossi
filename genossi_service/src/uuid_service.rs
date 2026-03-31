use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UuidService: Clone + Send + Sync {
    async fn new_v4(&self) -> Uuid;
}

mockall::mock! {
    pub UuidService {}

    impl Clone for UuidService {
        fn clone(&self) -> Self;
    }

    #[async_trait]
    impl UuidService for UuidService {
        async fn new_v4(&self) -> Uuid;
    }
}
