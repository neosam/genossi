use dioxus::prelude::*;

pub use crate::page::ContainerDetails;
pub use crate::page::Containers;
pub use crate::page::Home;
pub use crate::page::ProductDetails;
pub use crate::page::Products;
pub use crate::page::RackDetails;
pub use crate::page::Racks;

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
    #[route("/container-management")]
    Containers {},
    #[route("/container-management/:id")]
    ContainerDetails { id: String },
}
