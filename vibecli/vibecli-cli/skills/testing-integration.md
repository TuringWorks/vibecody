---
triggers: ["integration test", "testcontainers", "API mock", "end to end", "E2E test", "fixture", "test database"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Integration Testing

When writing integration tests:

1. Use Testcontainers for real database/service instances — Docker containers per test suite
2. Test the full request-response cycle: HTTP request → handler → service → DB → response
3. Use fixtures or factories for test data — avoid sharing mutable state between tests
4. Reset database state between tests: transactions (fast) or truncation (thorough)
5. Mock external services at the HTTP boundary with WireMock or MSW (Mock Service Worker)
6. Test happy paths AND error paths: 400 validation, 401 unauthorized, 404 not found, 500 errors
7. Use a separate test configuration with its own database — never test against production
8. Test API contracts: request format, response schema, status codes, headers
9. E2E tests: use Playwright or Cypress — test critical user journeys, not every UI detail
10. Keep integration tests focused — test one integration point per test case
11. Use CI-specific Docker compose for service dependencies (Postgres, Redis, Kafka)
12. Tag slow integration tests separately — run fast unit tests first in CI
