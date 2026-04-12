use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum WebDavError {
    RequestFailed(Arc<str>),
    AuthenticationFailed,
    ServerError(u16, Arc<str>),
}

impl std::fmt::Display for WebDavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebDavError::RequestFailed(msg) => write!(f, "WebDAV request failed: {}", msg),
            WebDavError::AuthenticationFailed => write!(f, "WebDAV authentication failed"),
            WebDavError::ServerError(status, msg) => {
                write!(f, "WebDAV server error ({}): {}", status, msg)
            }
        }
    }
}

pub struct WebDavClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
}

impl WebDavClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            client: reqwest::Client::new(),
            base_url,
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub async fn mkcol(&self, path: &str) -> Result<(), WebDavError> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| WebDavError::RequestFailed(Arc::from(e.to_string())))?;

        let status = response.status().as_u16();
        match status {
            201 => Ok(()),
            // Directory already exists
            405 => Ok(()),
            401 | 403 => Err(WebDavError::AuthenticationFailed),
            _ => {
                let body = response.text().await.unwrap_or_default();
                Err(WebDavError::ServerError(status, Arc::from(body.as_str())))
            }
        }
    }

    pub async fn put(&self, path: &str, data: Vec<u8>) -> Result<(), WebDavError> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .timeout(std::time::Duration::from_secs(30))
            .body(data)
            .send()
            .await
            .map_err(|e| WebDavError::RequestFailed(Arc::from(e.to_string())))?;

        let status = response.status().as_u16();
        match status {
            200 | 201 | 204 => Ok(()),
            401 | 403 => Err(WebDavError::AuthenticationFailed),
            _ => {
                let body = response.text().await.unwrap_or_default();
                Err(WebDavError::ServerError(status, Arc::from(body.as_str())))
            }
        }
    }

    /// Tests the connection by attempting MKCOL on the base directory.
    pub async fn test_connection(&self, directory: &str) -> Result<(), WebDavError> {
        self.mkcol_recursive(directory).await
    }

    /// Creates nested directories, handling each level.
    pub async fn mkcol_recursive(&self, path: &str) -> Result<(), WebDavError> {
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();
        let mut current = String::new();
        for part in parts {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(part);
            self.mkcol(&current).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mkcol_success() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .and(wiremock::matchers::path("/test-dir"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "pass");
        let result = client.mkcol("test-dir").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mkcol_already_exists() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .and(wiremock::matchers::path("/existing-dir"))
            .respond_with(wiremock::ResponseTemplate::new(405))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "pass");
        let result = client.mkcol("existing-dir").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mkcol_auth_failure() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .respond_with(wiremock::ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "wrong");
        let result = client.mkcol("dir").await;
        assert!(matches!(result, Err(WebDavError::AuthenticationFailed)));
    }

    #[tokio::test]
    async fn test_put_success() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/file.csv"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "pass");
        let result = client.put("file.csv", b"test data".to_vec()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .and(wiremock::matchers::path("/file.csv"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "pass");
        let result = client.put("file.csv", b"updated data".to_vec()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_put_auth_failure() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PUT"))
            .respond_with(wiremock::ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "wrong");
        let result = client.put("file.csv", b"data".to_vec()).await;
        assert!(matches!(result, Err(WebDavError::AuthenticationFailed)));
    }

    #[tokio::test]
    async fn test_mkcol_recursive() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("MKCOL"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .expect(3)
            .mount(&server)
            .await;

        let client = WebDavClient::new(&server.uri(), "user", "pass");
        let result = client.mkcol_recursive("a/b/c").await;
        assert!(result.is_ok());
    }
}
