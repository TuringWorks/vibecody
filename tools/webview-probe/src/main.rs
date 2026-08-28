//! GV1C — one harness, three webview engines.
//!
//! Hosts the voice-stack probes in a `wry` webview. `wry` is the layer Tauri
//! builds on, so this exercises **WKWebView on macOS, WebView2 on Windows and
//! WebKitGTK on Linux** — the engines we actually ship — rather than an
//! approximation of them.
//!
//! Two arms, split by what they need:
//!
//! * `--arm transport` — synthetic tone against an in-process `webrtc-rs`
//!   peer. Needs no audio hardware, so it is a CI gate on all three OSes.
//! * `--arm aec` — plays a tone through the speakers and measures what the
//!   microphone hears. Needs a real mic and speaker in one room, so it is a
//!   manual, per-platform measurement.
//! * `--arm probe` — capability report only.
//!
//! Exit codes: 0 pass · 1 fail · 2 harness error.

mod peer;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

const PAGE_TRANSPORT: &str = include_str!("../web/transport.html");
const PAGE_AEC: &str = include_str!("../web/aec.html");
const PAGE_PROBE: &str = include_str!("../web/probe.html");

#[derive(Clone, Copy, PartialEq)]
enum Arm { Transport, Aec, Probe }

impl Arm {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "transport" => Some(Arm::Transport),
            "aec" => Some(Arm::Aec),
            "probe" => Some(Arm::Probe),
            _ => None,
        }
    }
    fn page(self) -> &'static str {
        match self { Arm::Transport => PAGE_TRANSPORT, Arm::Aec => PAGE_AEC, Arm::Probe => PAGE_PROBE }
    }
    fn name(self) -> &'static str {
        match self { Arm::Transport => "transport", Arm::Aec => "aec", Arm::Probe => "probe" }
    }
}

fn engine() -> &'static str {
    if cfg!(target_os = "macos") { "WKWebView (WebKit)" }
    else if cfg!(target_os = "windows") { "WebView2 (Chromium)" }
    else if cfg!(target_os = "linux") { "WebKitGTK" }
    else { "unknown" }
}

/// Serve the page and the signaling endpoints on loopback.
///
/// A `http://127.0.0.1` origin is a secure context on every engine, which
/// `getUserMedia` and `RTCPeerConnection` both require. A custom scheme would
/// differ per platform, which is the one thing this harness must not do.
fn serve(port: u16, arm: Arm, obs: peer::Shared, t0: Instant) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let server = tiny_http::Server::http(("127.0.0.1", port)).expect("bind");
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let cors = tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap();
            if req.method() == &tiny_http::Method::Options {
                let h = tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap();
                let _ = req.respond(tiny_http::Response::empty(204).with_header(cors).with_header(h));
                continue;
            }
            if url.starts_with("/offer") {
                let mut sdp = String::new();
                let _ = std::io::Read::read_to_string(req.as_reader(), &mut sdp);
                let obs2 = Arc::clone(&obs);
                match rt.block_on(peer::answer(sdp, obs2, t0)) {
                    Ok(ans) => { let _ = req.respond(tiny_http::Response::from_string(ans).with_header(cors)); }
                    Err(e) => {
                        if let Ok(mut o) = obs.lock() { o.error = Some(e.to_string()); }
                        let _ = req.respond(tiny_http::Response::from_string(format!("ERR {e}"))
                            .with_status_code(500).with_header(cors));
                    }
                }
                continue;
            }
            if url.starts_with("/peer-stats") {
                let body = obs.lock().map(|o| peer::snapshot(&o).to_string()).unwrap_or_else(|_| "{}".into());
                let ct = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                let _ = req.respond(tiny_http::Response::from_string(body).with_header(cors).with_header(ct));
                continue;
            }
            let ct = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let _ = req.respond(tiny_http::Response::from_string(arm.page()).with_header(ct));
        }
    });
}

