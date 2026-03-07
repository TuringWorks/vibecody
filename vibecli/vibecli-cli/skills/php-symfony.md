---
triggers: ["Symfony", "symfony bundle", "doctrine", "symfony console", "symfony messenger"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["php"]
category: php
---

# Symfony Framework

When working with Symfony:

1. Use autowiring and autoconfiguration by default: define services in `services.yaml` with `_defaults: autowire: true, autoconfigure: true` and let Symfony inject dependencies by type-hint; only register services manually when you need specific arguments or tags.
2. Define routes with PHP attributes `#[Route('/api/users', methods: ['GET'])]` on controller methods for co-located routing; group common prefixes with `#[Route('/api/users')]` on the controller class.
3. Use Doctrine ORM with repository pattern: extend `ServiceEntityRepository`, define custom query methods, and use DQL or QueryBuilder for complex queries; avoid calling `EntityManager` directly in controllers.
4. Run database migrations with `php bin/console doctrine:migrations:diff` to auto-generate, review the SQL, then `doctrine:migrations:migrate`; always test both up and down migrations and version-control the migration files.
5. Use Symfony Messenger for async processing: define message classes (DTOs), handlers with `#[AsMessageHandler]`, and configure transports in `messenger.yaml`; use `doctrine` or `amqp` transport and set retry strategies per transport.
6. Validate input with Validator constraints as attributes on DTOs: `#[Assert\NotBlank]`, `#[Assert\Email]`, `#[Assert\Valid]` for nested objects; inject `ValidatorInterface` and return `ConstraintViolationList` as structured error responses.
7. Implement authentication with Security bundle: define a `UserProvider`, configure firewalls in `security.yaml`, use `#[IsGranted('ROLE_ADMIN')]` attribute on controllers, and implement custom voters for object-level permissions.
8. Write tests with `WebTestCase` for functional tests using `$client = static::createClient()` and `$client->request('GET', '/api/users')`; use `KernelTestCase` for service integration tests and `PHPUnit` with data providers for unit tests.
9. Use Symfony Console for CLI commands: extend `Command`, define with `#[AsCommand(name: 'app:import')]`, implement `execute()`, and use `ProgressBar` and `SymfonyStyle` for user-friendly output; register commands automatically via autoconfiguration.
10. Leverage the Event Dispatcher with custom events and subscribers: create event classes, dispatch with `EventDispatcherInterface`, and subscribe with `#[AsEventListener]`; use event priorities to control execution order.
11. Optimize performance with `APP_ENV=prod`, `composer dump-autoload --optimize`, `php bin/console cache:clear --env=prod`, and enable OPcache with `preload` in php.ini pointing to Symfony's preload file for sub-millisecond autoloading.
12. Deploy using the Symfony CLI or Deployer: run `composer install --no-dev --optimize-autoloader`, warm the cache, run migrations, and use health check endpoints; configure `php-fpm` pool settings (`pm.max_children`, `pm.max_requests`) based on available memory.
