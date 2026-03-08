---
triggers: ["Tizen", "tizen", "tizen studio", "tizen wearable", "tizen tv", "tizen .net", "tizen web app"]
tools_allowed: ["read_file", "write_file", "bash"]
category: tizen
---

# Tizen App Development

When working with Tizen:

1. Choose the right app model based on target: .NET (C# with NUI/Xamarin.Forms) for wearables and appliances, Web (HTML/CSS/JS with Tizen Web APIs) for TV apps, or Native (C/C++ with EFL) for performance-critical applications.
2. Set up Tizen Studio with the Package Manager; install the target platform SDK (wearable 5.5, mobile 6.0, TV 6.5) and device-specific extensions for emulator images and signing profiles.
3. Define app privileges in `tizen-manifest.xml` (native/.NET) or `config.xml` (web); request only required privileges (`http://tizen.org/privilege/internet`, `healthinfo`, `location`) to pass store review.
4. Build wearable UIs with circular layout using `CircularUI` (NUI) or `tau.CircleProgressBar` (web); design for the 360x360 screen with swipe-based navigation and minimal tap targets.
5. Develop TV apps as web applications using the Tizen TV Web API; handle remote control key events (`tizen.tvinputdevice.registerKey`) and optimize for 10-foot UI with large fonts and focus navigation.
6. Use the Tizen Emulator Manager to test across profiles (phone, watch, TV); connect to physical devices via `sdb connect <ip>` and deploy with `tizen install -n <tpk>` for on-device testing.
7. Package native apps with `tizen package -t tpk` and web apps with `tizen package -t wgt`; sign with a distributor certificate from the Tizen security profile configured in Tizen Studio.
8. Implement background services with Tizen Service Applications; register them in the manifest and communicate with the UI app via `MessagePort` API for inter-app data exchange.
9. Access sensor data on wearables through `tizen.sensorservice` (web) or `Tizen.Sensor` namespace (.NET); request the sensor privilege, register listeners with appropriate intervals, and stop sensing when inactive.
10. Persist data locally with Tizen's Web Storage, IndexedDB (web apps), or SQLite via `Tizen.Applications.Preference` (.NET); avoid heavy file I/O on wearables due to limited flash storage.
11. Optimize memory and CPU on resource-constrained devices by using `app_event_low_memory_cb` handlers, releasing caches aggressively, and profiling with Tizen Studio's Dynamic Analyzer.
12. Distribute apps through the Samsung Galaxy Store (formerly Tizen Store); generate a release certificate, set the `api-version` range in the manifest, and validate with `tizen cli validate` before submission.
