---
layout: page
title: "Demo 15: Deploy & Database"
permalink: /demos/15-deploy-database/
nav_order: 15
parent: Demos
---


## Overview

This demo covers VibeCody's deployment workflows and database management capabilities. You will learn how to deploy applications, manage database connections, run queries, execute migrations, generate schema DDL, and integrate with Supabase -- all from the CLI and VibeUI.

**Time to complete:** ~20 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../first-run/))
- At least one database accessible (PostgreSQL, MySQL, SQLite, MongoDB, Redis, or DuckDB)
- (Optional) A Supabase project for Supabase integration
- (Optional) VibeUI for the desktop panel experience

## Step-by-Step Walkthrough

### Step 1: Connect to a database

Use the `/db connect` command to establish a connection. VibeCody supports PostgreSQL, MySQL, SQLite, MongoDB, Redis, and DuckDB.

```bash
vibecli repl
> /db connect postgres://user:pass@localhost:5432/myapp
```

Expected output:

```
Connected to PostgreSQL (localhost:5432/myapp)
Server version: PostgreSQL 16.2
Tables: 23 | Views: 4 | Size: 142 MB

Connection saved as: myapp-local
Use /db switch myapp-local to reconnect later.
```

Other connection examples:

```bash
# MySQL
> /db connect mysql://user:pass@localhost:3306/myapp

# SQLite (local file)
> /db connect sqlite://./data/app.db

# MongoDB
> /db connect mongodb://user:pass@localhost:27017/myapp

# Redis
> /db connect redis://localhost:6379

# DuckDB (analytics)
> /db connect duckdb://./analytics.duckdb
```

### Step 2: Run queries interactively

Execute SQL queries directly from the REPL.

```bash
> /db query "SELECT id, name, email, created_at FROM users ORDER BY created_at DESC LIMIT 5"
```

Expected output:

```
Query Results (5 rows, 12ms)
+----+----------------+-------------------------+---------------------+
| id | name           | email                   | created_at          |
+----+----------------+-------------------------+---------------------+
| 42 | Alice Johnson  | alice@example.com       | 2026-03-12 14:22:01 |
| 41 | Bob Chen       | bob.chen@example.com    | 2026-03-11 09:15:33 |
| 40 | Carol Davis    | carol.d@example.com     | 2026-03-10 16:48:55 |
| 39 | Dan Williams   | dan.w@example.com       | 2026-03-09 11:30:22 |
| 38 | Eve Martinez   | eve.m@example.com       | 2026-03-08 08:05:10 |
+----+----------------+-------------------------+---------------------+

5 rows returned in 12ms
```

For MongoDB:

```bash
> /db query '{"collection": "users", "filter": {"active": true}, "limit": 5}'
```

For Redis:

```bash
> /db query "KEYS user:*"
> /db query "HGETALL user:42"
```

### Step 3: Explore database schema

View your database schema with interactive exploration.

```bash
# List all tables
> /db schema

# Describe a specific table
> /db schema users

# Show foreign key relationships
> /db schema --relationships
```

Expected output for `/db schema users`:

```
Table: users
=============
Column        Type            Nullable  Default          Constraints
----------    -----------     --------  ---------------  -----------
id            SERIAL          NO        nextval(seq)     PRIMARY KEY
name          VARCHAR(255)    NO                         NOT NULL
email         VARCHAR(255)    NO                         UNIQUE, NOT NULL
password_hash VARCHAR(255)    NO                         NOT NULL
role          VARCHAR(50)     YES       'user'
created_at    TIMESTAMPTZ     NO        NOW()
updated_at    TIMESTAMPTZ     YES

Indexes:
  users_pkey          PRIMARY KEY (id)
  users_email_key     UNIQUE (email)
  idx_users_role      BTREE (role)
  idx_users_created   BTREE (created_at DESC)

Foreign keys referencing this table:
  orders.user_id     -> users.id (ON DELETE CASCADE)
  sessions.user_id   -> users.id (ON DELETE CASCADE)
  profiles.user_id   -> users.id (ON DELETE CASCADE)

Row count: 1,247  |  Table size: 2.1 MB  |  Index size: 384 KB
```

### Step 4: Run database migrations

VibeCody includes a built-in migration runner that tracks applied migrations and supports rollbacks.

```bash
# Create a new migration
> /db migrate create add_phone_to_users
```

```
Created migration:
  migrations/20260313_143022_add_phone_to_users.sql

Edit the file and add your UP and DOWN SQL:
  -- UP
  ALTER TABLE users ADD COLUMN phone VARCHAR(20);

  -- DOWN
  ALTER TABLE users DROP COLUMN phone;
```

