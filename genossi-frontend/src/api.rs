use std::rc::Rc;

use rest_types::{MemberActionTO, MemberDocumentTO, MemberTO, MigrationStatusTO, UserTO, ValidationResultTO};
use tracing::info;
use uuid::Uuid;

use crate::state::{AuthInfo, Config};

// Config API
pub async fn fetch_config() -> Result<Config, reqwest::Error> {
    info!("Fetching config");
    let window = web_sys::window().unwrap();
    let origin = window.location().origin().unwrap();
    let url = format!("{}/assets/config.json", origin);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let config: Config = response.json().await?;
    info!("Config fetched: {:?}", config);
    Ok(config)
}

// Authentication API
pub async fn fetch_auth_info(backend_url: Rc<str>) -> Result<Option<AuthInfo>, reqwest::Error> {
    info!("Fetching auth info");
    let response = reqwest::get(format!("{}/api/auth/info", backend_url)).await?;
    if response.status() != 200 {
        return Ok(None);
    }
    let user: UserTO = response.json().await?;
    let auth_info = AuthInfo {
        user: user.username.into(),
        roles: user.roles.into_iter().map(|r| r.into()).collect(),
        privileges: user.privileges.into_iter().map(|p| p.into()).collect(),
        authenticated: true,
        claims: user.claims.into(),
    };
    info!("Auth info fetched");
    Ok(Some(auth_info))
}

