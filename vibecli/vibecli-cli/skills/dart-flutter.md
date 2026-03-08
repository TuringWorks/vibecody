---
triggers: ["Flutter", "flutter", "flutter widget", "flutter state", "Riverpod", "flutter bloc", "flutter navigation", "flutter test"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["flutter"]
category: dart
---

# Flutter Mobile & Cross-Platform

When working with Flutter:

1. Compose UIs from small, single-responsibility widgets; extract StatelessWidgets for pure display and use StatefulWidget only when local mutable state (animations, form fields) is required.
2. Manage app-wide state with Riverpod providers (`StateNotifierProvider`, `FutureProvider`, `StreamProvider`) or Bloc (`Cubit`/`Bloc` with events); keep business logic out of widgets entirely.
3. Use `GoRouter` or the declarative `Navigator 2.0` API for routing; define routes as constants and pass typed parameters via `extra` or path params to avoid stringly-typed navigation.
4. Implement platform channels (`MethodChannel`, `EventChannel`) for native interop; define a shared method name contract and handle `MissingPluginException` gracefully on unsupported platforms.
5. Configure build flavors with `--flavor` and `--dart-define` for environment-specific settings (API URLs, feature flags); maintain separate `firebase_options_*.dart` files per flavor.
6. Write widget tests with `testWidgets` and `WidgetTester`; use `pumpAndSettle` for animations, `find.byType`/`find.text` for assertions, and mock dependencies with `ProviderScope.overrides` (Riverpod) or `BlocProvider` stubs.
7. Profile performance with Flutter DevTools: check the "Rebuild Stats" tab for unnecessary rebuilds, use `const` constructors wherever possible, and wrap expensive subtrees with `RepaintBoundary`.
8. Use `Theme.of(context)` and `ThemeData` with `ColorScheme.fromSeed` for consistent theming; define text styles in `TextTheme` and reference them via `Theme.of(context).textTheme`.
9. Manage packages via `pubspec.yaml`; pin major versions, run `flutter pub outdated` regularly, and prefer well-maintained pub.dev packages with null-safety and platform support indicators.
10. Optimize list rendering with `ListView.builder` for long lists instead of `ListView(children:)`; use `AutomaticKeepAliveClientMixin` only when tab state must survive tab switches.
11. Handle async operations with `FutureBuilder`/`StreamBuilder` in simple cases, but prefer Riverpod's `AsyncValue` or Bloc states for proper loading/error/data tri-state handling.
12. Set up integration tests in `integration_test/` with `IntegrationTestWidgetsFlutterBinding`; run on real devices or emulators via `flutter test integration_test/ --device-id` for end-to-end validation.
