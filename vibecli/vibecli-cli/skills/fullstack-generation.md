# Full-Stack Code Generation

Generate complete frontend + backend + database + infrastructure in one pass.

## Triggers
- "full-stack generation", "generate app", "scaffold full stack"
- "create project", "generate frontend backend", "full app"

## Usage
```
/fullstack create "User management API"       # Generate project
/fullstack --frontend react --backend fastapi  # Specify frameworks
/fullstack --db postgresql --auth jwt          # Database and auth
/fullstack estimate                            # Estimate files/lines
/fullstack frontend                            # Generate frontend only
/fullstack backend                             # Generate backend only
/fullstack infra                               # Generate infra only
/fullstack tests                               # Generate tests only
```

## Supported Stacks
- **Frontend:** React, Vue, Angular, Svelte, Next.js, Nuxt, SvelteKit
- **Backend:** Express, FastAPI, Django, Spring Boot, Actix, Gin, Rails
- **Database:** PostgreSQL, MySQL, SQLite, MongoDB, Redis, DynamoDB
- **Auth:** JWT, OAuth2, Session-based, API Key, SAML

## Features
- 6 project layers: Frontend, Backend, Database, Infrastructure, Testing, Documentation
- 9 file types: Component, Route, Model, Controller, Migration, Config, Test, Dockerfile, Readme
- Template-based generation per framework
- Automatic Dockerfile + docker-compose generation
- CI/CD pipeline scaffolding
- Test file generation per layer
- File and line count estimation
