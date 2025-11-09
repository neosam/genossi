use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;

use crate::TransactionImpl;
use inventurly_dao::{
    permission::{PermissionDao, PrivilegeEntity, RoleEntity, SessionEntity, UserEntity},
    DaoError,
};

pub struct PermissionDaoImpl {
    pool: Arc<SqlitePool>,
}

impl PermissionDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Helper to parse timestamp from SQLite TEXT
    fn parse_timestamp(timestamp: Option<String>) -> Option<PrimitiveDateTime> {
        timestamp.and_then(|ts| {
            time::PrimitiveDateTime::parse(
                &ts,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
            .or_else(|_| {
                // Fallback: try SQLite datetime format
                let format = time::macros::format_description!(
                    "[year]-[month]-[day] [hour]:[minute]:[second]"
                );
                time::PrimitiveDateTime::parse(&ts, format)
            })
            .ok()
        })
    }

    /// Helper to format timestamp for SQLite
    fn format_timestamp(timestamp: Option<PrimitiveDateTime>) -> Option<String> {
        timestamp.map(|ts| {
            ts.format(&time::format_description::well_known::Iso8601::DEFAULT)
                .unwrap_or_default()
        })
    }
}

#[async_trait]
impl PermissionDao for PermissionDaoImpl {
    type Transaction = TransactionImpl;

