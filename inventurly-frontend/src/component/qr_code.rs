use dioxus::prelude::*;
use fast_qr::convert::{svg::SvgBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;

#[component]
pub fn QRCode(data: String, size: Option<u32>) -> Element {
    let size = size.unwrap_or(256);

    let svg_content = use_memo(move || {
        let qrcode = QRBuilder::new(data.clone()).build().ok()?;

        let svg = SvgBuilder::default()
            .shape(Shape::Square)
            .to_str(&qrcode);

        Some(svg)
    });

    match svg_content() {
        Some(svg) => {
            rsx! {
                div {
                    class: "qr-code-container",
                    style: "width: {size}px; height: {size}px;",
                    dangerous_inner_html: "{svg}"
                }
            }
        }
        None => {
            rsx! {
                div { class: "text-red-500", "Failed to generate QR code" }
            }
        }
    }
}