```bash
# Run pending migrations
> /db migrate up
```

```
Running migrations...
  [1/1] 20260313_143022_add_phone_to_users.sql ... OK (23ms)

Applied 1 migration. Current version: 20260313_143022
```

```bash
# Roll back the last migration
> /db migrate down

# Show migration status
> /db migrate status
```

```
Migration Status
=================
  20260101_000001_create_users           applied    2026-01-01
  20260115_120000_add_orders_table       applied    2026-01-15
  20260201_090000_add_sessions           applied    2026-02-01
  20260313_143022_add_phone_to_users     applied    2026-03-13

4 applied, 0 pending
```

### Step 5: Generate schema DDL

Generate DDL (Data Definition Language) for your existing database or from a description.

```bash
# Export DDL for the current database
> /db schema --ddl
```

```
-- Generated by VibeCody on 2026-03-13
-- Database: myapp (PostgreSQL 16.2)

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(50) DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_users_role ON users (role);
CREATE INDEX idx_users_created ON users (created_at DESC);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    total_cents INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ... (23 tables total)

Saved to: .vibecli/generated/schema.sql
```

Generate DDL from a natural language description:

```bash
> /db schema --generate "e-commerce app with users, products, orders, and reviews"
```

```
Generated schema for: e-commerce app
=====================================

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price_cents INTEGER NOT NULL CHECK (price_cents >= 0),
    stock INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    total_cents INTEGER NOT NULL CHECK (total_cents >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price_cents INTEGER NOT NULL
);

CREATE TABLE reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
    body TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, product_id)
);

Saved to: .vibecli/generated/ecommerce-schema.sql
Apply to database? [y/n]:
```

### Step 6: Supabase integration

Connect to your Supabase project for managed PostgreSQL with auth and realtime features.

```bash
# Connect using Supabase project URL and key
> /db connect supabase://your-project-ref --key "eyJ..."

# Or configure in config.toml
```

```toml
[supabase]
project_url = "https://xyzcompany.supabase.co"
anon_key = "eyJ..."
service_role_key = "eyJ..."
```

```bash
# List Supabase tables with RLS status
> /db supabase tables
```

```
Supabase Tables (xyzcompany)
==============================
Table         Rows    RLS     Realtime
----------    -----   ------  --------
users         1,247   ON      OFF
orders        3,891   ON      ON
products      456     OFF     OFF
reviews       2,103   ON      OFF
sessions      892     ON      OFF

Warning: `products` has RLS disabled -- consider enabling it.
```

```bash
# Generate RLS policies
> /db supabase rls products
```

```
Suggested RLS Policies for `products`:
  1. Allow authenticated read:
     CREATE POLICY "products_select" ON products
       FOR SELECT USING (auth.role() = 'authenticated');

  2. Allow admin insert/update:
     CREATE POLICY "products_admin_write" ON products
       FOR ALL USING (auth.jwt() ->> 'role' = 'admin');

Apply policies? [y/n]:
```

### Step 7: Deploy workflows

Use the Deploy panel or REPL commands for one-click deployments.

```bash
# Deploy to a configured target
> /deploy --target staging
```

```
Deploying to staging...
  [1/5] Building release binary        OK  (45s)
  [2/5] Running tests                  OK  (12s, 234 passed)
  [3/5] Building Docker image          OK  (30s)
  [4/5] Pushing to registry            OK  (8s)
  [5/5] Updating ECS service           OK  (25s)

Deployment complete!
  URL:      https://staging.myapp.com
  Image:    ghcr.io/myorg/myapp:v1.2.3-rc1
  Duration: 2m 00s
```

```bash
# Check deployment status
> /deploy status

# Roll back to the previous version
> /deploy rollback --target staging

# View deployment history
> /deploy history --target staging --limit 5
```

### Step 8: Use the Deploy and Database panels in VibeUI

Open VibeUI and explore the **Deploy** and **Database** panels.

```bash
cd vibeui && npm run tauri dev
```

**Deploy Panel:**
- Pipeline view with build/test/push/deploy stages
- One-click deploy buttons for each configured target
- Rollback with confirmation dialog
- Deployment history timeline

**Database Panel:**
- Connection manager with saved connections
- SQL editor with syntax highlighting, autocomplete, and results table
- Schema explorer tree with table/column/index details
- Migration runner with diff preview before applying
- Visual ERD (entity-relationship diagram) generated from schema

