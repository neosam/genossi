use dioxus::prelude::*;

pub use crate::page::Home;
pub use crate::page::Products;
pub use crate::page::ProductDetails;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/inventory")]
    Products {},
    #[route("/inventory/:id")]
    ProductDetails { id: String },
}