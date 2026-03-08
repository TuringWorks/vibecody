---
triggers: ["Angular standalone", "angular signals", "angular defer", "angular SSR", "angular zoneless", "angular nx", "angular CDK", "angular schematics", "angular micro frontend"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: frontend
---

# Angular Advanced Patterns

When working with advanced Angular development:

1. Use standalone components by default with `standalone: true`, importing dependencies directly in the component's `imports` array instead of declaring them in NgModules; bootstrap the app with `bootstrapApplication(AppComponent, { providers: [...] })` and use `provideRouter(routes)` for routing.
2. Adopt Angular Signals with `signal()` for reactive state, `computed()` for derived values, and `effect()` for side effects; prefer signals over RxJS `BehaviorSubject` for component state, and use `toSignal()`/`toObservable()` for interop at service boundaries.
3. Use `@defer` blocks to lazy-load heavy components with built-in triggers: `@defer (on viewport)` for below-the-fold content, `@defer (on interaction)` for user-activated features, and `@defer (when condition)` for conditional loading, with `@placeholder`, `@loading`, and `@error` sub-blocks.
4. Implement SSR with Angular Universal using `provideClientHydration()` for incremental DOM hydration that reuses server-rendered HTML; enable event replay so user interactions during hydration are captured and replayed once the component is interactive.
5. Enable zoneless change detection with `provideExperimentalZonelessChangeDetection()` to eliminate Zone.js overhead; trigger change detection explicitly via signals or `ChangeDetectorRef.markForCheck()`, and remove `zone.js` from `polyfills` to reduce bundle size by ~30KB.
6. Architect micro-frontends using Module Federation with `@angular-architects/native-federation`; expose remote components via `exposeModule` in webpack/esbuild config, load them dynamically with `loadRemoteModule()`, and share core dependencies (`@angular/core`, `rxjs`) to avoid duplication.
7. Structure monorepos with Nx by creating libraries per domain (`libs/shared/ui`, `libs/feature-auth`, `libs/data-access`), enforce module boundaries with `@nx/enforce-module-boundaries` lint rule, and use `nx affected --target=test` to run only impacted tests on CI.
8. Build custom UI primitives with Angular CDK: use `Overlay` for dropdowns/modals with `cdkConnectedOverlay` positioning, `A11yModule` for focus trapping and live announcements, `DragDrop` for sortable lists, and `BreakpointObserver` for responsive behavior without CSS-only media queries.
9. Create custom schematics with `@angular-devkit/schematics` by defining a `collection.json`, writing rule factories that compose `template()`, `move()`, `mergeWith()`, and `apply()` transformations; publish them as an npm package and invoke with `ng generate your-package:schematic-name`.
10. Master RxJS operator patterns: use `switchMap` for cancellable requests (search typeahead), `exhaustMap` for non-cancellable actions (form submit), `concatMap` for ordered sequential calls, and `combineLatestWith`/`withLatestFrom` for joining streams; always handle errors with `catchError` returning a fallback observable.
11. Optimize builds by enabling route-level code splitting with `loadComponent` in route definitions, configuring `preloadAllModules` or a custom `PreloadingStrategy` for predictive loading, and using `provideServiceWorker()` for offline caching with a stale-while-revalidate strategy.
12. Write tests using the Angular testing utilities with `TestBed.configureTestingModule({ imports: [MyStandaloneComponent] })`; use `ComponentFixture` with `detectChanges()`, mock services with `jasmine.createSpyObj` or `jest.fn()`, and test signal-based reactivity by setting signal values directly and asserting DOM updates.
