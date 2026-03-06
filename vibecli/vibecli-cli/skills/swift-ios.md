---
triggers: ["SwiftUI", "Combine", "Core Data", "swift async", "iOS development", "swift app", "ObservableObject"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["swift"]
category: swift
---

# Swift iOS Development

When building iOS apps with Swift:

1. Use SwiftUI for new apps — `View` protocol with `body` computed property
2. State management: `@State` for local, `@StateObject` for owned objects, `@ObservedObject` for passed objects
3. Use `@Published` properties in `ObservableObject` classes for reactive updates
4. Use `async/await` for network calls — `URLSession.shared.data(from: url)`
5. Use `Codable` (Encodable + Decodable) for JSON serialization — `JSONDecoder().decode(T.self, from: data)`
6. Navigation: `NavigationStack` with `NavigationLink` or `.navigationDestination(for:)`
7. Use Core Data with `@FetchRequest` property wrapper for SwiftUI integration
8. Use `Task { }` to launch async work from synchronous SwiftUI event handlers
9. Error handling: use `do/try/catch` — define custom Error enums for domain errors
10. Use `@Environment(\.dismiss)` for dismissing views; `@EnvironmentObject` for shared app state
11. Lists: `List(items) { item in Row(item: item) }` — auto handles scrolling and reuse
12. Use `#Preview` macro for SwiftUI previews — test different states and device sizes
