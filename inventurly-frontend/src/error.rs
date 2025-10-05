use reqwest::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum InventurlyError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Time ComponentRange error: {0}")]
    TimeComponentRange(#[from] time::error::ComponentRange),
}

#[allow(dead_code)]
pub fn error_handler(e: InventurlyError) {
    match e {
        InventurlyError::Reqwest(e) => {
            eprintln!("Error: {}", e);
            if let Some(StatusCode::UNAUTHORIZED) = e.status() {
                let _ = web_sys::window().expect("no window").location().reload();
            }
        }
        InventurlyError::TimeComponentRange(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

#[allow(dead_code)]
pub fn result_handler<T>(res: Result<T, InventurlyError>) -> Option<T> {
    match res {
        Ok(t) => Some(t),
        Err(e) => {
            error_handler(e);
            None
        }
    }
}
