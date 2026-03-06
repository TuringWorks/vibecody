---
triggers: ["Spring Boot", "spring", "@RestController", "@Autowired", "JPA", "spring security", "java REST"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Java Spring Boot

When building Spring Boot applications:

1. Use `@RestController` for REST APIs — combines `@Controller` + `@ResponseBody`
2. Use constructor injection over `@Autowired` field injection — better for testing and immutability
3. Layer architecture: `@Controller` → `@Service` → `@Repository` (JPA interface)
4. Use `@Validated` + JSR-303 annotations (`@NotNull`, `@Size`, `@Email`) for request validation
5. JPA: extend `JpaRepository<Entity, ID>` — get CRUD methods free; add query methods by naming
6. Use `@Transactional` on service methods — rollback on unchecked exceptions by default
7. Use Spring Profiles (`@Profile("dev")`) for environment-specific beans
8. Exception handling: `@ControllerAdvice` + `@ExceptionHandler` for global error responses
9. Use `application.yml` over `application.properties` — cleaner hierarchy
10. Security: use `SecurityFilterChain` bean — JWT validation with `spring-security-oauth2-resource-server`
11. Use `@Async` for non-blocking operations; `@Scheduled` for recurring tasks
12. Test with `@SpringBootTest` for integration; `@WebMvcTest` for controller-only tests with MockMvc
