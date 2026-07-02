use js_sys::{wasm_bindgen::JsValue, Date};
use wasm_bindgen::prelude::*;

// CodeMirror interop functions
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = window, js_name = createTypstEditor)]
    pub fn create_typst_editor(
        element_id: &str,
        content: &str,
        on_change: &Closure<dyn FnMut(String)>,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = window, js_name = setEditorContent)]
    pub fn set_editor_content(editor_id: &JsValue, content: &str);

    #[wasm_bindgen(js_namespace = window, js_name = getEditorContent)]
    pub fn get_editor_content(editor_id: &JsValue) -> String;

    #[wasm_bindgen(js_namespace = window, js_name = destroyEditor)]
    pub fn destroy_editor(editor_id: &JsValue);
}

#[allow(dead_code)]
pub fn get_current_year() -> u32 {
    current_datetime().to_iso_week_date().0 as u32
}

// Function to get the current week number based on ISO 8601
#[allow(dead_code)]
pub fn get_current_week() -> u8 {
    current_datetime().iso_week()
}

#[allow(dead_code)]
pub fn js_date_to_primitive_date_time(date: &Date) -> time::PrimitiveDateTime {
    time::PrimitiveDateTime::new(
        time::Date::from_calendar_date(
            date.get_full_year() as i32,
            time::Month::January.nth_next(date.get_month() as u8),
            date.get_date() as u8,
        )
        .unwrap(),
        time::Time::from_hms(
            date.get_hours() as u8,
            date.get_minutes() as u8,
            date.get_seconds() as u8,
        )
        .unwrap(),
    )
}

#[allow(dead_code)]
pub fn current_datetime() -> time::PrimitiveDateTime {
    let date = Date::new_0();
    js_date_to_primitive_date_time(&date)
}

#[allow(dead_code)]
pub fn date_time_str_to_primitive_date_time(date_time_str: &str) -> time::PrimitiveDateTime {
    let date = Date::new(&JsValue::from_str(date_time_str));
    js_date_to_primitive_date_time(&date)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = navigator)]
    type Clipboard;

    #[wasm_bindgen(js_namespace = navigator, js_name = clipboard, getter, catch)]
    fn get_clipboard() -> Result<Clipboard, JsValue>;

    #[wasm_bindgen(method, js_name = writeText, catch)]
    fn write_text(this: &Clipboard, text: &str) -> Result<js_sys::Promise, JsValue>;
}

#[allow(dead_code)]
pub async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    // Try modern clipboard API first
    match get_clipboard() {
        Ok(clipboard) => {
            match clipboard.write_text(text) {
                Ok(promise) => {
                    wasm_bindgen_futures::JsFuture::from(promise).await?;
                    Ok(())
                }
                Err(_) => {
                    // Fallback to execCommand
                    copy_with_exec_command(text)
                }
            }
        }
        Err(_) => {
            // Clipboard API not available, use fallback
            copy_with_exec_command(text)
        }
    }
}

#[allow(dead_code)]
fn copy_with_exec_command(text: &str) -> Result<(), JsValue> {
    use js_sys::Reflect;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or(JsValue::from_str("No window object"))?;
    let document = window
        .document()
        .ok_or(JsValue::from_str("No document object"))?;

    // Create a temporary textarea element
    let textarea = document
        .create_element("textarea")
        .map_err(|e| JsValue::from(e))?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .map_err(|_| JsValue::from_str("Failed to create textarea"))?;

    // Set the text and styling
    textarea.set_value(text);
    textarea.style().set_property("position", "fixed").ok();
    textarea.style().set_property("left", "-9999px").ok();
    textarea.style().set_property("top", "-9999px").ok();

    // Append to body
    document
        .body()
        .ok_or(JsValue::from_str("No body element"))?
        .append_child(&textarea)
        .map_err(|e| JsValue::from(e))?;

    // Select and copy
    textarea.select();

    // Call execCommand using Reflect
    let exec_command = Reflect::get(&document, &JsValue::from_str("execCommand"))
        .map_err(|_| JsValue::from_str("execCommand not available"))?;

    let exec_command_fn = exec_command
        .dyn_ref::<js_sys::Function>()
        .ok_or(JsValue::from_str("execCommand is not a function"))?;

    let success = exec_command_fn
        .call1(&document, &JsValue::from_str("copy"))
        .map_err(|_| JsValue::from_str("execCommand call failed"))?
        .as_bool()
        .unwrap_or(false);

    // Remove the temporary element
    textarea.remove();

    if success {
        Ok(())
    } else {
        Err(JsValue::from_str("execCommand('copy') failed"))
    }
}

