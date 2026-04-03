use dioxus::prelude::*;

pub use crate::page::Home;
pub use crate::page::MemberDetails;
pub use crate::page::Members;
pub use crate::page::Permissions;
pub use crate::page::Templates;
pub use crate::page::Validation;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/members")]
    Members {},
    #[route("/members/:id")]
    MemberDetails { id: String },
    #[route("/permissions")]
    Permissions {},
    #[route("/validation")]
    Validation {},
    #[route("/templates")]
    Templates {},
}
