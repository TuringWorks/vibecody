---
triggers: ["React Native", "Expo", "mobile app", "react navigation", "native module", "mobile performance"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: mobile
---

# React Native & Expo

When building mobile apps with React Native:

1. Use Expo for new projects — managed workflow handles build, signing, OTA updates
2. Navigation: use `@react-navigation/native` — Stack, Tab, Drawer navigators
3. Styling: use `StyleSheet.create()` — similar to CSS but uses flexbox (column default)
4. Platform-specific: `Platform.OS === 'ios'` or `Platform.select({ ios: {}, android: {} })`
5. Lists: use `FlatList` for long lists — virtualizes off-screen items automatically
6. Images: use `expo-image` or `react-native-fast-image` — handle caching and resizing
7. State: use Zustand or TanStack Query — same patterns as React web
8. Storage: `expo-secure-store` for credentials, `AsyncStorage` for non-sensitive data
9. Animations: use `react-native-reanimated` — runs animations on the UI thread (60fps)
10. Testing: `@testing-library/react-native` for component tests, Detox for E2E
11. Performance: avoid inline styles/functions in `FlatList`, memoize expensive renders
12. OTA updates: use EAS Update for instant JS updates without App Store review
