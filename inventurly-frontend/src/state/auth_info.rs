use std::collections::HashMap;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuthInfo {
    #[serde(rename = "username")]
    pub user: Rc<str>,
    pub roles: Rc<[Rc<str>]>,
    pub privileges: Rc<[Rc<str>]>,
    #[serde(default)]
    pub authenticated: bool,
    #[serde(default)]
    pub claims: Rc<HashMap<String, String>>,
}

impl Default for AuthInfo {
    fn default() -> Self {
        Self {
            user: "".into(),
            roles: Rc::new([]),
            privileges: Rc::new([]),
            authenticated: false,
            claims: Rc::new(HashMap::new()),
        }
    }
}

impl AuthInfo {
    #[allow(dead_code)]
    pub fn has_privilege(&self, privilege: &str) -> bool {
        self.privileges.iter().any(|p| p.as_ref() == privilege)
    }

    pub fn get_inventur_id(&self) -> Option<String> {
        self.claims.get("inventur_id").cloned()
    }

    /// Check if user logged in with inventur token (has claims)
    pub fn is_token_based(&self) -> bool {
        self.claims.contains_key("inventur_id")
    }

    /// Check if user can edit inventur data (measurements, custom entries)
    /// - active: everyone can edit
    /// - post_processing: only role-based users (not token-based)
    pub fn can_edit_inventur(&self, status: &str) -> bool {
        status == "active" || (status == "post_processing" && !self.is_token_based())
    }
}
