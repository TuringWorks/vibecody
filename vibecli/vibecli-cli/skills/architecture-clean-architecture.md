---
triggers: ["clean architecture", "hexagonal", "ports and adapters", "onion architecture", "dependency inversion", "layers"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Clean Architecture

When applying clean architecture principles:

1. Dependency rule: outer layers depend on inner layers — never the reverse
2. Layers (inside→out): Entities → Use Cases → Interface Adapters → Frameworks/Drivers
3. Entities: core business rules, independent of any framework or database
4. Use Cases: application-specific business rules — orchestrate entities, define ports
5. Ports: interfaces defined by inner layers (e.g., `trait UserRepository`)
6. Adapters: implementations of ports (e.g., `PostgresUserRepository implements UserRepository`)
7. Dependency injection: wire adapters to ports at the composition root (main/startup)
8. Keep frameworks at the edges — your business logic should be framework-agnostic
9. DTOs (Data Transfer Objects) at layer boundaries — don't pass entities to the UI
10. Test use cases with mock adapters — no database, no HTTP, no file system
11. Benefits: testable, independent of UI/DB/framework, easy to swap implementations
12. Don't over-architect: apply to core domain — simple CRUD doesn't need 4 layers
