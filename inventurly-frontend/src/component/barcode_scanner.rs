use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub barcode: String,
    pub format: String,
}

#[component]
pub fn BarcodeScanner(
    on_scan: EventHandler<ScanResult>,
    on_close: EventHandler<()>,
) -> Element {
    let scanner_container_id = use_signal(|| "barcode-scanner-container".to_string());
    let scanning = use_signal(|| false);
    let error_message = use_signal(|| None::<String>);
    let quagga_initialized = use_signal(|| false);
    let debug_messages = use_signal(|| Vec::<String>::new());

    // Initialize QuaggaJS scanner
    let start_scanner = use_coroutine({
        let scanner_container_id = scanner_container_id.clone();
        let mut scanning = scanning.clone();
        let mut error_message = error_message.clone();
        let mut quagga_initialized = quagga_initialized.clone();
        let mut debug_messages = debug_messages.clone();
        let on_scan = on_scan.clone();
        let on_close = on_close.clone();
        
        move |_: UnboundedReceiver<()>| async move {
            let mut add_debug = |message: String| {
                let mut messages = debug_messages.read().clone();
                messages.push(format!("[{}] {}", js_sys::Date::new_0().to_iso_string().as_string().unwrap(), message));
                if messages.len() > 10 {
                    messages.remove(0); // Keep only last 10 messages
                }
                debug_messages.set(messages);
                web_sys::console::log_1(&JsValue::from_str(&format!("BarcodeScanner: {}", message)));
            };
            
            add_debug("Starting barcode scanner initialization".to_string());
            
            // Check if we're in a secure context
            let window = web_sys::window().unwrap();
            let is_secure = window.is_secure_context();
            add_debug(format!("Is secure context: {}", is_secure));
            add_debug(format!("Current location: {}", window.location().href().unwrap_or("unknown".to_string())));
            
            // Detect mobile device
            let user_agent = window.navigator().user_agent().unwrap_or_default();
            let is_mobile = user_agent.contains("Mobile") || user_agent.contains("Android") || user_agent.contains("iPhone");
            add_debug(format!("Mobile device detected: {}", is_mobile));
            add_debug(format!("User agent: {}", user_agent));
            
            // Check if MediaDevices is available
            let navigator = window.navigator();
            if js_sys::Reflect::has(&navigator, &JsValue::from_str("mediaDevices")).unwrap_or(false) {
                add_debug("MediaDevices API is available".to_string());
                
                // Test getUserMedia availability
                let media_devices = js_sys::Reflect::get(&navigator, &JsValue::from_str("mediaDevices")).unwrap();
                if js_sys::Reflect::has(&media_devices, &JsValue::from_str("getUserMedia")).unwrap_or(false) {
                    add_debug("getUserMedia is available".to_string());
                } else {
                    add_debug("ERROR: getUserMedia is not available - WebRTC not supported".to_string());
                    error_message.set(Some("Camera access not supported on this device/browser".to_string()));
                    return;
                }
            } else {
                error_message.set(Some("MediaDevices API is not available in this browser".to_string()));
                add_debug("ERROR: MediaDevices API not available".to_string());
                return;
            }
            
            // Wait a bit for the DOM to be ready
            gloo_timers::future::TimeoutFuture::new(200).await;
            add_debug("DOM ready wait completed".to_string());
            
            // Load WebRTC adapter first (critical for mobile browsers)
            add_debug("Loading WebRTC adapter polyfill".to_string());
            match load_webrtc_adapter().await {
                Ok(_) => {
                    add_debug("WebRTC adapter loaded successfully".to_string());
                }
                Err(e) => {
                    add_debug(format!("WARNING: WebRTC adapter failed to load: {}", e));
                    // Continue anyway, might still work on some browsers
                }
            }
            
            // Check if QuaggaJS is loaded
            let quagga_exists = js_sys::Reflect::has(&window, &JsValue::from_str("Quagga")).unwrap_or(false);
            add_debug(format!("QuaggaJS already loaded: {}", quagga_exists));
            
            if !quagga_exists {
                add_debug("Loading QuaggaJS from CDN".to_string());
                match load_quagga_js().await {
                    Ok(_) => {
                        add_debug("QuaggaJS loaded successfully from CDN".to_string());
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to load QuaggaJS library: {}", e)));
                        add_debug(format!("ERROR: {}", e));
                        return;
                    }
                }
            }
            
            // Wait a bit more for QuaggaJS to be fully ready
            gloo_timers::future::TimeoutFuture::new(300).await;
            
            // Verify container exists
            let document = window.document().unwrap();
            if document.get_element_by_id(&scanner_container_id.read()).is_none() {
                let error_msg = "Scanner container element not found in DOM".to_string();
                error_message.set(Some(error_msg.clone()));
                add_debug(format!("ERROR: {}", error_msg));
                return;
            }
            add_debug("Scanner container found in DOM".to_string());
            
            // Test camera access explicitly before initializing QuaggaJS
            add_debug("Testing camera access...".to_string());
            match test_camera_access().await {
                Ok(_) => {
                    add_debug("Camera access test successful".to_string());
                }
                Err(e) => {
                    add_debug(format!("Camera access test failed: {}", e));
                    error_message.set(Some(format!("Camera access denied or unavailable: {}", e)));
                    return;
                }
            }
            
            // Initialize QuaggaJS with fallback constraints
            match init_quagga_with_fallbacks(&scanner_container_id.read()).await {
                Ok(_) => {
                    add_debug("QuaggaJS initialized successfully".to_string());
                    quagga_initialized.set(true);
                    scanning.set(true);
                    
                    // Set up barcode detection callback
                    set_quagga_callback(move |barcode: String| {
                        web_sys::console::log_1(&JsValue::from_str(&format!("Barcode detected: {}", barcode)));
                        on_scan.call(ScanResult {
                            barcode: barcode.clone(),
                            format: "ean".to_string(),
                        });
                        stop_quagga();
                        on_close.call(());
                    });
                    
                    start_quagga();
                    add_debug("QuaggaJS started, waiting for barcode detection".to_string());
                }
                Err(e) => {
                    error_message.set(Some(format!("Camera initialization failed: {}", e)));
                    add_debug(format!("ERROR: Camera initialization failed: {}", e));
                }
            }
        }
    });

    let cleanup_scanner = move || {
        if *quagga_initialized.read() {
            stop_quagga();
            web_sys::console::log_1(&JsValue::from_str("Scanner stopped and cleaned up"));
        }
    };

    // Start scanner when component mounts
    use_effect(move || {
        start_scanner.send(());
    });

    rsx! {
        div {
            class: "fixed inset-0 z-50 bg-black bg-opacity-75 flex items-center justify-center p-2",
            onclick: move |_| {
                cleanup_scanner();
                on_close.call(());
            },
            
            div {
                class: "relative bg-white rounded-lg w-full max-w-lg md:max-w-4xl max-h-screen overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),
                
                div { class: "flex justify-between items-center p-4 border-b",
                    h2 { class: "text-lg md:text-xl font-bold", "Scan EAN Barcode" }
                    button {
                        class: "text-gray-500 hover:text-gray-700 text-xl",
                        onclick: move |_| {
                            cleanup_scanner();
                            on_close.call(());
                        },
                        "✕"
                    }
                }
                
                div { class: "flex-1 overflow-y-auto p-4 space-y-4",
                    if let Some(error) = error_message.read().as_ref() {
                        div { class: "bg-red-100 text-red-700 p-3 rounded",
                            div { class: "font-bold", "Camera Error:" }
                            div { class: "mt-1", {error.clone()} }
                            div { class: "mt-3 text-sm",
                                strong { "Troubleshooting tips:" }
                                ul { class: "list-disc ml-5 mt-1 space-y-1",
                                    li { "Make sure you clicked \"Allow\" when asked for camera permission" }
                                    li { "Check if another application is using your camera" }
                                    li { "Try refreshing the page and allowing camera access again" }
                                    li { "Make sure your device has a working camera" }
                                    li { "On iOS: Use Safari 11+ or Chrome. On Android: Use Chrome 47+" }
                                    li { "Ensure you're using HTTPS or localhost (not HTTP)" }
                                }
                            }
                        }
                    }
                    
                    div { 
                        id: "{scanner_container_id.read()}",
                        class: "relative bg-black rounded overflow-hidden",
                        style: "min-height: 250px; height: 40vh; max-height: 400px;",
                        
                        if *scanning.read() {
                            div { class: "absolute inset-0 flex items-center justify-center pointer-events-none z-10",
                                div { 
                                    class: "border-2 border-green-500",
                                    style: "width: min(280px, 80%); height: min(80px, 25%);",
                                }
                            }
                        }
                        
                        if !*scanning.read() && error_message.read().is_none() {
                            div { class: "flex items-center justify-center h-full",
                                div { class: "text-white text-center",
                                    div { class: "mb-2", "🎥 Initializing camera..." }
                                    div { class: "text-sm", "Please allow camera access when prompted" }
                                }
                            }
                        }
                    }
                    
                    div { class: "text-center text-sm text-gray-600",
                        "Position the EAN barcode within the green frame to scan"
                    }
                    
                    div { class: "text-center text-xs text-gray-500",
                        "Supports EAN-13 and EAN-8 barcodes"
                    }
                    
                    // Debug information (mobile-friendly)
                    if error_message.read().is_some() && !debug_messages.read().is_empty() {
                        details { class: "border rounded p-2",
                            summary { class: "cursor-pointer text-sm text-gray-600 font-medium", "🔧 Technical Details" }
                            div { class: "mt-2 bg-gray-50 p-2 rounded text-xs font-mono max-h-32 overflow-y-auto space-y-1",
                                for message in debug_messages.read().iter() {
                                    div { class: "break-all", {message.clone()} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Load script using Promise-based approach to avoid closure issues
async fn load_script_from_url(url: &str, timeout_ms: u32) -> Result<(), String> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    
    let script = document.create_element("script").map_err(|e| format!("Failed to create script element: {:?}", e))?;
    let script = script.dyn_into::<web_sys::HtmlScriptElement>().map_err(|e| format!("Failed to cast to script element: {:?}", e))?;
    
    script.set_src(url);
    script.set_type("text/javascript");
    
    let (tx, rx) = futures::channel::oneshot::channel::<Result<(), String>>();
    let tx = std::rc::Rc::new(std::cell::RefCell::new(Some(tx)));
    
    // Create load event handler
    let tx_success = tx.clone();
    let onload = Closure::wrap(Box::new(move || {
        if let Some(sender) = tx_success.borrow_mut().take() {
            let _ = sender.send(Ok(()));
        }
    }) as Box<dyn FnMut()>);
    
    // Create error event handler
    let tx_error = tx.clone();
    let onerror = Closure::wrap(Box::new(move || {
        if let Some(sender) = tx_error.borrow_mut().take() {
            let _ = sender.send(Err("Script failed to load".to_string()));
        }
    }) as Box<dyn FnMut()>);
    
    script.set_onload(Some(onload.as_ref().unchecked_ref()));
    script.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    
    document.body().unwrap().append_child(&script).map_err(|e| format!("Failed to append script: {:?}", e))?;
    
    // Wait for load or timeout
    let result = futures::future::select(
        Box::pin(rx),
        Box::pin(gloo_timers::future::TimeoutFuture::new(timeout_ms))
    ).await;
    
    // Clean up closures
    onload.forget();
    onerror.forget();
    
    match result {
        futures::future::Either::Left((Ok(result), _)) => result,
        futures::future::Either::Left((Err(_), _)) => Err("Script loading was cancelled".to_string()),
        futures::future::Either::Right((_, _)) => Err(format!("Script loading timed out after {}ms", timeout_ms)),
    }
}

// Test camera access explicitly
async fn test_camera_access() -> Result<(), String> {
    match wasm_bindgen_futures::JsFuture::from(test_camera_js()).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{:?}", e)),
    }
}

// Load WebRTC adapter polyfill with multiple CDN fallbacks
async fn load_webrtc_adapter() -> Result<(), String> {
    let window = web_sys::window().unwrap();
    
    // Check if adapter is already loaded
    if js_sys::Reflect::has(&window, &JsValue::from_str("adapter")).unwrap_or(false) {
        return Ok(());
    }
    
    let adapter_urls = vec![
        "https://unpkg.com/webrtc-adapter@latest/out/adapter.js",
        "https://webrtc.github.io/adapter/adapter-latest.js",
        "https://cdn.jsdelivr.net/npm/webrtc-adapter@latest/out/adapter.js",
    ];
    
    for (i, url) in adapter_urls.iter().enumerate() {
        web_sys::console::log_1(&JsValue::from_str(&format!("Trying WebRTC adapter CDN #{}: {}", i + 1, url)));
        
        match load_script_from_url(url, 15000).await {
            Ok(_) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("WebRTC adapter loaded from CDN #{}", i + 1)));
                return Ok(());
            }
            Err(e) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("WebRTC adapter CDN #{} failed: {}", i + 1, e)));
                if i == adapter_urls.len() - 1 {
                    return Err(format!("All WebRTC adapter CDNs failed. Last error: {}", e));
                }
            }
        }
    }
    
    Err("Failed to load WebRTC adapter from any CDN".to_string())
}

// Load QuaggaJS with multiple CDN fallbacks
async fn load_quagga_js() -> Result<(), String> {
    let quagga_urls = vec![
        "https://unpkg.com/@ericblade/quagga2@1.8.4/dist/quagga.min.js",
        "https://cdn.jsdelivr.net/npm/@ericblade/quagga2@1.8.4/dist/quagga.min.js",
        "https://cdnjs.cloudflare.com/ajax/libs/quagga/0.12.1/quagga.min.js",
    ];
    
    for (i, url) in quagga_urls.iter().enumerate() {
        web_sys::console::log_1(&JsValue::from_str(&format!("Trying QuaggaJS CDN #{}: {}", i + 1, url)));
        
        match load_script_from_url(url, 15000).await {
            Ok(_) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("QuaggaJS loaded from CDN #{}", i + 1)));
                return Ok(());
            }
            Err(e) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("QuaggaJS CDN #{} failed: {}", i + 1, e)));
                if i == quagga_urls.len() - 1 {
                    return Err(format!("All QuaggaJS CDNs failed. Last error: {}", e));
                }
            }
        }
    }
    
    Err("Failed to load QuaggaJS from any CDN".to_string())
}

