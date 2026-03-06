---
triggers: ["Jetpack Compose", "kotlin android", "ViewModel", "Room database", "coroutine android", "composable"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: kotlin
---

# Kotlin Android Development

When building Android apps with Kotlin:

1. Use Jetpack Compose for UI — `@Composable` functions, not XML layouts
2. State management: `remember { mutableStateOf(value) }` for local state; `ViewModel` for screen state
3. Use `collectAsState()` to observe `StateFlow` from ViewModel in Compose
4. Navigation: use `NavHost` + `NavController` with typed route strings
5. Use `Room` for local database: `@Entity`, `@Dao` with suspend functions, `@Database` class
6. Use `kotlinx.coroutines` with `viewModelScope` for async work in ViewModels
7. Use `Hilt` for dependency injection: `@HiltViewModel`, `@Inject constructor`, `@Module`
8. LazyColumn for lists — equivalent to RecyclerView but declarative
9. Use `Material3` components: `Scaffold`, `TopAppBar`, `FloatingActionButton`
10. Handle configuration changes: ViewModel survives rotation, `rememberSaveable` for simple state
11. Use `Modifier` chain for layout: `.fillMaxWidth().padding(16.dp).clickable { }`
12. Test Compose with `createComposeRule().setContent { }` + `onNodeWithText().assertExists()`
