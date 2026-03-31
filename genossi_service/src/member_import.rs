use async_trait::async_trait;
use std::fmt::Debug;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberImportError {
    pub row: usize,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<MemberImportError>,
}

#[async_trait]
pub trait MemberImportService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;

    async fn import_members(
        &self,
        data: &[u8],
        context: Authentication<Self::Context>,
    ) -> Result<MemberImportResult, ServiceError>;
}
