use async_trait::async_trait;
use genossi_dao::audit_timestamp::AuditTimestampEntry;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum TimestampError {
    DataAccess(Arc<str>),
    TsaError(Arc<str>),
    NotFound,
    NotConfigured,
    NothingToTimestamp,
    DuplicateHash,
}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampError::DataAccess(e) => write!(f, "Data access error: {}", e),
            TimestampError::TsaError(e) => write!(f, "TSA error: {}", e),
            TimestampError::NotFound => write!(f, "Timestamp not found"),
            TimestampError::NotConfigured => write!(f, "TSA not configured"),
            TimestampError::NothingToTimestamp => write!(f, "No audit entries to timestamp"),
            TimestampError::DuplicateHash => {
                write!(f, "Hash unchanged since last timestamp")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimestampVerification {
    pub token_valid: bool,
    pub hash_matches: bool,
    pub audit_log_consistent: bool,
    pub timestamp: time::PrimitiveDateTime,
    pub audit_hash: Arc<str>,
}

#[automock]
#[async_trait]
pub trait TimestampService: Send + Sync + 'static {
    async fn create_timestamp(&self) -> Result<AuditTimestampEntry, TimestampError>;
    async fn get_all(&self) -> Result<Arc<[AuditTimestampEntry]>, TimestampError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditTimestampEntry>, TimestampError>;
    async fn verify(&self, id: Uuid) -> Result<TimestampVerification, TimestampError>;
}
