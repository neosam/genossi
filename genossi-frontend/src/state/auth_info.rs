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

}
