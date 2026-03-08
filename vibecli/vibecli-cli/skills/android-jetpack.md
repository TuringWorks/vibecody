---
triggers: ["Jetpack Compose", "android compose", "compose ui", "android viewmodel", "android room", "hilt android", "android navigation compose", "material3 android"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gradle"]
category: android
---

# Android Jetpack Compose & Modern Android

When working with Android Jetpack Compose:

1. Build UI with composable functions that take parameters and emit UI; keep composables stateless by hoisting state to the caller and accepting lambdas for events (state hoisting pattern).
2. Use `NavHost` with `composable()` route declarations for navigation; pass arguments via route parameters (`"detail/{id}"`) and use `SavedStateHandle` in ViewModels to retrieve them type-safely.
3. Scope ViewModels to navigation graph destinations or activities with `hiltViewModel()` or `viewModel()`; expose UI state as `StateFlow` and collect in composables with `collectAsStateWithLifecycle()`.
4. Define Room databases with `@Entity`, `@Dao`, and `@Database` annotations; return `Flow<List<T>>` from DAO queries for reactive UI updates and run writes in `withContext(Dispatchers.IO)`.
5. Configure Hilt dependency injection with `@HiltAndroidApp` on Application, `@AndroidEntryPoint` on activities/fragments, `@HiltViewModel` on ViewModels, and `@Module`/`@Provides` for custom bindings.
6. Use Kotlin Coroutines with `viewModelScope.launch` for ViewModel async work and `Flow` operators (`map`, `combine`, `flatMapLatest`) for reactive data pipelines from repository to UI.
7. Schedule background tasks with WorkManager using `OneTimeWorkRequest` or `PeriodicWorkRequest`; define `Worker` subclasses with `doWork()` and chain dependent work with `then()`.
8. Replace SharedPreferences with DataStore (`Preferences DataStore` for key-value, `Proto DataStore` for typed schemas); access via `Flow` and edit with `dataStore.edit { prefs -> }` suspending calls.
9. Apply Material 3 theming with `MaterialTheme` composable, defining `colorScheme` from `dynamicColorScheme` (Android 12+) with fallback, `typography`, and `shapes` for consistent design tokens.
10. Write composable tests with `createComposeRule()`; use `onNodeWithText`, `onNodeWithTag`, `performClick`, and `assertIsDisplayed` for UI assertions; inject fake repositories via Hilt test modules.
11. Optimize recomposition by using stable types, marking classes with `@Stable` or `@Immutable`, using `key()` in loops, and checking recomposition counts in Layout Inspector's composition column.
12. Use `LazyColumn`/`LazyRow` with `items(key = { })` for efficient list rendering; provide unique stable keys, use `contentType` for heterogeneous lists, and prefetch with `LazyListState` scroll detection.
