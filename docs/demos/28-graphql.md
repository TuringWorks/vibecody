---
layout: page
title: "Demo 28: GraphQL Explorer"
permalink: /demos/graphql/
nav_order: 28
parent: Demos
---


## Overview

This demo covers VibeCody's GraphQL Explorer, which lets you introspect schemas, build queries with autocomplete, edit variables, browse schema documentation, and manage query history. Available in both the CLI and VibeUI's dedicated GraphQL panel.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCody installed and configured
- A GraphQL endpoint to test against (this demo uses the public Star Wars API at `https://swapi-graphql.netlify.app/.netlify/functions/index`)
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Step 1: Connect to a GraphQL endpoint

Set up a connection in the REPL:

```bash
vibecli
> /graphql connect https://swapi-graphql.netlify.app/.netlify/functions/index
```

```
Connected to GraphQL endpoint
Introspecting schema...
  Types:       47
  Queries:     5
  Mutations:   0
  Subscriptions: 0
Schema cached at ~/.vibecli/graphql/swapi-schema.json
```

VibeCody automatically runs an introspection query on connection and caches the schema locally for autocomplete and documentation.

### Step 2: Introspect the schema

Explore available types and fields:

```bash
> /graphql schema types
```

```
Root Query Fields:
  allFilms(after, first, before, last) → FilmsConnection
  allPeople(after, first, before, last) → PeopleConnection
  allPlanets(after, first, before, last) → PlanetsConnection
  allSpecies(after, first, before, last) → SpeciesConnection
  allStarships(after, first, before, last) → StarshipsConnection

Object Types (top 10):
  Film          { title, episodeID, openingCrawl, director, ... }
  Person        { name, birthYear, eyeColor, gender, ... }
  Planet        { name, diameter, rotationPeriod, ... }
  Starship      { name, model, manufacturer, ... }
  Species       { name, classification, designation, ... }
```

Drill into a specific type:

```bash
> /graphql schema type Person
```

```
Type: Person
  name: String
  birthYear: String
  eyeColor: String
  gender: String
  hairColor: String
  height: Int
  mass: Float
  skinColor: String
  homeworld: Planet
  filmConnection: PersonFilmsConnection
  starshipConnection: PersonStarshipsConnection
  vehicleConnection: PersonVehiclesConnection
```

### Step 3: Run a query

Execute a GraphQL query directly:

```bash
> /graphql query '{
    allFilms {
      films {
        title
        director
        releaseDate
      }
    }
  }'
```

```
{
  "data": {
    "allFilms": {
      "films": [
        { "title": "A New Hope", "director": "George Lucas", "releaseDate": "1977-05-25" },
        { "title": "The Empire Strikes Back", "director": "Irvin Kershner", "releaseDate": "1980-05-17" },
        { "title": "Return of the Jedi", "director": "Richard Marquand", "releaseDate": "1983-05-25" },
        { "title": "The Phantom Menace", "director": "George Lucas", "releaseDate": "1999-05-19" },
        { "title": "Attack of the Clones", "director": "George Lucas", "releaseDate": "2002-05-16" },
        { "title": "Revenge of the Sith", "director": "George Lucas", "releaseDate": "2005-05-19" }
      ]
    }
  }
}
```

### Step 4: Use variables

Pass query variables alongside the query:

```bash
> /graphql query \
    --query 'query GetPerson($id: ID!) { person(id: $id) { name birthYear homeworld { name } } }' \
    --variables '{"id": "cGVvcGxlOjE="}'
```

```
{
  "data": {
    "person": {
      "name": "Luke Skywalker",
      "birthYear": "19BBY",
      "homeworld": {
        "name": "Tatooine"
      }
    }
  }
}
```

### Step 5: Use the query builder with autocomplete

The REPL provides tab completion when building queries:

```bash
> /graphql build
```

```
GraphQL Query Builder (Tab for autocomplete, Ctrl+D to execute)
  query {
    allPe[TAB]
    → allPeople
    allPeople {
      people {
        na[TAB]
        → name
        name
        bi[TAB]
        → birthYear
        birthYear
      }
    }
  }

Execute query? [Y/n]: y
```

