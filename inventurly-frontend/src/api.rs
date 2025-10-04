use std::rc::Rc;

use rest_types::{
    ProductTO, RackTO, UserTO,
    DuplicateDetectionResultTO, CheckDuplicateRequestTO,
};
use tracing::info;
use uuid::Uuid;

use crate::state::{AuthInfo, Config};

// Authentication API
pub async fn fetch_auth_info(backend_url: Rc<str>) -> Result<Option<AuthInfo>, reqwest::Error> {
    info!("Fetching auth info");
    let response = reqwest::get(format!("{}/auth/info", backend_url)).await?;
    if response.status() != 200 {
        return Ok(None);
    }
    let user: UserTO = response.json().await?;
    let auth_info = AuthInfo {
        user: user.username.into(),
        privileges: user.roles.into_iter().map(|r| r.into()).collect(),
        authenticated: true,
    };
    info!("Auth info fetched");
    Ok(Some(auth_info))
}

pub async fn login(
    backend_url: Rc<str>,
    username: &str,
    password: &str,
) -> Result<bool, reqwest::Error> {
    info!("Logging in user: {}", username);
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/login", backend_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password,
        }))
        .send()
        .await?;
    
    Ok(response.status() == 200)
}

pub async fn logout(backend_url: Rc<str>) -> Result<(), reqwest::Error> {
    info!("Logging out");
    let client = reqwest::Client::new();
    client
        .post(format!("{}/auth/logout", backend_url))
        .send()
        .await?;
    Ok(())
}

// Config API
pub async fn load_config() -> Result<Config, reqwest::Error> {
    info!("Loading config.json");
    let protocol = web_sys::window()
        .expect("no window")
        .location()
        .protocol()
        .expect("no protocol");
    let host = web_sys::window()
        .expect("no window")
        .location()
        .host()
        .expect("no host");
    let url = format!("{protocol}//{host}/assets/config.json");
    info!("Config URL: {url}");
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res: Config = response.json().await?;
    info!("Config loaded");
    Ok(res)
}


// Product API
pub async fn get_products(config: &Config) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Fetching products");
    let url = format!("{}/products", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Products fetched");
    Ok(res)
}

pub async fn get_product(config: &Config, id: Uuid) -> Result<ProductTO, reqwest::Error> {
    info!("Fetching product {id}");
    let url = format!("{}/products/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product fetched");
    Ok(res)
}

pub async fn create_product(config: &Config, product: ProductTO) -> Result<ProductTO, reqwest::Error> {
    info!("Creating product");
    let url = format!("{}/products", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&product).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product created");
    Ok(res)
}

pub async fn update_product(config: &Config, product: ProductTO) -> Result<ProductTO, reqwest::Error> {
    info!("Updating product {:?}", product.id);
    let url = format!("{}/products/{}", config.backend, product.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&product).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Product updated");
    Ok(res)
}

pub async fn delete_product(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting product {id}");
    let url = format!("{}/products/{}", config.backend, id);
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Product deleted");
    Ok(())
}

// Search products
pub async fn search_products(
    config: &Config,
    query: &str,
) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Searching products: {query}");
    let url = format!("{}/products/search?q={}", config.backend, query);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Search completed");
    Ok(res)
}

// Duplicate detection
pub async fn check_duplicates(
    config: &Config,
    request: CheckDuplicateRequestTO,
) -> Result<DuplicateDetectionResultTO, reqwest::Error> {
    info!("Checking for duplicate products");
    let url = format!("{}/duplicate-detection/check", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&request).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Duplicate check completed");
    Ok(res)
}


// CSV Import
pub async fn import_csv(
    config: &Config,
    csv_data: String,
) -> Result<Vec<ProductTO>, reqwest::Error> {
    info!("Importing CSV data");
    let url = format!("{}/csv-import/products", config.backend);
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "text/csv")
        .body(csv_data)
        .send()
        .await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("CSV import completed");
    Ok(res)
}

// Rack API
pub async fn get_racks(config: &Config) -> Result<Vec<RackTO>, reqwest::Error> {
    info!("Fetching racks");
    let url = format!("{}/racks", config.backend);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Racks fetched");
    Ok(res)
}

pub async fn get_rack(config: &Config, id: Uuid) -> Result<RackTO, reqwest::Error> {
    info!("Fetching rack {id}");
    let url = format!("{}/racks/{}", config.backend, id);
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack fetched");
    Ok(res)
}

pub async fn create_rack(config: &Config, rack: RackTO) -> Result<RackTO, reqwest::Error> {
    info!("Creating rack");
    let url = format!("{}/racks", config.backend);
    let client = reqwest::Client::new();
    let response = client.post(url).json(&rack).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack created");
    Ok(res)
}

pub async fn update_rack(config: &Config, rack: RackTO) -> Result<RackTO, reqwest::Error> {
    info!("Updating rack {:?}", rack.id);
    let url = format!("{}/racks/{}", config.backend, rack.id.unwrap());
    let client = reqwest::Client::new();
    let response = client.put(url).json(&rack).send().await?;
    response.error_for_status_ref()?;
    let res = response.json().await?;
    info!("Rack updated");
    Ok(res)
}

pub async fn delete_rack(config: &Config, id: Uuid) -> Result<(), reqwest::Error> {
    info!("Deleting rack {id}");
    let url = format!("{}/racks/{}", config.backend, id);
    let client = reqwest::Client::new();
    let response = client.delete(url).send().await?;
    response.error_for_status_ref()?;
    info!("Rack deleted");
    Ok(())
}