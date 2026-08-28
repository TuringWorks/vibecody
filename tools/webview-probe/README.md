# webview-probe

One binary that measures the voice stack on **the webview engine we actually
ship**, on each platform we ship it.

`wry` is the layer Tauri builds on, so this exercises **WKWebView** on macOS,
**WebView2** on Windows and **WebKitGTK** on Linux — not an approximation of
them, and not a browser standing in for them.

## Why it exists

The three desktop shells all declare `bundle.targets: "all"`, and the release
matrix ships macOS, Linux and Windows. The webview engine is a different
product on each, and a voice pipeline depends on engine behaviour that is not
specified anywhere: whether echo cancellation exists, whether automatic gain
control and noise suppression are exposed, and whether a WebRTC peer can reach
a local server given the engine's ICE candidate policy.

Those questions had been answered on macOS by hand. This makes all three
answerable, and makes the cheap half of it a CI gate.

## Arms

| Arm | What it measures | Audio hardware | Where it runs |
|---|---|---|---|
| `probe` | `getUserMedia`, `RTCPeerConnection`, secure context, and which of `echoCancellation` / `autoGainControl` / `noiseSuppression` the engine exposes. Opens the mic. | mic | manual |
| `transport` | A synthetic 1 kHz track to an in-process `webrtc-rs` peer and back again — ICE state, packets each way, and the tone recovered from the returned track. | **none** | **CI, all three OSes** |
| `aec` | Plays a tone through the speakers and measures what the mic hears, **A/B against an `echoCancellation: false` control arm**. | mic **and** speaker, same room | manual, per platform |

The `aec` arm's control is not optional. Without it, "the mic heard nothing"
and "the tone never played" are the same measurement.

## Running

```bash
cargo run -- --arm transport --out result.json     # exit 0 = pass
cargo run -- --arm probe
cargo run -- --arm aec                             # plays ~4s of 1 kHz tone
```

Exit codes: **0** pass · **1** fail · **2** harness error or timeout (90 s).

## Linux needs a fix that wry does not apply

WebKitGTK ships media capture **and WebRTC** off, and denies the permission
request unless the embedder answers it. Checked against wry 0.53.5: it sets
neither `enable-media-stream` nor `enable-webrtc`, and connects no
`permission-request` signal. Tauri 2.11 adds none of them.

`apply_linux_media_fix` in `src/main.rs` does all three, via
`WebViewExtUnix::webview()`. **The same calls are what the shipping shells
need**, reachable from Tauri through `WebviewWindow::with_webview()`.

`enable-webrtc` is the one that hides best. Off, the constructor is not merely
blocked — it is absent, so the page fails with `Can't find variable:
RTCPeerConnection` and any capability check reads it as "this engine cannot do
WebRTC". That is what the first Linux run of the transport arm measured, and
this arm had never run on Linux before: the build failed there, and the two
engines that did run were green.

## Reading a result

`verdict.pass` drives the exit code. Every result also carries `os`, `arch`,
`engine` and the `peer` counters, so a JSON file identifies the machine it came
from without anyone having to remember.

Absolute dB figures move with room noise — two valid runs measured 40.8 dB and
58.2 dB of suppression on the same hardware. Read the verdict and the
control arm, not the decimal.