## Demo Recording

```json
{
  "meta": {
    "title": "Deploy & Database Management",
    "description": "Connect to databases, run queries, execute migrations, generate schemas, and deploy applications from VibeCody.",
    "duration_seconds": 480,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/db connect sqlite://./demo.db", "delay_ms": 2000 }
      ],
      "description": "Connect to a local SQLite database"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/db schema --generate \"blog with users, posts, comments, and tags\"", "delay_ms": 4000 }
      ],
      "description": "Generate a schema from a natural language description"
    },
    {
      "id": 3,
      "action": "Narrate",
      "value": "VibeCody generated a complete DDL with tables, foreign keys, indexes, and constraints."
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/db query \"SELECT name FROM sqlite_master WHERE type='table'\"", "delay_ms": 2000 }
      ],
      "description": "List all tables in the database"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/db query \"INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')\"", "delay_ms": 1500 },
        { "input": "/db query \"SELECT * FROM users\"", "delay_ms": 1500 }
      ],
      "description": "Insert and query data"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/db schema users", "delay_ms": 2000 }
      ],
      "description": "Inspect the users table schema in detail"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/db migrate create add_avatar_to_users", "delay_ms": 1500 },
        { "input": "/db migrate up", "delay_ms": 2000 },
        { "input": "/db migrate status", "delay_ms": 1500 }
      ],
      "description": "Create and run a database migration"
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/db schema --ddl", "delay_ms": 3000 }
      ],
      "description": "Export the full database DDL"
    },
    {
      "id": 9,
      "action": "repl",
      "commands": [
        { "input": "/db connect postgres://user:pass@localhost:5432/myapp", "delay_ms": 2000 }
      ],
      "description": "Connect to a PostgreSQL database"
    },
    {
      "id": 10,
      "action": "repl",
      "commands": [
        { "input": "/db schema --relationships", "delay_ms": 3000 }
      ],
      "description": "View foreign key relationships across all tables"
    },
    {
      "id": 11,
      "action": "Narrate",
      "value": "Now switching to Supabase integration for managed PostgreSQL."
    },
    {
      "id": 12,
      "action": "repl",
      "commands": [
        { "input": "/db supabase tables", "delay_ms": 2000 },
        { "input": "/db supabase rls products", "delay_ms": 3000 }
      ],
      "description": "List Supabase tables and generate RLS policies"
    },
    {
      "id": 13,
      "action": "Narrate",
      "value": "Finally, let's deploy the application to staging."
    },
    {
      "id": 14,
      "action": "repl",
      "commands": [
        { "input": "/deploy --target staging", "delay_ms": 8000 }
      ],
      "description": "Deploy the application to the staging environment"
    },
    {
      "id": 15,
      "action": "repl",
      "commands": [
        { "input": "/deploy status", "delay_ms": 2000 },
        { "input": "/deploy history --target staging --limit 5", "delay_ms": 2000 }
      ],
      "description": "Check deployment status and history"
    },
    {
      "id": 16,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to explore the Database panel",
      "delay_ms": 8000
    },
    {
      "id": 17,
      "action": "Navigate",
      "target": "panel://database",
      "description": "Open the Database panel in VibeUI"
    },
    {
      "id": 18,
      "action": "Screenshot",
      "label": "database-schema-explorer",
      "description": "Capture the schema explorer with table tree and ERD"
    },
    {
      "id": 19,
      "action": "Navigate",
      "target": "panel://deploy",
      "description": "Open the Deploy panel"
    },
    {
      "id": 20,
      "action": "Screenshot",
      "label": "deploy-pipeline-view",
      "description": "Capture the deployment pipeline with stage indicators"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `Connection refused` | Ensure the database server is running and accepting connections on the specified port |
| `Authentication failed` | Double-check username, password, and database name in the connection string |
| `Migration failed` | Run `/db migrate status` to see which migration failed, then fix the SQL and retry |
| `Permission denied` on DDL export | Your database user needs `SELECT` on `information_schema` (PostgreSQL) or equivalent |
| Supabase `401 Unauthorized` | Verify your `anon_key` or `service_role_key` in config.toml |
| Deploy timeout | Check network connectivity to the container registry and deployment target |

## What's Next

- [Demo 16: MCP Server Integration](../16-mcp-servers/) -- Connect external tool servers via Model Context Protocol
- [Demo 14: Cloud Provider Integration](../14-cloud-providers/) -- Scan for cloud usage and generate IaC
- [Demo 11: Docker & Container Management](../11-docker/) -- Build and manage containers
