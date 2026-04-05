---
layout: page
title: "Demo 27: HTTP Playground"
permalink: /demos/http-playground/
nav_order: 27
parent: Demos
---


## Overview

This demo covers VibeCody's HTTP Playground, an API request builder built into both the CLI and VibeUI. You can construct GET, POST, PUT, and DELETE requests with custom headers, body content, and query parameters, organize requests into collections, manage environment variables for API keys, and import or export cURL commands.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCody installed and configured
- A target API to test against (this demo uses `https://jsonplaceholder.typicode.com`)
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Step 1: Send a basic GET request

Open the REPL and use the HTTP command:

```bash
vibecli
> /http GET https://jsonplaceholder.typicode.com/posts/1
```

```
HTTP/1.1 200 OK
Content-Type: application/json; charset=utf-8
X-Response-Time: 142ms

{
  "userId": 1,
  "id": 1,
  "title": "sunt aut facere repellat provident occaecati excepturi...",
  "body": "quia et suscipit\nsuscipit recusandae..."
}
```

### Step 2: Send a POST request with headers and body

Create a new resource with custom headers and a JSON body:

```bash
> /http POST https://jsonplaceholder.typicode.com/posts \
    --header "Content-Type: application/json" \
    --header "X-Custom-Header: vibecody" \
    --body '{
      "title": "VibeCody Test",
      "body": "Testing the HTTP playground",
      "userId": 1
    }'
```

```
HTTP/1.1 201 Created
Content-Type: application/json; charset=utf-8
X-Response-Time: 203ms

{
  "title": "VibeCody Test",
  "body": "Testing the HTTP playground",
  "userId": 1,
  "id": 101
}
```

### Step 3: Use query parameters

Add query parameters to filter results:

```bash
> /http GET https://jsonplaceholder.typicode.com/posts \
    --query "userId=1" \
    --query "_limit=3"
```

```
HTTP/1.1 200 OK
Content-Type: application/json; charset=utf-8
X-Response-Time: 118ms

[
  { "userId": 1, "id": 1, "title": "sunt aut facere..." },
  { "userId": 1, "id": 2, "title": "qui est esse..." },
  { "userId": 1, "id": 3, "title": "ea molestias quasi..." }
]
```

### Step 4: Set up environment variables

Define environments to switch between staging and production API keys:

```bash
> /http env set dev \
    --var "BASE_URL=https://jsonplaceholder.typicode.com" \
    --var "API_KEY=dev-key-12345"

> /http env set prod \
    --var "BASE_URL=https://api.example.com" \
    --var "API_KEY=prod-key-secret"

> /http env use dev
```

Now reference variables in requests:

```bash
> /http GET {{BASE_URL}}/posts/1 \
    --header "Authorization: Bearer {{API_KEY}}"
```

### Step 5: Save requests to a collection

Organize related requests into named collections:

```bash
> /http collection create "JSONPlaceholder"

> /http collection add "JSONPlaceholder" \
    --name "Get Post" \
    --method GET \
    --url "{{BASE_URL}}/posts/1"

> /http collection add "JSONPlaceholder" \
    --name "Create Post" \
    --method POST \
    --url "{{BASE_URL}}/posts" \
    --header "Content-Type: application/json" \
    --body '{"title":"New Post","body":"Content","userId":1}'

> /http collection list
```

```
Collections:
  JSONPlaceholder (2 requests)
    1. Get Post        GET  {{BASE_URL}}/posts/1
    2. Create Post     POST {{BASE_URL}}/posts
```

Run a saved request by name:

```bash
> /http collection run "JSONPlaceholder" "Get Post"
```

### Step 6: View request history

Browse previously sent requests:

```bash
> /http history
```

```
Request History (last 10):
  #  Method  URL                                          Status  Time
  1  GET     .../posts/1                                  200     142ms
  2  POST    .../posts                                    201     203ms
  3  GET     .../posts?userId=1&_limit=3                  200     118ms
  4  GET     .../posts/1                                  200     135ms
```

Replay a previous request:

```bash
> /http history replay 2
```