// ─── Phase 24 Plan 02 ─── WYSIWYG editor execCommand facade ─────────────────
//
// The three `exec_command_*` helpers below are the contenteditable-execCommand
// facade used by the WYSIWYG Mail editor (see
// `component::mail_compose::wysiwyg_editor`). They mirror the
// `copy_with_exec_command` pattern above: acquire `document.execCommand` via
// `js_sys::Reflect` (no new JS bundle needed — see 24-RESEARCH.md Pattern 2
// and EDIT-02: "no new frontend deps").
//
// IMPORTANT — Pitfall 1 of 24-RESEARCH.md: the editor MUST call
// `exec_command_bool(&doc, "styleWithCSS", false)` exactly once at mount so
// bold/italic emit semantic <b>/<i> tags (ammonia-safe) instead of
// <span style="…"> (ammonia-stripped). Callers land in `wysiwyg_editor.rs`
// and `wysiwyg_toolbar.rs` within the same plan boundary — until then the
// helpers may show `dead_code` warnings, which is expected.

#[allow(dead_code)]
pub fn exec_command_bool(
    doc: &web_sys::Document,
    cmd: &str,
    arg: bool,
) -> Result<bool, wasm_bindgen::JsValue> {
    use js_sys::Reflect;
    use wasm_bindgen::JsCast;

    let exec_command = Reflect::get(doc, &JsValue::from_str("execCommand"))
        .map_err(|_| JsValue::from_str("execCommand not available"))?;
    let exec_command_fn = exec_command
        .dyn_ref::<js_sys::Function>()
        .ok_or(JsValue::from_str("execCommand is not a function"))?;

    let returned = exec_command_fn.call3(
        doc,
        &JsValue::from_str(cmd),
        &JsValue::from_bool(false),
        &JsValue::from_bool(arg),
    )?;
    Ok(returned.as_bool().unwrap_or(false))
}

#[allow(dead_code)]
pub fn exec_command_str(
    doc: &web_sys::Document,
    cmd: &str,
    arg: &str,
) -> Result<bool, wasm_bindgen::JsValue> {
    use js_sys::Reflect;
    use wasm_bindgen::JsCast;

    let exec_command = Reflect::get(doc, &JsValue::from_str("execCommand"))
        .map_err(|_| JsValue::from_str("execCommand not available"))?;
    let exec_command_fn = exec_command
        .dyn_ref::<js_sys::Function>()
        .ok_or(JsValue::from_str("execCommand is not a function"))?;

    let returned = exec_command_fn.call3(
        doc,
        &JsValue::from_str(cmd),
        &JsValue::from_bool(false),
        &JsValue::from_str(arg),
    )?;
    Ok(returned.as_bool().unwrap_or(false))
}

#[allow(dead_code)]
pub fn exec_command_simple(
    doc: &web_sys::Document,
    cmd: &str,
) -> Result<bool, wasm_bindgen::JsValue> {
    use js_sys::Reflect;
    use wasm_bindgen::JsCast;

    let exec_command = Reflect::get(doc, &JsValue::from_str("execCommand"))
        .map_err(|_| JsValue::from_str("execCommand not available"))?;
    let exec_command_fn = exec_command
        .dyn_ref::<js_sys::Function>()
        .ok_or(JsValue::from_str("execCommand is not a function"))?;

    let returned = exec_command_fn.call1(doc, &JsValue::from_str(cmd))?;
    Ok(returned.as_bool().unwrap_or(false))
}

// ─── Phase 4 Plan 02 ─── BarcodeDetector + ZXing-Polyfill bridge ─────────────

/// Browser-native Barcode/QR detection.
/// NOT available in iOS Safari (any version through 26.5) — see RESEARCH.md §Browser-Support-Realität.
/// `has_barcode_detector()` MUST be called first; if false, fall back to ZXing-JS polyfill.
#[wasm_bindgen]
extern "C" {
    pub type BarcodeDetector;

    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> BarcodeDetector;

    #[wasm_bindgen(method)]
    pub fn detect(this: &BarcodeDetector, source: &JsValue) -> js_sys::Promise;
}

/// Feature detection: `'BarcodeDetector' in window`.
/// Source: RESEARCH.md §"BarcodeDetector via wasm-bindgen extern".
#[allow(dead_code)]
pub fn has_barcode_detector() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    js_sys::Reflect::has(&window, &JsValue::from_str("BarcodeDetector")).unwrap_or(false)
}

/// Asset path for the lazy-loaded ZXing-JS polyfill (used by qr_scanner.rs in Plan 05
/// when has_barcode_detector() == false).
/// Manganis fingerprints the path at build time — see Pitfall 7 in RESEARCH.md.
#[allow(dead_code)]
pub const ZXING_POLYFILL: manganis::Asset = manganis::asset!("/assets/zxing.umd.min.js");