// Enhanced QuaggaJS initialization with fallback constraints
async fn init_quagga_with_fallbacks(container_id: &str) -> Result<(), String> {
    // Try different camera configurations in order of preference
    let configurations = vec![
        // First attempt: High quality with environment camera (mobile rear camera)
        serde_json::json!({
            "inputStream": {
                "name": "Live",
                "type": "LiveStream",
                "target": format!("#{}", container_id),
                "constraints": {
                    "width": { "min": 480, "ideal": 640, "max": 1280 },
                    "height": { "min": 320, "ideal": 480, "max": 720 },
                    "facingMode": "environment"
                }
            },
            "decoder": {
                "readers": ["ean_reader", "ean_8_reader"]
            },
            "locate": true
        }),
        // Second attempt: Any camera, medium quality
        serde_json::json!({
            "inputStream": {
                "name": "Live",
                "type": "LiveStream",
                "target": format!("#{}", container_id),
                "constraints": {
                    "width": { "min": 480, "ideal": 640 },
                    "height": { "min": 320, "ideal": 480 }
                }
            },
            "decoder": {
                "readers": ["ean_reader", "ean_8_reader"]
            },
            "locate": true
        }),
        // Third attempt: Basic constraints
        serde_json::json!({
            "inputStream": {
                "name": "Live",
                "type": "LiveStream",
                "target": format!("#{}", container_id),
                "constraints": {
                    "width": 640,
                    "height": 480
                }
            },
            "decoder": {
                "readers": ["ean_reader", "ean_8_reader"]
            },
            "locate": true
        }),
        // Last attempt: Minimal constraints
        serde_json::json!({
            "inputStream": {
                "name": "Live",
                "type": "LiveStream",
                "target": format!("#{}", container_id)
            },
            "decoder": {
                "readers": ["ean_reader", "ean_8_reader"]
            }
        })
    ];

    for (i, config) in configurations.iter().enumerate() {
        web_sys::console::log_1(&JsValue::from_str(&format!("Trying camera configuration #{}", i + 1)));
        
        match wasm_bindgen_futures::JsFuture::from(init_quagga_js_with_config(&config.to_string())).await {
            Ok(_) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("Camera configuration #{} succeeded", i + 1)));
                return Ok(());
            }
            Err(e) => {
                web_sys::console::log_1(&JsValue::from_str(&format!("Camera configuration #{} failed: {:?}", i + 1, e)));
                if i == configurations.len() - 1 {
                    // Last attempt failed
                    return Err(format!("All camera configurations failed. Last error: {:?}", e));
                }
                // Try next configuration
                continue;
            }
        }
    }
    
    Err("Failed to initialize camera with any configuration".to_string())
}

