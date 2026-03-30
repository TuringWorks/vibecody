---
layout: page
title: "Demo 32: Legacy Migration"
permalink: /demos/32-legacy-migration/
---


## Overview

The Legacy Migration engine translates codebases from 18 source languages to 10 target languages using AI-powered translation rules, service boundary detection, and 6 migration strategies. With 31 supported language pairs and configurable strategies like Strangler Fig and Parallel Run, VibeCody handles everything from mainframe COBOL to desktop VB6 conversions.

## Prerequisites

- VibeCLI installed and on your PATH
- At least one AI provider configured
- Access to the source codebase you want to migrate
- For VibeUI: the desktop app running with the **Batch Builder** panel visible (Migration tab)

## Supported Languages

### 18 Source Languages

| Category        | Languages                                                |
|-----------------|----------------------------------------------------------|
| Mainframe       | COBOL, RPG, PL/I, Natural, Rexx                         |
| Scientific      | Fortran, Ada                                             |
| Desktop/RAD     | VB6, Delphi, PowerBuilder, Clipper                       |
| Scripting       | Perl, TCL, Smalltalk                                     |
| Legacy Business | Pascal, Progress, ABAP, MUMPS                            |

### 10 Target Languages

Rust, Go, Python, TypeScript, Java, C#, Kotlin, Swift, Ruby, Elixir

### 31 Supported Translation Pairs

Not every source-target combination is supported. VibeCody validates the pair before starting a migration. Common pairs include COBOL-to-Java, VB6-to-C#, Fortran-to-Rust, Perl-to-Python, and ABAP-to-TypeScript.

## 6 Migration Strategies

| Strategy               | Description                                                              | Best For                          |
|------------------------|--------------------------------------------------------------------------|-----------------------------------|
| **BigBang**            | Translate everything at once, switch over in a single deployment         | Small codebases, tight timelines  |
| **StranglerFig**       | Incrementally replace modules while the legacy system remains live       | Large systems, low risk tolerance  |
| **BranchByAbstraction**| Introduce abstraction layers, swap implementations behind interfaces     | Tightly coupled code              |
| **ParallelRun**        | Run old and new systems side by side, compare outputs                    | Mission-critical correctness       |
| **DatabaseFirst**      | Migrate the data layer first, then services on top                       | Data-centric applications          |
| **APIFirst**           | Wrap legacy code in modern APIs, migrate internals later                 | Systems with many consumers        |

## Step-by-Step Walkthrough

### 1. Analyze the Source Codebase

Scan the legacy codebase to detect languages, module boundaries, and dependencies.

**CLI:**

```bash
vibecli --legacymigrate analyze --source ./legacy-cobol-app
```

Example output:

```
Source Analysis: legacy-cobol-app
Language: COBOL (94%), JCL (6%)
Files: 342
Lines: 287,000
Modules detected: 18
Service boundaries: 5
External dependencies: DB2, CICS, MQ
Recommended strategy: StranglerFig
Recommended target: Java
```

**VibeUI:**

In the **Batch Builder** panel, switch to the **Migration** tab and click **Analyze Source**. Point it to your legacy project directory.

### 2. Configure the Migration

Choose the target language and strategy. Override the recommendations if needed.

**CLI:**

```bash
vibecli --legacymigrate configure \
  --source ./legacy-cobol-app \
  --target java \
  --strategy strangler-fig \
  --output ./migrated-java-app
```

**VibeUI:**

In the Migration tab, select the target language from the dropdown, choose a strategy, and set the output directory.

### 3. Review Translation Rules

Before running the migration, inspect the translation rules that will be applied. Rules map language constructs, data types, and patterns from source to target.

**CLI:**

```bash
vibecli --legacymigrate rules --pair cobol-java
```

Example output:

```
Translation Rules: COBOL → Java
  PIC X(n)       → String (length n)
  PIC 9(n)       → int / long
  PIC 9(n)V9(m)  → BigDecimal
  PERFORM THRU   → method call sequence
  COPY member    → import + class inclusion
  EVALUATE/WHEN  → switch/case
  88-level       → enum / boolean
  REDEFINES      → union type / inheritance
  ...
  Total rules: 47
```

**VibeUI:**

Click **View Rules** to see the full mapping table for the selected language pair.

### 4. Detect Service Boundaries

The engine identifies logical service boundaries in the legacy code to guide modular migration.

**CLI:**

```bash
vibecli --legacymigrate boundaries --source ./legacy-cobol-app
```

Example output:

```
Service Boundaries Detected: 5
  1. CustomerManagement (42 programs, 63K lines)
  2. OrderProcessing (78 programs, 89K lines)
  3. InventoryControl (51 programs, 44K lines)
  4. BillingEngine (38 programs, 51K lines)
  5. Reporting (33 programs, 40K lines)
```

### 5. Start the Migration

Launch the migration. For Strangler Fig, this creates the first batch of migrated modules.

**CLI:**

```bash
vibecli --legacymigrate start \
  --source ./legacy-cobol-app \
  --target java \
  --strategy strangler-fig \
  --boundary CustomerManagement
```

**VibeUI:**

Click **Start Migration** after configuring all options. Progress appears in the Monitor tab.

### 6. Validate the Output

Run the built-in validation to compare behavior between the legacy and migrated code.

**CLI:**

```bash
vibecli --legacymigrate validate --output ./migrated-java-app
```

Example output:

```
Validation Results:
  Modules migrated: 42
  Compilation: PASS
  Unit tests generated: 156 (all passing)
  Behavioral comparison: 98.7% match
  Warnings: 2 (manual review suggested for CICS transaction mapping)
```

### 7. Iterate on Remaining Boundaries

For incremental strategies, repeat steps 5-6 for each service boundary until the full codebase is migrated.

**CLI:**

```bash
vibecli --legacymigrate start --boundary OrderProcessing
vibecli --legacymigrate start --boundary InventoryControl
# ... continue for each boundary
```

## Demo Recording JSON

```json
{
  "demo_id": "32-legacy-migration",
  "title": "Legacy Migration",
  "version": "1.0.0",
  "steps": [
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate analyze --source ./legacy-cobol-app",
      "description": "Analyze the legacy COBOL codebase"
    },
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate configure --source ./legacy-cobol-app --target java --strategy strangler-fig --output ./migrated-java-app",
      "description": "Configure migration target and strategy"
    },
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate rules --pair cobol-java",
      "description": "Review the 47 COBOL-to-Java translation rules"
    },
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate boundaries --source ./legacy-cobol-app",
      "description": "Detect service boundaries in the legacy code"
    },
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate start --source ./legacy-cobol-app --target java --strategy strangler-fig --boundary CustomerManagement",
      "description": "Migrate the first service boundary"
    },
    {
      "action": "cli_command",
      "command": "vibecli --legacymigrate validate --output ./migrated-java-app",
      "description": "Validate migrated code against legacy behavior"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "Migration",
      "description": "Configure and launch migration from the GUI"
    },
    {
      "action": "vibeui_interaction",
      "panel": "BatchBuilder",
      "tab": "Monitor",
      "description": "Watch migration progress per service boundary"
    }
  ]
}
```

## What's Next

- [Demo 31: Batch Builder](../31-batch-builder/) -- Use Batch Builder for greenfield code generation
- [Demo 33: App Builder](../33-app-builder/) -- Scaffold new applications from natural language
- [Demo 35: Compliance & Audit](../35-compliance/) -- Ensure migrated code meets compliance requirements