The builder reads from the cached schema to suggest field names, arguments, and types as you type.

### Step 6: Browse query history

View and replay previous queries:

```bash
> /graphql history
```

```
Query History:
  #  Endpoint          Query (truncated)                     Time     Status
  1  swapi-graphql     { allFilms { films { title ... } } }  243ms    200
  2  swapi-graphql     query GetPerson($id: ID!) { ... }     189ms    200
  3  swapi-graphql     { allPeople { people { name ... } } } 201ms    200
```

Replay a previous query:

```bash
> /graphql history replay 1
```

### Step 7: Browse schema documentation

View auto-generated documentation for any type:

```bash
> /graphql docs Film
```

```
Film
  A single film.

Fields:
  title: String!
    The title of this film.
  episodeID: Int
    The episode number of this film.
  openingCrawl: String
    The opening paragraphs at the beginning of this film.
  director: String
    The name of the director of this film.
  producers: [String]
    The name(s) of the producer(s) of this film.
  releaseDate: String
    The ISO 8601 date format of film release.

Connections:
  speciesConnection: FilmSpeciesConnection
  starshipConnection: FilmStarshipsConnection
  vehicleConnection: FilmVehiclesConnection
  characterConnection: FilmCharactersConnection
  planetConnection: FilmPlanetsConnection
```

### Step 8: Add custom headers

Authenticate with APIs that require tokens:

```bash
> /graphql connect https://api.github.com/graphql \
    --header "Authorization: Bearer ghp_your_token_here"
```

```bash
> /graphql query '{ viewer { login name repositories(first: 5) { nodes { name } } } }'
```

### Step 9: Use the GraphQL panel in VibeUI

Open VibeUI and navigate to the **GraphQL** panel. The interface provides:

- **Endpoint bar** at the top with a connect button and header editor.
- **Query editor** on the left with syntax highlighting, bracket matching, and autocomplete driven by the introspected schema.
- **Variables editor** below the query editor for passing JSON variables.
- **Response viewer** on the right showing formatted JSON results with expandable nodes.
- **Schema browser** as a sidebar tab listing all types, fields, and documentation. Click any type name to jump to its docs.
- **History** tab at the bottom showing all past queries with one-click replay and a diff view for comparing responses across runs.

## Demo Recording

```json
{
  "meta": {
    "title": "GraphQL Explorer",
    "description": "Introspect schemas, build queries, and browse documentation.",
    "duration_seconds": 240,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/graphql connect https://swapi-graphql.netlify.app/.netlify/functions/index", "delay_ms": 4000 }
      ],
      "description": "Connect to a GraphQL endpoint and introspect the schema"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/graphql schema types", "delay_ms": 2000 },
        { "input": "/graphql schema type Person", "delay_ms": 2000 }
      ],
      "description": "Explore schema types and field definitions"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/graphql query '{ allFilms { films { title director releaseDate } } }'", "delay_ms": 3000 }
      ],
      "description": "Execute a GraphQL query"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/graphql query --query 'query GetPerson($id: ID!) { person(id: $id) { name birthYear } }' --variables '{\"id\": \"cGVvcGxlOjE=\"}'", "delay_ms": 3000 }
      ],
      "description": "Run a query with variables"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/graphql docs Film", "delay_ms": 2000 }
      ],
      "description": "Browse auto-generated schema documentation"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/graphql history", "delay_ms": 1500 },
        { "input": "/graphql history replay 1", "delay_ms": 3000 }
      ],
      "description": "View and replay query history"
    },
    {
      "id": 7,
      "action": "vibeui",
      "panel": "GraphQL",
      "actions": ["connect", "introspect", "build_query", "edit_variables", "view_response", "browse_docs"],
      "description": "Use the GraphQL panel in VibeUI with autocomplete and schema browser",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 29: Regex & Encoding Tools](../regex-encoding/) -- Pattern testing, JWT decoding, and data conversion
- [Demo 30: Notebook & Scripts](../notebook-scripts/) -- Interactive notebooks and AI-assisted scripting
- [Demo 25: SWE-bench Benchmarking](../swe-bench/) -- Benchmark your AI provider with SWE-bench
