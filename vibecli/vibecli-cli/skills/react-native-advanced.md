---
triggers: ["React Native Fabric", "expo router", "turbo module", "react native reanimated", "EAS build", "react native performance", "hermes engine", "expo"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: mobile
---

# React Native Advanced

When working with advanced React Native development:

1. Adopt the New Architecture (Fabric renderer + TurboModules) by enabling `newArchEnabled` in `gradle.properties` (Android) and `RCTNewArchEnabled` in Podfile (iOS); migrate native modules to the Codegen spec (`TurboModule`) for synchronous, type-safe JS-to-native calls.
2. Use Expo Router for file-based routing with nested layouts (`_layout.tsx`), dynamic segments (`[id].tsx`), and groups (`(tabs)/`); leverage typed routes with `useLocalSearchParams<>()` and use `router.push`/`router.replace` for navigation with full deep-link support.
3. Build native modules with TurboModules by defining a spec in `NativeModuleName.ts` using `TurboModuleRegistry.getEnforcing`, implementing the native side in Objective-C++/Kotlin, and running Codegen to generate the bridge; this eliminates the old bridge serialization overhead.
4. Implement animations with Reanimated 3 using `useSharedValue`, `useAnimatedStyle`, and `withTiming`/`withSpring`/`withDecay` worklets that run on the UI thread; use `useAnimatedScrollHandler` for scroll-driven animations without JS thread involvement.
5. Handle gestures with `react-native-gesture-handler` by composing `Gesture.Pan()`, `Gesture.Pinch()`, and `Gesture.Tap()` via `Gesture.Simultaneous` or `Gesture.Exclusive`; wrap gesture-driven views in `GestureDetector` and combine with Reanimated shared values for 60fps interactions.
6. Set up CodePush (or EAS Update) for over-the-air JS bundle updates; target deployment keys per environment (staging/production), configure rollback percentages, and use `codePush.sync({ installMode: IMMEDIATE })` for critical fixes or `ON_NEXT_RESTART` for non-urgent updates.
7. Configure EAS Build with `eas.json` profiles for development, preview, and production; use `eas build --profile production --platform all` for store submissions, set up internal distribution for testing, and automate submissions with `eas submit`.
8. Profile performance using Flipper, React DevTools Profiler, and `systrace`; identify JS-to-native bridge bottlenecks, reduce re-renders with `React.memo` and `useCallback`, flatten view hierarchies, and move heavy computation to `InteractionManager.runAfterInteractions`.
9. Optimize Hermes engine usage by ensuring `hermes` is enabled in `app.json` (Expo) or `build.gradle` (bare), using Hermes bytecode precompilation for faster startup, and profiling with `hermes-profile-transformer` to identify hot functions in CPU profiles.
10. Write platform-specific code using `.ios.tsx`/`.android.tsx` file extensions for divergent UIs, `Platform.select()` for inline value switching, and abstract platform differences behind shared hook interfaces (`useHaptics`, `useBiometrics`) to keep feature code platform-agnostic.
11. Manage large lists with `FlashList` (Shopify) instead of `FlatList` for better recycling performance; set `estimatedItemSize`, use `keyExtractor` with stable IDs, avoid inline arrow functions in `renderItem`, and implement `getItemType` for heterogeneous lists.
12. Handle navigation state persistence by serializing `NavigationState` to `AsyncStorage` on state change and restoring it on app launch during development; in production, use deep-link URL parsing with Expo Router's `linking` config for universal links and custom scheme handling.