### Step 7: Import and export cURL commands

Import an existing cURL command:

```bash
> /http import curl 'curl -X PUT https://jsonplaceholder.typicode.com/posts/1 \
    -H "Content-Type: application/json" \
    -d "{\"title\":\"Updated Title\"}"'
```

```
Imported as: PUT https://jsonplaceholder.typicode.com/posts/1
Headers: Content-Type: application/json
Body: {"title":"Updated Title"}

Execute now? [y/N]: y
HTTP/1.1 200 OK
...
```

Export any request as cURL:

```bash
> /http history export 2 --format curl
```

```
curl -X POST 'https://jsonplaceholder.typicode.com/posts' \
  -H 'Content-Type: application/json' \
  -H 'X-Custom-Header: vibecody' \
  -d '{"title":"VibeCody Test","body":"Testing the HTTP playground","userId":1}'
```

### Step 8: Response viewer modes

Switch between response display formats:

```bash
> /http GET https://example.com --response-format json
> /http GET https://example.com --response-format raw
> /http GET https://example.com --response-format headers-only
```

The JSON viewer pretty-prints and syntax-highlights the response. The raw mode shows the unprocessed body. Headers-only mode is useful for debugging CORS or caching headers.

### Step 9: Use the HTTP panel in VibeUI

Open VibeUI and navigate to the **HTTP** panel. The interface provides:

- **Request builder** at the top: method dropdown, URL bar, tabs for Headers, Body, Query Params, and Auth.
- **Send button** fires the request. The response appears below with tabs for Body (syntax-highlighted JSON/HTML/raw), Headers, and Timing.
- **Collections sidebar** on the left for organizing and replaying saved requests.
- **Environment switcher** in the toolbar to toggle between dev, staging, and production variables.
- **History** tab at the bottom shows all past requests with one-click replay.
- **Import/Export** buttons in the toolbar support cURL paste and export.

## Demo Recording

```json
{
  "meta": {
    "title": "HTTP Playground",
    "description": "Build API requests, manage collections, and import/export cURL commands.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/http GET https://jsonplaceholder.typicode.com/posts/1", "delay_ms": 3000 }
      ],
      "description": "Send a basic GET request"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/http POST https://jsonplaceholder.typicode.com/posts --header \"Content-Type: application/json\" --body '{\"title\":\"Test\",\"body\":\"Demo\",\"userId\":1}'", "delay_ms": 3000 }
      ],
      "description": "Send a POST request with headers and JSON body"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/http env set dev --var \"BASE_URL=https://jsonplaceholder.typicode.com\" --var \"API_KEY=test-key\"", "delay_ms": 1500 },
        { "input": "/http env use dev", "delay_ms": 1000 },
        { "input": "/http GET {{BASE_URL}}/posts/1 --header \"Authorization: Bearer {{API_KEY}}\"", "delay_ms": 3000 }
      ],
      "description": "Set up environment variables and use them in a request"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/http collection create \"Demo API\"", "delay_ms": 1000 },
        { "input": "/http collection add \"Demo API\" --name \"Get Post\" --method GET --url \"{{BASE_URL}}/posts/1\"", "delay_ms": 1500 },
        { "input": "/http collection list", "delay_ms": 1500 }
      ],
      "description": "Create a request collection and add requests"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/http history", "delay_ms": 2000 },
        { "input": "/http history export 1 --format curl", "delay_ms": 1500 }
      ],
      "description": "Browse request history and export as cURL"
    },
    {
      "id": 6,
      "action": "vibeui",
      "panel": "HTTP",
      "actions": ["build_request", "send", "view_response", "save_collection", "switch_env"],
      "description": "Use the HTTP panel in VibeUI to build requests and manage collections",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 28: GraphQL Explorer](../28-graphql/) -- Introspect schemas and build queries with autocomplete
- [Demo 29: Regex & Encoding Tools](../29-regex-encoding/) -- Pattern testing, JWT decoding, and data conversion
- [Demo 30: Notebook & Scripts](../30-notebook-scripts/) -- Interactive notebooks and AI-assisted scripting