    async fn has_privilege(&self, user: &str, privilege: &str) -> Result<bool, DaoError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) > 0 as has_privilege
            FROM user u
            INNER JOIN user_role ur ON u.name = ur.user_name
            INNER JOIN role_privilege rp ON ur.role_name = rp.role_name
            INNER JOIN privilege p ON rp.privilege_name = p.name
            WHERE u.name = ? AND p.name = ?
            "#,
            user,
            privilege
        )
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(result != 0)
    }

    async fn all_users(&self) -> Result<Arc<[UserEntity]>, DaoError> {
        let users =
            sqlx::query!("SELECT name, update_timestamp, update_process FROM user ORDER BY name")
                .fetch_all(self.pool.as_ref())
                .await
                .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let user_entities: Vec<UserEntity> = users
            .into_iter()
            .map(|row| UserEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(user_entities))
    }

    async fn get_user(&self, name: &str) -> Result<Option<UserEntity>, DaoError> {
        let user = sqlx::query!(
            "SELECT name, update_timestamp, update_process FROM user WHERE name = ?",
            name
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(user.map(|row| UserEntity {
            name: Arc::from(row.name.as_str()),
            update_timestamp: Self::parse_timestamp(row.update_timestamp),
            update_process: Arc::from(row.update_process.as_str()),
        }))
    }

    async fn create_user(&self, user: &UserEntity, process: &str) -> Result<(), DaoError> {
        let timestamp = Self::format_timestamp(user.update_timestamp);
        let name = user.name.as_ref();

        sqlx::query!(
            "INSERT INTO user (name, update_timestamp, update_process) VALUES (?, ?, ?)",
            name,
            timestamp,
            process
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn delete_user(&self, username: &str) -> Result<(), DaoError> {
        sqlx::query!("DELETE FROM user WHERE name = ?", username)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn all_roles(&self) -> Result<Arc<[RoleEntity]>, DaoError> {
        let roles =
            sqlx::query!("SELECT name, update_timestamp, update_process FROM role ORDER BY name")
                .fetch_all(self.pool.as_ref())
                .await
                .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let role_entities: Vec<RoleEntity> = roles
            .into_iter()
            .map(|row| RoleEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(role_entities))
    }

    async fn get_role(&self, name: &str) -> Result<Option<RoleEntity>, DaoError> {
        let role = sqlx::query!(
            "SELECT name, update_timestamp, update_process FROM role WHERE name = ?",
            name
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(role.map(|row| RoleEntity {
            name: Arc::from(row.name.as_str()),
            update_timestamp: Self::parse_timestamp(row.update_timestamp),
            update_process: Arc::from(row.update_process.as_str()),
        }))
    }

    async fn create_role(&self, role: &RoleEntity, process: &str) -> Result<(), DaoError> {
        let timestamp = Self::format_timestamp(role.update_timestamp);
        let name = role.name.as_ref();

        sqlx::query!(
            "INSERT INTO role (name, update_timestamp, update_process) VALUES (?, ?, ?)",
            name,
            timestamp,
            process
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn delete_role(&self, role_name: &str) -> Result<(), DaoError> {
        sqlx::query!("DELETE FROM role WHERE name = ?", role_name)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn all_privileges(&self) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        let privileges = sqlx::query!(
            "SELECT name, update_timestamp, update_process FROM privilege ORDER BY name"
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let privilege_entities: Vec<PrivilegeEntity> = privileges
            .into_iter()
            .map(|row| PrivilegeEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(privilege_entities))
    }

    async fn get_privilege(&self, name: &str) -> Result<Option<PrivilegeEntity>, DaoError> {
        let privilege = sqlx::query!(
            "SELECT name, update_timestamp, update_process FROM privilege WHERE name = ?",
            name
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(privilege.map(|row| PrivilegeEntity {
            name: Arc::from(row.name.as_str()),
            update_timestamp: Self::parse_timestamp(row.update_timestamp),
            update_process: Arc::from(row.update_process.as_str()),
        }))
    }

    async fn create_privilege(
        &self,
        privilege: &PrivilegeEntity,
        process: &str,
    ) -> Result<(), DaoError> {
        let timestamp = Self::format_timestamp(privilege.update_timestamp);
        let name = privilege.name.as_ref();

        sqlx::query!(
            "INSERT INTO privilege (name, update_timestamp, update_process) VALUES (?, ?, ?)",
            name,
            timestamp,
            process
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn delete_privilege(&self, privilege_name: &str) -> Result<(), DaoError> {
        sqlx::query!("DELETE FROM privilege WHERE name = ?", privilege_name)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn add_user_role(
        &self,
        username: &str,
        role: &str,
        process: &str,
    ) -> Result<(), DaoError> {
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_default();

        sqlx::query!(
            "INSERT INTO user_role (user_name, role_name, update_timestamp, update_process) VALUES (?, ?, ?, ?)",
            username,
            role,
            timestamp,
            process
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn remove_user_role(&self, username: &str, role: &str) -> Result<(), DaoError> {
        sqlx::query!(
            "DELETE FROM user_role WHERE user_name = ? AND role_name = ?",
            username,
            role
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn get_user_roles(&self, username: &str) -> Result<Arc<[RoleEntity]>, DaoError> {
        let roles = sqlx::query!(
            r#"
            SELECT r.name, r.update_timestamp, r.update_process
            FROM role r
            INNER JOIN user_role ur ON r.name = ur.role_name
            WHERE ur.user_name = ?
            ORDER BY r.name
            "#,
            username
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let role_entities: Vec<RoleEntity> = roles
            .into_iter()
            .map(|row| RoleEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(role_entities))
    }

    async fn add_role_privilege(
        &self,
        role_name: &str,
        privilege_name: &str,
        process: &str,
    ) -> Result<(), DaoError> {
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_default();

        sqlx::query!(
            "INSERT INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES (?, ?, ?, ?)",
            role_name,
            privilege_name,
            timestamp,
            process
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn remove_role_privilege(
        &self,
        role_name: &str,
        privilege_name: &str,
    ) -> Result<(), DaoError> {
        sqlx::query!(
            "DELETE FROM role_privilege WHERE role_name = ? AND privilege_name = ?",
            role_name,
            privilege_name
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn get_role_privileges(
        &self,
        role_name: &str,
    ) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        let privileges = sqlx::query!(
            r#"
            SELECT p.name, p.update_timestamp, p.update_process
            FROM privilege p
            INNER JOIN role_privilege rp ON p.name = rp.privilege_name
            WHERE rp.role_name = ?
            ORDER BY p.name
            "#,
            role_name
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let privilege_entities: Vec<PrivilegeEntity> = privileges
            .into_iter()
            .map(|row| PrivilegeEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(privilege_entities))
    }

    async fn get_user_privileges(
        &self,
        username: &str,
    ) -> Result<Arc<[PrivilegeEntity]>, DaoError> {
        let privileges = sqlx::query!(
            r#"
            SELECT DISTINCT p.name, p.update_timestamp, p.update_process
            FROM privilege p
            INNER JOIN role_privilege rp ON p.name = rp.privilege_name
            INNER JOIN user_role ur ON rp.role_name = ur.role_name
            WHERE ur.user_name = ?
            ORDER BY p.name
            "#,
            username
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        let privilege_entities: Vec<PrivilegeEntity> = privileges
            .into_iter()
            .map(|row| PrivilegeEntity {
                name: Arc::from(row.name.as_str()),
                update_timestamp: Self::parse_timestamp(row.update_timestamp),
                update_process: Arc::from(row.update_process.as_str()),
            })
            .collect();

        Ok(Arc::from(privilege_entities))
    }

    async fn create_session(&self, session: &SessionEntity) -> Result<(), DaoError> {
        let id = session.id.as_ref();
        let user_id = session.user_id.as_ref();
        let claims = session.claims.as_ref().map(|c| c.as_ref());

        sqlx::query!(
            "INSERT INTO session (id, user_id, expires, created, claims) VALUES (?, ?, ?, ?, ?)",
            id,
            user_id,
            session.expires,
            session.created,
            claims
        )
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionEntity>, DaoError> {
        let session = sqlx::query!(
            "SELECT id, user_id, expires, created, claims FROM session WHERE id = ?",
            session_id
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(session.map(|row| SessionEntity {
            id: Arc::from(row.id.as_deref().unwrap_or_default()),
            user_id: Arc::from(row.user_id.as_str()),
            expires: row.expires,
            created: row.created,
            claims: row.claims.map(|c| Arc::from(c.as_str())),
        }))
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), DaoError> {
        sqlx::query!("DELETE FROM session WHERE id = ?", session_id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }

    async fn cleanup_expired_sessions(&self, before_timestamp: i64) -> Result<(), DaoError> {
        sqlx::query!("DELETE FROM session WHERE expires < ?", before_timestamp)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DaoError::DatabaseError(e.to_string().into()))?;

        Ok(())
    }
}