// JavaScript interop for QuaggaJS and camera testing
#[wasm_bindgen(inline_js = "
let quaggaCallback = null;

export function testCamera() {
    return new Promise((resolve, reject) => {
        if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
            reject('getUserMedia is not supported');
            return;
        }
        
        navigator.mediaDevices.getUserMedia({ video: true })
            .then(stream => {
                // Stop the stream immediately - we just wanted to test access
                stream.getTracks().forEach(track => track.stop());
                resolve();
            })
            .catch(err => {
                console.error('Camera test failed:', err);
                reject(err.message || err.name || 'Camera access denied');
            });
    });
}

export function initQuaggaWithConfig(configJson) {
    return new Promise((resolve, reject) => {
        if (typeof Quagga === 'undefined') {
            reject('Quagga is not loaded');
            return;
        }
        
        let config;
        try {
            config = JSON.parse(configJson);
        } catch (e) {
            reject('Invalid configuration JSON: ' + e.message);
            return;
        }
        
        console.log('Initializing Quagga with config:', config);
        
        Quagga.init(config, function(err) {
            if (err) {
                console.error('Quagga init error:', err);
                reject(err.message || err.toString() || 'Failed to initialize scanner');
                return;
            }
            console.log('Quagga initialized successfully');
            resolve();
        });
    });
}