// Member API
pub async fn get_members(config: &Config) -> Result<Vec<MemberTO>, reqwest::Error> {
    info!("Fetching members");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_member(config: &Config, id: Uuid) -> Result<MemberTO, reqwest::Error> {
    info!("Fetching member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_member(
    config: &Config,
    member: MemberTO,
) -> Result<MemberTO, reqwest::Error> {
    info!("Creating member");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new().post(url).json(&member).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn update_member(
    config: &Config,
    member: MemberTO,
) -> Result<MemberTO, reqwest::Error> {
    info!("Updating member {:?}", member.id);
    let id = member.id.unwrap();
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().put(url).json(&member).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_member(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    reqwest::Client::new().delete(url).send().await?.error_for_status_ref()?;
    Ok(())
}

// Member Action API
pub async fn get_member_actions(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberActionTO>, reqwest::Error> {
    info!("Fetching actions for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn create_member_action(
    config: &Config,
    member_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, reqwest::Error> {
    info!("Creating action for member {member_id}");
    let url = format!("{}/api/members/{member_id}/actions", config.backend);
    let response = reqwest::Client::new().post(url).json(&action).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn update_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
    action: MemberActionTO,
) -> Result<MemberActionTO, reqwest::Error> {
    info!("Updating action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    let response = reqwest::Client::new().put(url).json(&action).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_member_action(
    config: &Config,
    member_id: Uuid,
    action_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Deleting action {action_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/{action_id}",
        config.backend
    );
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

pub async fn get_migration_status(
    config: &Config,
    member_id: Uuid,
) -> Result<MigrationStatusTO, reqwest::Error> {
    info!("Fetching migration status for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/migration-status",
        config.backend
    );
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn confirm_migration(
    config: &Config,
    member_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Confirming migration for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/actions/confirm-migration",
        config.backend
    );
    let client = reqwest::Client::new();
    client.post(url).send().await?.error_for_status_ref()?;
    Ok(())
}

// Member Document API
pub async fn get_member_documents(
    config: &Config,
    member_id: Uuid,
) -> Result<Vec<MemberDocumentTO>, reqwest::Error> {
    info!("Fetching documents for member {member_id}");
    let url = format!("{}/api/members/{member_id}/documents", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn upload_member_document(
    config: &Config,
    member_id: Uuid,
    document_type: &str,
    description: Option<&str>,
    file: web_sys::File,
) -> Result<MemberDocumentTO, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!("{}/api/members/{member_id}/documents", config.backend);

    let form_data =
        web_sys::FormData::new().map_err(|e| format!("Failed to create FormData: {:?}", e))?;
    form_data
        .append_with_str("document_type", document_type)
        .map_err(|e| format!("Failed to append document_type: {:?}", e))?;
    if let Some(desc) = description {
        form_data
            .append_with_str("description", desc)
            .map_err(|e| format!("Failed to append description: {:?}", e))?;
    }
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|e| format!("Failed to append file: {:?}", e))?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object".to_string())?;

    if !resp.ok() {
        let status = resp.status();
        let text = JsFuture::from(resp.text().unwrap())
            .await
            .map_err(|e| format!("Failed to read error body: {:?}", e))?
            .as_string()
            .unwrap_or_default();
        return Err(format!("Upload failed ({}): {}", status, text));
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| format!("Failed to parse response: {:?}", e))?;

    let doc: MemberDocumentTO = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Failed to deserialize: {:?}", e))?;

    Ok(doc)
}

pub async fn delete_member_document(
    config: &Config,
    member_id: Uuid,
    document_id: Uuid,
) -> Result<(), reqwest::Error> {
    info!("Deleting document {document_id} for member {member_id}");
    let url = format!(
        "{}/api/members/{member_id}/documents/{document_id}",
        config.backend
    );
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

pub fn document_download_url(config: &Config, member_id: Uuid, document_id: Uuid) -> String {
    format!(
        "{}/api/members/{member_id}/documents/{document_id}",
        config.backend
    )
}

// Template API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum FileTreeEntry {
    #[serde(rename = "file")]
    File { name: String, path: String },
    #[serde(rename = "directory")]
    Directory {
        name: String,
        path: String,
        children: Vec<FileTreeEntry>,
    },
}

pub async fn get_templates(config: &Config) -> Result<Vec<FileTreeEntry>, reqwest::Error> {
    info!("Fetching templates");
    let url = format!("{}/api/templates", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn get_template_content(config: &Config, path: &str) -> Result<String, String> {
    info!("Fetching template content: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.text().await.map_err(|e| e.to_string())
}

pub async fn save_template(config: &Config, path: &str, content: &str) -> Result<(), String> {
    info!("Saving template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .put(url)
        .header("Content-Type", "text/plain")
        .body(content.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub async fn delete_template(config: &Config, path: &str) -> Result<(), String> {
    info!("Deleting template: {path}");
    let url = format!("{}/api/templates/{}", config.backend, path);
    let response = reqwest::Client::new()
        .delete(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

pub fn template_render_url(config: &Config, path: &str, member_id: Uuid) -> String {
    format!(
        "{}/api/templates/render/{}/{}",
        config.backend, path, member_id
    )
}

pub async fn render_template_pdf(config: &Config, path: &str, member_id: Uuid) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_url(config, path, member_id);
    info!("Rendering template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    let window = web_sys::window().ok_or("No window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object".to_string())?;

    if !resp.ok() {
        let status = resp.status();
        let text = JsFuture::from(resp.text().unwrap())
            .await
            .map_err(|e| format!("Failed to read error body: {:?}", e))?
            .as_string()
            .unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| format!("Failed to read blob: {:?}", e))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| "Not a Blob".to_string())?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create blob URL: {:?}", e))?;

    Ok(blob_url)
}

// Config API (backend config store)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigEntryTO {
    pub key: String,
    pub value: String,
    pub value_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetConfigRequest {
    pub value: String,
    pub value_type: String,
}

pub async fn get_config_entries(config: &Config) -> Result<Vec<ConfigEntryTO>, reqwest::Error> {
    info!("Fetching config entries");
    let url = format!("{}/api/config", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn set_config_entry(
    config: &Config,
    key: &str,
    value: &str,
    value_type: &str,
) -> Result<ConfigEntryTO, reqwest::Error> {
    info!("Setting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    let body = SetConfigRequest {
        value: value.to_string(),
        value_type: value_type.to_string(),
    };
    let response = reqwest::Client::new().put(url).json(&body).send().await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}

pub async fn delete_config_entry(config: &Config, key: &str) -> Result<(), reqwest::Error> {
    info!("Deleting config entry: {key}");
    let url = format!("{}/api/config/{}", config.backend, key);
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status_ref()?;
    Ok(())
}

// Mail API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailJobTO {
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailRecipientTO {
    pub id: String,
    pub to_address: String,
    pub member_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub sent_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailJobDetailTO {
    #[serde(flatten)]
    pub job: MailJobTO,
    pub recipients: Vec<MailRecipientTO>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendMailRequest {
    pub to_address: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BulkRecipient {
    pub address: String,
    pub member_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendBulkMailRequest {
    pub to_addresses: Vec<BulkRecipient>,
    pub subject: String,
    pub body: String,
}

pub async fn send_bulk_mail(
    config: &Config,
    recipients: &[BulkRecipient],
    subject: &str,
    body: &str,
) -> Result<MailJobTO, String> {
    info!("Sending bulk mail to {} recipients", recipients.len());
    let url = format!("{}/api/mail/send-bulk", config.backend);
    let req = SendBulkMailRequest {
        to_addresses: recipients.to_vec(),
        subject: subject.to_string(),
        body: body.to_string(),
    };
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_mail_jobs(config: &Config) -> Result<Vec<MailJobTO>, String> {
    info!("Fetching mail jobs");
    let url = format!("{}/api/mail/jobs", config.backend);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn get_mail_job_detail(config: &Config, id: &str) -> Result<MailJobDetailTO, String> {
    info!("Fetching mail job detail: {id}");
    let url = format!("{}/api/mail/jobs/{}", config.backend, id);
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn retry_mail_job(config: &Config, id: &str) -> Result<MailJobTO, String> {
    info!("Retrying mail job: {id}");
    let url = format!("{}/api/mail/jobs/{}/retry", config.backend, id);
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    response.json().await.map_err(|e| e.to_string())
}

pub async fn send_test_mail(config: &Config, to_address: &str) -> Result<(), String> {
    info!("Sending test mail to: {to_address}");
    let url = format!("{}/api/mail/test", config.backend);
    let req = serde_json::json!({ "to_address": to_address });
    let response = reqwest::Client::new()
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, text));
    }
    Ok(())
}

// Validation API
pub async fn get_validation(config: &Config) -> Result<ValidationResultTO, reqwest::Error> {
    info!("Fetching validation results");
    let url = format!("{}/api/validation", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    Ok(response.json().await?)
}
