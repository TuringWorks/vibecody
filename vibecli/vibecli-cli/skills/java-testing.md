---
triggers: ["JUnit", "Mockito", "AssertJ", "Testcontainers", "java test", "MockBean", "SpringBootTest"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: testing
---

# Java Testing

When testing Java applications:

1. Use JUnit 5 (`@Test`, `@BeforeEach`, `@ParameterizedTest`) — not JUnit 4
2. Use AssertJ for fluent assertions: `assertThat(result).isEqualTo(expected).isNotNull()`
3. Use Mockito for mocking: `@Mock`, `@InjectMocks`, `when().thenReturn()`, `verify()`
4. Use `@ParameterizedTest` with `@CsvSource` or `@MethodSource` for data-driven tests
5. Use Testcontainers for integration tests with real databases and message queues
6. Test naming: `shouldReturnUser_whenValidId()` or `givenValidId_whenGetUser_thenReturnsUser()`
7. Use `@Nested` classes to group related test cases within a test class
8. Arrange-Act-Assert pattern: setup → execute → verify — one assertion concern per test
9. Mock external services at the HTTP level with WireMock for integration tests
10. Use `@SpringBootTest` sparingly — prefer `@WebMvcTest`, `@DataJpaTest` for sliced tests
11. Use `@Captor` with `ArgumentCaptor` to verify complex arguments passed to mocks
12. Aim for fast tests: mock heavy dependencies, use in-memory databases (H2) for JPA tests