export function startQuagga() {
    if (typeof Quagga !== 'undefined') {
        Quagga.start();
        
        // Set up detection handler
        Quagga.onDetected(function(result) {
            if (result && result.codeResult && result.codeResult.code) {
                console.log('Barcode detected:', result.codeResult.code);
                
                // Validate EAN checksum for better accuracy
                const code = result.codeResult.code;
                if (validateEAN(code)) {
                    // Call the Rust callback
                    if (quaggaCallback) {
                        quaggaCallback(code);
                    }
                }
            }
        });
        
        // Optional: Process results for debugging
        Quagga.onProcessed(function(result) {
            const drawingCtx = Quagga.canvas.ctx.overlay;
            const drawingCanvas = Quagga.canvas.dom.overlay;
            
            if (result) {
                if (result.boxes) {
                    drawingCtx.clearRect(0, 0, drawingCanvas.width, drawingCanvas.height);
                    result.boxes.filter(function (box) {
                        return box !== result.box;
                    }).forEach(function (box) {
                        Quagga.ImageDebug.drawPath(box, {x: 0, y: 1}, drawingCtx, {color: 'green', lineWidth: 2});
                    });
                }
                
                if (result.box) {
                    Quagga.ImageDebug.drawPath(result.box, {x: 0, y: 1}, drawingCtx, {color: '#00F', lineWidth: 2});
                }
                
                if (result.codeResult && result.codeResult.code) {
                    Quagga.ImageDebug.drawPath(result.line, {x: 'x', y: 'y'}, drawingCtx, {color: 'red', lineWidth: 3});
                }
            }
        });
    }
}

