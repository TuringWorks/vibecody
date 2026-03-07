---
triggers: ["Grails", "grails", "groovy web", "GORM", "grails plugin", "grails domain"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: groovy
---

# Groovy/Grails Framework

When working with Groovy/Grails:

1. Follow Grails conventions: place domain classes in `grails-app/domain`, controllers in `grails-app/controllers`, services in `grails-app/services`; the framework auto-wires based on naming and location.
2. Define GORM domain classes with constraints and mappings: use `static constraints = { email blank: false, unique: true }` and `static mapping = { table 'users'; cache true }` to control validation and persistence.
3. Use services for business logic, not controllers; annotate with `@Transactional` at the class level and `@Transactional(readOnly = true)` on read-only methods to manage database sessions correctly.
4. Leverage GORM dynamic finders (`findByEmailAndActive`) for simple queries and `where` queries or `DetachedCriteria` for complex ones; avoid HQL unless joining across unrelated entities.
5. Configure data sources in `application.yml` with environment-specific blocks; use `dbCreate: update` in development and `dbCreate: none` with database migration plugin in production.
6. Use Grails interceptors instead of filters for cross-cutting concerns: define `before()`, `after()`, and `afterView()` closures with `match(controller: 'api', action: '*')` for scoped interception.
7. Implement REST APIs with `@Resource(uri='/api/books')` on domain classes for quick CRUD, or use `RestfulController` subclasses with custom `respond` calls for fine-grained control.
8. Use the database migration plugin (`grails-database-migration`) for schema changes in production: run `grails dbm-gorm-diff` to generate changelogs and `grails dbm-update` to apply them.
9. Write tests using Grails testing support: `@Mock([Domain])` for unit tests, `@Integration` for integration tests; use `DomainClassUnitTestMixin` to get save/validate without a database.
10. Configure caching with `grails-cache` plugin: annotate service methods with `@Cacheable('cacheName')` and `@CacheEvict('cacheName')` to avoid redundant database queries.
11. Use Grails profiles and plugins to share functionality: create a plugin with `grails create-plugin`, publish to a local Maven repo, and depend on it via `build.gradle` for cross-project reuse.
12. Optimize GORM performance by enabling second-level cache with `cache: true` on frequently read domains, using `fetch: 'join'` for eager loading, and batching with `batch-size` to eliminate N+1 queries.
