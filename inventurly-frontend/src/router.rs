use dioxus::prelude::*;

pub use crate::page::ContainerDetails;
pub use crate::page::Containers;
pub use crate::page::DuplicateDetection;
pub use crate::page::Home;
pub use crate::page::InventurDetails;
pub use crate::page::InventurMeasurements;
pub use crate::page::InventurRackMeasure;
pub use crate::page::InventurRackSelection;
pub use crate::page::Inventurs;
pub use crate::page::Permissions;
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
    #[route("/inventory/duplicates")]
    DuplicateDetection {},
    #[route("/inventurs")]
    Inventurs {},
    #[route("/inventurs/:id")]
    InventurDetails { id: String },
    #[route("/inventurs/:id/measurements")]
    InventurMeasurements { id: String },
    #[route("/inventurs/:id/select-rack")]
    InventurRackSelection { id: String },
    #[route("/inventurs/:inventur_id/rack/:rack_id/measure")]
    InventurRackMeasure { inventur_id: String, rack_id: String },
    #[route("/rack-management")]
    Racks {},
    #[route("/rack-management/:id")]
    RackDetails { id: String },
    #[route("/container-management")]
    Containers {},
    #[route("/container-management/:id")]
    ContainerDetails { id: String },
    #[route("/permissions")]
    Permissions {},
}