export function stopQuagga() {
    if (typeof Quagga !== 'undefined') {
        Quagga.stop();
        Quagga.offDetected();
        Quagga.offProcessed();
        console.log('Quagga stopped');
    }
}

export function setQuaggaCallback(callback) {
    quaggaCallback = callback;
}

function validateEAN(code) {
    // Basic EAN validation
    if (!code || (code.length !== 8 && code.length !== 13)) {
        return false;
    }
    
    // Check if all characters are digits
    if (!/^\\d+$/.test(code)) {
        return false;
    }
    
    // Calculate and verify checksum
    let sum = 0;
    for (let i = 0; i < code.length - 1; i++) {
        const digit = parseInt(code[i]);
        if (i % 2 === 0) {
            sum += digit;
        } else {
            sum += digit * 3;
        }
    }
    
    const checkDigit = (10 - (sum % 10)) % 10;
    return checkDigit === parseInt(code[code.length - 1]);
}
")]
extern "C" {
    #[wasm_bindgen(js_name = testCamera)]
    fn test_camera_js() -> js_sys::Promise;
    
    #[wasm_bindgen(js_name = initQuaggaWithConfig)]
    fn init_quagga_js_with_config(config_json: &str) -> js_sys::Promise;
    
    #[wasm_bindgen(js_name = startQuagga)]
    fn start_quagga();
    
    #[wasm_bindgen(js_name = stopQuagga)]
    fn stop_quagga();
    
    #[wasm_bindgen(js_name = setQuaggaCallback)]
    fn set_quagga_callback_js(callback: &Closure<dyn Fn(String)>);
}

fn set_quagga_callback<F>(callback: F) 
where 
    F: Fn(String) + 'static
{
    let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn(String)>);
    set_quagga_callback_js(&closure);
    closure.forget(); // Keep the closure alive
}