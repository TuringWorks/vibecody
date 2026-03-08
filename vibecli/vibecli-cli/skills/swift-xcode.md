---
triggers: ["Xcode", "xcode", "SwiftUI", "swiftui", "xcode project", "xcode build", "swift package manager", "Instruments profiling"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["xcodebuild"]
category: swift
---

# Xcode & SwiftUI Development

When working with Xcode and SwiftUI:

1. Structure SwiftUI views as small composable components; extract subviews into separate structs when a view body exceeds ~40 lines and pass data via explicit parameters or `@Binding`.
2. Use `#Preview` macro (Xcode 15+) for live previews; provide mock data through preview-specific extensions or a `PreviewContainer` that injects sample `@State` and environment objects.
3. Profile with Instruments by selecting Product > Profile (Cmd+I); use the Time Profiler for CPU bottlenecks, Allocations for memory leaks, and SwiftUI Instruments for view body re-evaluations.
4. Organize assets in `.xcassets` catalogs with named color sets and image sets; reference them via `Color("name")` or `Image("name")` and use asset symbol generation for compile-time safety.
5. Manage build configurations (Debug/Release/Staging) in `.xcconfig` files; set `SWIFT_ACTIVE_COMPILATION_CONDITIONS` per config and use `#if` compiler directives for environment-specific code.
6. Configure schemes for different targets (app, tests, UI tests); use scheme environment variables for test overrides and enable Address Sanitizer / Thread Sanitizer in the diagnostics tab.
7. Write unit tests with XCTest; use `XCTAssertEqual`, `XCTAssertThrowsError`, and async `XCTestExpectation` for asynchronous code; keep test targets mirroring the source folder structure.
8. Add dependencies via Swift Package Manager (File > Add Package Dependencies); pin to exact versions or version ranges in `Package.swift` and avoid mixing SPM with CocoaPods when possible.
9. Configure entitlements in the `.entitlements` file for capabilities like App Sandbox, iCloud, Push Notifications, and Keychain Sharing; update the Signing & Capabilities tab to match.
10. Set up provisioning profiles in Xcode's Signing settings; use Automatic Signing for development and manual profiles with match (fastlane) or Xcode Cloud for CI distribution builds.
11. Use `@Observable` (iOS 17+) or `@ObservableObject` with `@Published` for view models; inject shared state via `.environment()` modifier and access with `@Environment` in child views.
12. Automate builds from the command line with `xcodebuild -scheme <name> -configuration Release -archivePath build/ archive` followed by `xcodebuild -exportArchive` for CI/CD pipelines.
