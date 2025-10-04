use dioxus::prelude::*;

pub use crate::page::Home;
pub use crate::page::Products;
pub use crate::page::ProductDetails;
pub use crate::page::Racks;
pub use crate::page::RackDetails;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/inventory")]
    Products {},
    #[route("/inventory/:id")]
    ProductDetails { id: String },
    #[route("/rack-management")]
    Racks {},
    #[route("/rack-management/:id")]
    RackDetails { id: String },
}