---
triggers: ["Cocoa", "AppKit", "NSWindow", "NSViewController", "macOS app", "core data mac", "cocoa bindings", "mac app development"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["xcodebuild"]
category: swift
---

# Cocoa / AppKit macOS Development

When working with Cocoa and AppKit:

1. Use `NSWindowController` subclasses to manage window lifecycle and `NSViewController` for content; load views from XIBs or build them programmatically in `loadView()` with Auto Layout constraints.
2. Implement Cocoa Bindings with `@objc dynamic` properties on your model/controller and bind controls in Interface Builder or via `bind(_:to:withKeyPath:options:)` for two-way data flow without glue code.
3. Model your data layer with Core Data using `NSManagedObject` subclasses; define the schema in `.xcdatamodeld`, use `NSFetchedResultsController` for table views, and perform writes on background contexts with `perform {}`.
4. Adopt `NSDocument`-based architecture for document-centric apps; override `data(ofType:)` and `read(from:ofType:)` for serialization, and get free window management, undo, and autosave support.
5. Configure App Sandbox entitlements in the `.entitlements` file; request only the minimum capabilities (network, file access, camera) and use Security-Scoped Bookmarks for persistent file access outside the sandbox.
6. Use Combine publishers for reactive patterns in AppKit; bind `@Published` properties to UI updates via `sink` or `assign`, and cancel subscriptions in `deinit` with `AnyCancellable` storage.
7. Notarize your app for distribution outside the Mac App Store by running `xcrun notarytool submit app.zip --apple-id <email> --team-id <team> --wait`; staple the ticket with `xcrun stapler staple App.app`.
8. Integrate Sparkle for auto-updates by adding the SPM package, configuring an `appcast.xml` feed URL in Info.plist, and signing updates with `generate_keys` / EdDSA signatures.
9. Build menu bar apps (agent apps) by setting `LSUIElement = YES` in Info.plist, creating an `NSStatusItem` in `applicationDidFinishLaunching`, and attaching an `NSMenu` with action targets.
10. Handle keyboard shortcuts by implementing `keyEquivalent` on `NSMenuItem` or overriding `keyDown(with:)` in views; use `NSEvent.addLocalMonitorForEvents(matching:)` for global hotkey capture within the app.
11. Manage multiple windows with `NSWindowController` instances tracked in the app delegate or a window manager; use `NSWindow.delegate` methods (`windowWillClose`) to clean up resources and remove references.
12. Write UI tests with `XCUIApplication` for AppKit by launching the app, querying `menuBars`, `windows`, `buttons`, and `textFields`, and asserting element existence and values for automated regression testing.
