use dioxus::prelude::*;

#[derive(Debug, Clone)]
pub struct ErrorStore {
    pub message: Option<String>,
}

impl ErrorStore {
    pub fn new() -> Self {
        Self { message: None }
    }

    pub fn set_error(&mut self, message: String) {
        self.message = Some(message);
    }

    pub fn clear_error(&mut self) {
        self.message = None;
    }
}

pub static ERROR_STORE: GlobalSignal<ErrorStore> = GlobalSignal::new(|| ErrorStore::new());