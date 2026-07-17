//! QrScanner Component (Phase 4 Plan 05) — Camera-Lifecycle + BarcodeDetector or ZXing-JS polyfill.
//!
//! Hard constraints (RESEARCH §QR-Scanner Integration Plan):
//! - Mount AFTER user click — iOS-Safari user-gesture requirement (D-03).
//! - Stop MediaStream tracks on unmount — Pitfall 2 (no permission-light-leak, T-04-19).
//! - `<video playsinline muted autoplay>` — Pitfall 3 (iOS-Safari fullscreen quirk, T-04-22).
//! - ZXing-JS path via `manganis::Asset` (Plan 02 `ZXING_POLYFILL`) — Pitfall 7
//!   (hash-fingerprinted asset path, T-04-23).
//!
//! W-02 acceptance: `decide_camera_path` is a pure helper carved out for cargo-tests
//! (no web-sys dependency), so the branch-logic is verifiable without real camera hardware.
//! UI-Verifikation des Camera-Pfads selbst erfolgt in Phase-5-Generalprobe.

use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    HtmlVideoElement, MediaDevices, MediaStream, MediaStreamConstraints, MediaStreamTrack,
    MediaTrackConstraints,
};

use crate::i18n::{use_i18n, Key};
use crate::js::{self, has_barcode_detector, ZXING_POLYFILL};

/// Welcher Decoder-Pfad genutzt wird, je nachdem ob der Browser den nativen
/// `BarcodeDetector` mitbringt oder ob das ZXing-JS-Polyfill geladen werden muss.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CameraPath {
    /// BarcodeDetector ist im `window`-Objekt vorhanden — kein Asset-Download nötig.
    Native,
    /// BarcodeDetector fehlt, aber das ZXing-JS-Polyfill ist verfügbar (Plan 02 hat den
    /// `manganis::Asset`-Pfad bereitgestellt).
    Polyfill,
    /// Weder Native noch Polyfill verfügbar (z.B. wenn der Polyfill-Asset nicht geladen
    /// werden kann oder das Asset-System komplett offline ist).
    Unsupported,
}

