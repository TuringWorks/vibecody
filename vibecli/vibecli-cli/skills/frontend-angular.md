---
triggers: ["Angular", "angular", "angular component", "angular service", "angular signals", "NgRx", "angular routing", "angular form", "angular universal"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: frontend
---

# Angular Framework

When working with Angular:

1. Generate standalone components with `ng generate component --standalone`; use `imports` array directly in `@Component` to declare dependencies and avoid NgModule boilerplate in modern Angular (v15+).
2. Use Angular Signals (`signal()`, `computed()`, `effect()`) for fine-grained reactivity; prefer signals over RxJS for component-local state and reserve Observables for async streams and HTTP.
3. Handle HTTP calls with `HttpClient` injected via `inject()` function; create typed service methods returning `Observable<T>` and handle errors with `catchError` in a shared interceptor.
4. Configure routing with `provideRouter(routes)` in `app.config.ts`; use lazy loading with `loadComponent` for standalone components and `loadChildren` for feature route groups to reduce initial bundle size.
5. Build reactive forms with `FormBuilder`, `FormGroup`, and `FormControl`; attach validators (`Validators.required`, custom async validators) and display errors conditionally with `control.hasError('name')`.
6. Implement dependency injection with `providedIn: 'root'` for singletons, component-level `providers` for scoped instances, and `InjectionToken` for interface-based or value-based injection.
7. Create HTTP interceptors with `HttpInterceptorFn` (functional style) for auth tokens, logging, and error handling; register them with `provideHttpClient(withInterceptors([authInterceptor]))`.
8. Manage complex global state with NgRx Store; define actions, reducers, and selectors in feature slices, use `createEffect` for side effects, and access state via `store.select()` in components.
9. Write component tests with `TestBed.configureTestingModule`; use `ComponentFixture` for DOM assertions, `HttpTestingController` for HTTP mocks, and `fakeAsync`/`tick` for time-dependent logic.
10. Enable server-side rendering with Angular Universal via `ng add @angular/ssr`; implement `TransferState` to avoid duplicate HTTP calls between server and client hydration.
11. Optimize performance with `ChangeDetectionStrategy.OnPush` on components; use `trackBy` in `@for` loops, lazy load images with `NgOptimizedImage`, and audit bundle size with `ng build --stats-json`.
12. Use Angular CLI schematics (`ng generate service/pipe/guard/directive`) to scaffold code consistently; create custom schematics in `schematics/` for project-specific patterns and enforce team conventions.
