---
triggers: ["NestJS microservice", "nest graphql", "nest websocket", "nest CQRS", "nest guards", "nest interceptors"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: typescript
---

# NestJS Advanced Patterns

When working with advanced NestJS patterns:

1. Implement guards for authorization: `@UseGuards(RolesGuard)` with `canActivate(context: ExecutionContext)` — access the request via `context.switchToHttp().getRequest()` and use `@SetMetadata('roles', ['admin'])` with Reflector for role-based access.
2. Use interceptors for cross-cutting concerns: `@UseInterceptors(TransformInterceptor)` wraps handler execution with `intercept(context, next): Observable` — use `next.handle().pipe(map(data => ({ data, timestamp: Date.now() })))` for response transformation.
3. Build GraphQL APIs with code-first approach: `@Resolver(() => User)` with `@Query()`, `@Mutation()`, `@ResolveField()` decorators — NestJS auto-generates the SDL schema from TypeScript types.
4. Implement CQRS with `@nestjs/cqrs`: define commands (`CreateUserCommand`), handlers (`@CommandHandler(CreateUserCommand)`), and events (`UserCreatedEvent`) — use `EventBus` to decouple write-side effects from command processing.
5. Create microservices with transport layers: `app.connectMicroservice({ transport: Transport.REDIS, options: { host: 'localhost' } })` — use `@MessagePattern('topic')` for request-response and `@EventPattern('event')` for fire-and-forget.
6. Handle WebSockets with `@WebSocketGateway(8080)`: use `@SubscribeMessage('events')` for message handlers and `@WebSocketServer() server: Server` to broadcast — apply guards and interceptors with `@UseGuards()` just like HTTP.
7. Use custom pipes for validation and transformation: `@UsePipes(new ZodValidationPipe(schema))` or create `ParseDatePipe` that converts string params to Date objects — throw `BadRequestException` for invalid input.
8. Implement custom decorators by composing built-in ones: `const Auth = (...roles) => applyDecorators(UseGuards(JwtGuard, RolesGuard), SetMetadata('roles', roles), ApiBearerAuth())` — single decorator handles auth, roles, and Swagger docs.
9. Use `@nestjs/bull` for background job processing: `@Processor('queue')` with `@Process('job-name')` handlers — inject `@InjectQueue('queue') private queue: Queue` in services to add jobs with retry and backoff options.
10. Configure health checks with `@nestjs/terminus`: `@HealthCheck()` endpoint combining `TypeOrmHealthIndicator`, `HttpHealthIndicator`, and custom indicators — expose at `/health` for load balancer probes.
11. Use dynamic modules for configurable providers: `MyModule.forRoot(options)` returns `{ module: MyModule, providers: [{ provide: CONFIG, useValue: options }] }` — use `forRootAsync` with `useFactory` for async configuration with dependency injection.
12. Test with NestJS testing utilities: `const module = await Test.createTestingModule({ providers: [Service, { provide: Repo, useValue: mockRepo }] }).compile()` — use `module.get(Service)` to get instances with mocked dependencies.
