---
triggers: ["Laravel", "laravel eloquent", "laravel artisan", "laravel livewire", "laravel queue", "laravel sanctum"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["php"]
category: php
---

# Laravel Framework

When working with Laravel:

1. Use Form Request classes (`php artisan make:request StoreUserRequest`) for validation and authorization logic; keep controllers thin by moving validation rules, `authorize()` checks, and custom messages into dedicated request classes.
2. Define Eloquent relationships with eager loading: use `->with(['posts', 'posts.comments'])` in queries and `$with` property on models for default eager loads to prevent N+1 query problems; verify with Laravel Debugbar or Telescope.
3. Use database transactions with `DB::transaction(function () { ... })` for multi-model operations; for nested transactions, rely on savepoints and always test rollback scenarios in your test suite.
4. Queue long-running tasks with `dispatch(new ProcessOrder($order))` and configure `queue.php` for Redis or SQS; implement `ShouldQueue` on jobs, set `$tries`, `$backoff`, and `$maxExceptions`, and handle `failed()` for dead-letter processing.
5. Implement API authentication with Sanctum for SPA and mobile apps: use `auth:sanctum` middleware, issue tokens with `$user->createToken('api')`, and scope abilities with `tokenCan()` for fine-grained access control.
6. Use Laravel Policies for authorization: `php artisan make:policy PostPolicy --model=Post`, register in `AuthServiceProvider`, and call `$this->authorize('update', $post)` in controllers for clean, testable permission logic.
7. Write Feature tests with `$this->actingAs($user)->postJson('/api/posts', $data)->assertCreated()->assertJsonStructure([...])` and use database factories with `Post::factory()->count(5)->create()` for realistic test data.
8. Optimize for production: run `php artisan config:cache`, `route:cache`, `view:cache`, and `event:cache`; use `php artisan optimize` as a single command and ensure `APP_DEBUG=false` and `APP_ENV=production`.
9. Use Livewire for interactive UI components: keep component state minimal, use `wire:model.lazy` instead of `wire:model` to reduce round-trips, and implement `Renderless` components for logic-only concerns.
10. Structure large applications with domain directories: group Models, Actions, DTOs, and Enums by domain (`app/Domain/Billing/`) and use Action classes (`CreateInvoiceAction`) instead of fat service classes for single-responsibility operations.
11. Use Laravel Pint for code style enforcement (`./vendor/bin/pint`), PHPStan or Larastan at level 8 for static analysis, and configure both in CI to catch issues before merge.
12. Deploy with `php artisan down --secret="bypass-token"`, run migrations with `php artisan migrate --force`, clear and rebuild caches, then `php artisan up`; use zero-downtime deployers like Envoyer or Deployer with symlinked releases.
