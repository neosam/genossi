use std::rc::Rc;

use rest_types::{MemberTO, UserTO};
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