/// Entscheidet welcher Camera-Pfad genutzt wird. Pure logic — Cargo-testbar
/// (kein web-sys-Aufruf, kein Browser-State).
///
/// Inputs: feature-Detection-Resultate (Browser-API-Presence).
/// Native hat immer Vorrang vor Polyfill — kein zusätzlicher Asset-Download wenn nicht nötig.
#[allow(dead_code)]
pub fn decide_camera_path(has_native: bool, has_polyfill: bool) -> CameraPath {
    match (has_native, has_polyfill) {
        (true, _) => CameraPath::Native,
        (false, true) => CameraPath::Polyfill,
        (false, false) => CameraPath::Unsupported,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ScannerState {
    RequestingPermission,
    Streaming,
    Error,
}

#[component]
pub fn QrScanner(
    on_scan: EventHandler<String>,
    on_error: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut state = use_signal(|| ScannerState::RequestingPermission);
    let mut error_msg = use_signal(|| Option::<String>::None);

    // Stream holder for use_drop cleanup (Pattern 2 — RESEARCH §use_drop pattern).
    let stream_holder: Rc<RefCell<Option<MediaStream>>> = use_hook(|| Rc::new(RefCell::new(None)));

    // Mount-Effect: Camera anfragen + Decoder starten.
    {
        let stream_holder = stream_holder.clone();
        let cam_denied_msg = i18n.t(Key::HelperLoginCameraDenied).to_string();
        let cam_unavail_msg = i18n.t(Key::HelperLoginCameraNotAvailable).to_string();
        use_effect(move || {
            let stream_holder = stream_holder.clone();
            let cam_denied_msg = cam_denied_msg.clone();
            let cam_unavail_msg = cam_unavail_msg.clone();
            spawn(async move {
                // Branch-Entscheidung über Pure-Helper (W-02). Polyfill ist immer verfügbar,
                // weil Plan 02 `ZXING_POLYFILL` als manganis-Asset gepinnt hat.
                let path = decide_camera_path(has_barcode_detector(), true);

                // 1. ZXing lazy-laden, falls Polyfill-Pfad gewählt wurde.
                if matches!(path, CameraPath::Polyfill) {
                    let zxing_url = ZXING_POLYFILL.to_string();
                    let script = format!(
                        r#"
                        (async () => {{
                            if (!window.__zxing_loaded) {{
                                await new Promise((resolve, reject) => {{
                                    const s = document.createElement('script');
                                    s.src = '{}';
                                    s.onload = resolve;
                                    s.onerror = reject;
                                    document.head.appendChild(s);
                                }});
                                window.__zxing_loaded = true;
                            }}
                            if (!window.__zxing_reader && window.ZXing) {{
                                window.__zxing_reader = new ZXing.BrowserMultiFormatReader();
                            }}
                        }})();
                        "#,
                        zxing_url
                    );
                    let _ = dioxus::document::eval(&script).await;
                }

                // 2. Camera-Stream anfordern.
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        error_msg.set(Some(cam_unavail_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_unavail_msg);
                        return;
                    }
                };
                let navigator = window.navigator();
                let media_devices: MediaDevices = match navigator.media_devices() {
                    Ok(md) => md,
                    Err(_) => {
                        error_msg.set(Some(cam_unavail_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_unavail_msg);
                        return;
                    }
                };
                // Rückkamera bevorzugen (Pitfall: facingMode ist Hint, nicht garantiert).
                let mut video_constraints = MediaTrackConstraints::new();
                video_constraints.facing_mode(&JsValue::from_str("environment"));
                let mut constraints = MediaStreamConstraints::new();
                constraints.video(video_constraints.as_ref());
                let promise = match media_devices.get_user_media_with_constraints(&constraints) {
                    Ok(p) => p,
                    Err(_) => {
                        error_msg.set(Some(cam_denied_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_denied_msg);
                        return;
                    }
                };
                let stream_value = match JsFuture::from(promise).await {
                    Ok(v) => v,
                    Err(_) => {
                        error_msg.set(Some(cam_denied_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_denied_msg);
                        return;
                    }
                };
                let stream: MediaStream = match stream_value.dyn_into() {
                    Ok(s) => s,
                    Err(_) => {
                        error_msg.set(Some(cam_unavail_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_unavail_msg);
                        return;
                    }
                };
                *stream_holder.borrow_mut() = Some(stream.clone());
                state.set(ScannerState::Streaming);

                // 3. Stream ins <video>-Element pipen + Decoder-Loop starten.
                let document = match window.document() {
                    Some(d) => d,
                    None => return,
                };
                let video_element = match document.get_element_by_id("qr-scanner-video") {
                    Some(el) => match el.dyn_into::<HtmlVideoElement>() {
                        Ok(v) => v,
                        Err(_) => return,
                    },
                    None => return,
                };
                video_element.set_src_object(Some(&stream));
                let _ = video_element.play();

                match path {
                    CameraPath::Native => {
                        // Native BarcodeDetector — Frame-Loop via TimeoutFuture.
                        spawn(async move {
                            use gloo_timers::future::TimeoutFuture;
                            let detector = js::BarcodeDetector::new(&JsValue::NULL);
                            loop {
                                TimeoutFuture::new(250).await;
                                let win = match web_sys::window() {
                                    Some(w) => w,
                                    None => return,
                                };
                                let doc = match win.document() {
                                    Some(d) => d,
                                    None => return,
                                };
                                // Element könnte unmounted sein — dann Loop beenden.
                                let el = match doc.get_element_by_id("qr-scanner-video") {
                                    Some(e) => e,
                                    None => return,
                                };
                                let promise = detector.detect(el.as_ref());
                                if let Ok(result) = JsFuture::from(promise).await {
                                    if let Ok(arr) = result.dyn_into::<js_sys::Array>() {
                                        if arr.length() > 0 {
                                            let first = arr.get(0);
                                            if let Ok(raw) = js_sys::Reflect::get(
                                                &first,
                                                &JsValue::from_str("rawValue"),
                                            ) {
                                                if let Some(text) = raw.as_string() {
                                                    on_scan.call(text);
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                    CameraPath::Polyfill => {
                        // ZXing-JS-Polyfill: Reader bekommt die video-id, schreibt
                        // Resultate in `window.__zxing_last_result`. Wir pollen.
                        let _ = dioxus::document::eval(
                            r#"
                            (async () => {
                                while (!window.__zxing_reader) {
                                    await new Promise(r => setTimeout(r, 50));
                                }
                                window.__zxing_reader.decodeFromVideoDevice(
                                    undefined,
                                    'qr-scanner-video',
                                    (result, _err) => {
                                        if (result) {
                                            window.__zxing_last_result = result.getText();
                                        }
                                    }
                                );
                            })();
                            "#,
                        )
                        .await;
                        spawn(async move {
                            use gloo_timers::future::TimeoutFuture;
                            loop {
                                TimeoutFuture::new(200).await;
                                let mut eval = dioxus::document::eval(
                                    "return window.__zxing_last_result || null;",
                                );
                                if let Ok(v) = eval.recv::<serde_json::Value>().await {
                                    if let Some(text) = v.as_str() {
                                        if !text.is_empty() {
                                            on_scan.call(text.to_string());
                                            // Reset flag, damit der nächste Scan auslöst.
                                            let _ = dioxus::document::eval(
                                                "window.__zxing_last_result = null;",
                                            )
                                            .await;
                                            return;
                                        }
                                    }
                                }
                            }
                        });
                    }
                    CameraPath::Unsupported => {
                        error_msg.set(Some(cam_unavail_msg.clone()));
                        state.set(ScannerState::Error);
                        on_error.call(cam_unavail_msg);
                    }
                }
            });
        });
    }

    // use_drop: stop MediaStream tracks (Pattern 2 — Pitfall 2 / T-04-19).
    let stream_for_drop = stream_holder.clone();
    use_drop(move || {
        if let Some(stream) = stream_for_drop.borrow_mut().take() {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                if let Ok(track) = tracks.get(i).dyn_into::<MediaStreamTrack>() {
                    track.stop();
                }
            }
        }
        // ZXing-Reader auch zurücksetzen, falls genutzt.
        wasm_bindgen_futures::spawn_local(async {
            let _ = dioxus::document::eval(
                "if (window.__zxing_reader) { try { window.__zxing_reader.reset(); } catch (_) {} }",
            )
            .await;
        });
    });

    rsx! {
        div { class: "fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4",
            div { class: "relative bg-black aspect-square w-full max-w-md rounded-lg overflow-hidden",
                button {
                    class: "absolute top-2 right-2 text-white text-2xl z-10 px-3 py-1",
                    "aria-label": "Schließen",
                    onclick: move |_| on_cancel.call(()),
                    "\u{00D7}"
                }
                video {
                    id: "qr-scanner-video",
                    class: "w-full h-full object-cover",
                    playsinline: "true",
                    muted: "true",
                    autoplay: "true",
                }
                div { class: "absolute inset-8 border-2 border-white rounded-lg pointer-events-none" }
                match state() {
                    ScannerState::RequestingPermission => rsx! {
                        div { class: "absolute inset-0 flex items-center justify-center bg-black/60 text-white",
                            span { "{i18n.t(Key::HelperLoginCameraStarting)}" }
                        }
                    },
                    ScannerState::Streaming => rsx! {
                        div { class: "absolute bottom-3 left-0 right-0 text-center text-white text-sm",
                            span { "{i18n.t(Key::HelperLoginQrFrameHint)}" }
                        }
                    },
                    ScannerState::Error => rsx! {
                        div { class: "absolute inset-0 flex items-center justify-center bg-red-50 text-red-700 p-4",
                            if let Some(msg) = error_msg.read().as_ref() { span { "{msg}" } }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_when_barcode_detector_available() {
        // BarcodeDetector vorhanden → kein ZXing-Download (auch wenn Polyfill da wäre).
        assert_eq!(decide_camera_path(true, false), CameraPath::Native);
        assert_eq!(decide_camera_path(true, true), CameraPath::Native);
    }

    #[test]
    fn polyfill_when_only_polyfill_available() {
        // iOS Safari-Hauptpfad: kein BarcodeDetector, ZXing-Asset geladen.
        assert_eq!(decide_camera_path(false, true), CameraPath::Polyfill);
    }

    #[test]
    fn unsupported_when_neither_available() {
        // Worst-Case: kein Native + kein Polyfill → Component zeigt Error.
        assert_eq!(decide_camera_path(false, false), CameraPath::Unsupported);
    }

    #[test]
    fn native_takes_priority_over_polyfill() {
        // Beide verfügbar — Native gewinnt (kein 200 KB ZXing-Bundle laden, wenn nicht nötig).
        assert_eq!(decide_camera_path(true, true), CameraPath::Native);
    }
}