/// WebKitGTK ships media capture **off**, and denies the permission request
/// unless the embedder answers it. wry does neither: it never sets
/// `enable-media-stream`, and connects no `permission-request` signal (checked
/// against wry 0.53.5). Tauri does not add them either — so this is what every
/// Tauri app on Linux is missing, and it is why the harness must apply it
/// itself before it can claim to have *tested* Linux.
#[cfg(target_os = "linux")]
fn apply_linux_media_fix(webview: &wry::WebView) {
    // `ObjectExt` is what carries `is::<T>()` — the downcast check that tells a
    // microphone request apart from a geolocation one. It is a glib trait
    // rather than a webkit2gtk one, which is why importing the webkit traits
    // alone left the call unresolved.
    use webkit2gtk::glib::ObjectExt;
    use webkit2gtk::{PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, WebViewExt};
    use wry::WebViewExtUnix;
    let wv = webview.webview();
    if let Some(settings) = WebViewExt::settings(&wv) {
        settings.set_enable_media_stream(true);
        settings.set_enable_webaudio(true);
    }
    wv.connect_permission_request(|_, req| {
        if req.is::<UserMediaPermissionRequest>() { req.allow(); return true; }
        false
    });
}
#[cfg(not(target_os = "linux"))]
fn apply_linux_media_fix(_webview: &wry::WebView) {}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let arm = get("--arm").as_deref().and_then(Arm::parse).unwrap_or(Arm::Transport);
    let out = get("--out").unwrap_or_else(|| format!("gv1c-{}.json", arm.name()));
    let port: u16 = get("--port").and_then(|p| p.parse().ok()).unwrap_or(8907);

    let t0 = Instant::now();
    let obs: peer::Shared = Arc::new(Mutex::new(peer::Obs::default()));
    serve(port, arm, Arc::clone(&obs), t0);

    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title(format!("gv1c {} — {}", arm.name(), engine()))
        .with_inner_size(tao::dpi::LogicalSize::new(760.0, 520.0))
        .build(&event_loop)
        .expect("window");

    let result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&result);
    let webview = WebViewBuilder::new()
        .with_url(format!("http://127.0.0.1:{port}/"))
        .with_ipc_handler(move |req| {
            if let Ok(mut g) = sink.lock() { *g = Some(req.body().to_string()); }
        })
        .build(&window)
        .expect("webview");
    apply_linux_media_fix(&webview);

    let started = Instant::now();
    let mut code = 2;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
        let finished = result.lock().ok().and_then(|g| g.clone());
        if let Some(body) = finished {
            let mut v: serde_json::Value =
                serde_json::from_str(&body).unwrap_or_else(|_| serde_json::json!({"raw": body}));
            if let Some(map) = v.as_object_mut() {
                map.insert("engine".into(), serde_json::json!(engine()));
                map.insert("os".into(), serde_json::json!(std::env::consts::OS));
                map.insert("arch".into(), serde_json::json!(std::env::consts::ARCH));
                map.insert("arm".into(), serde_json::json!(arm.name()));
                if let Ok(o) = obs.lock() { map.insert("peer".into(), peer::snapshot(&o)); }
            }
            let pretty = serde_json::to_string_pretty(&v).unwrap_or(body);
            let _ = std::fs::write(&out, &pretty);
            println!("{pretty}");
            code = if v.get("verdict").and_then(|x| x.get("pass")).and_then(|b| b.as_bool()).unwrap_or(false) { 0 } else { 1 };
            *control_flow = ControlFlow::Exit;
        }
        // Never let a hung engine hang CI.
        if started.elapsed().as_secs() > 90 {
            eprintln!("gv1c: TIMEOUT after 90s on {}", engine());
            code = 2;
            *control_flow = ControlFlow::Exit;
        }
        if *control_flow == ControlFlow::Exit {
            std::process::exit(code);
        }
    });
}
