---
triggers: ["XCTest", "swift test", "UI testing swift", "snapshot test swift", "swift unit test"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["swift"]
category: testing
---

# Swift Testing

When testing Swift applications:

1. Use `XCTest` framework — `XCTestCase` subclass with `test*` method naming
2. Use `XCTAssertEqual`, `XCTAssertTrue`, `XCTAssertNil`, `XCTAssertThrowsError` for assertions
3. Use `setUp()` / `tearDown()` (or `setUpWithError()`) for test fixture initialization
4. Test async code with `func testAsync() async throws { let result = await fetchData() }`
5. Use `expectation(description:)` + `wait(for:timeout:)` for callback-based async testing
6. UI testing: `XCUIApplication().launch()` → query with `app.buttons["Label"]` → `tap()`
7. Use `XCTAssertNoThrow` for operations that should succeed silently
8. Mock dependencies with protocols — create `MockService: ServiceProtocol` for injection
9. Use `@testable import ModuleName` to access internal symbols in tests
10. Snapshot testing with `swift-snapshot-testing` library: `assertSnapshot(matching: view)`
11. Use `measure { }` blocks for performance testing — Xcode shows baseline comparisons
12. Test View Models separately from Views — ViewModels should be UI-framework-independent
