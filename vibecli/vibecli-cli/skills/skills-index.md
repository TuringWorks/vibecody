---
name: "Skills Index"
description: "Routing table for the 1143-skill catalog: how to pick a skill, the full category map with counts, and task-to-category shortcuts. Load this FIRST before calling list_skills, so the catalog is never listed unfiltered."
category: shared
triggers: ["skills index", "which skill", "what skills are available", "find a skill", "list skills", "skill catalog", "choose a skill", "skill routing"]
tools_allowed: ["read_file"]
---

# Skills Index — start here

**1143 skills** are available. Never list them all: an unfiltered listing is >150k tokens and will not fit in any model's context.

## How to select a skill (any model, any provider)

1. Pick the one or two **categories** below that match the task.
2. Call `list_skills` with `category` set — and add a `query` if the category has more than ~30 entries.
3. Read the returned `description` fields. Each says what the skill covers and when to use it.
4. Call `get_skill` with the chosen `name` to load the full body, then follow it.
5. If nothing matches, proceed without a skill. Do not force an unrelated one.

Rules of thumb:

- One or two skills per task. Loading more crowds out the actual work.
- Prefer the most specific match (`rust-axum` over `rust-traits-generics` over `agent-development`).
- Skills named `shared-*` hold text referenced by other skills — load one only when a skill you already loaded points at it.
- Skills named `*-sector-operations` are the entry point for a whole industry; role skills inside that industry are named `<sector>-<role>`.

## Category map

### Languages & frameworks — 111 skills
Categories: `assembly` (1), `c` (1), `clojure` (1), `cobol` (1), `cpp` (3), `crystal` (1), `csharp` (4), `dart` (2), `delphi` (1), `elixir` (2), `erlang` (1), `fortran` (1), `frontend` (6), `fsharp` (1), `gaming` (2), `go` (7), `groovy` (1), `haskell` (1), `java` (12), `javascript` (4), `julia` (2), `kotlin` (4), `legacy` (3), `lua` (1), `mobile` (4), `nim` (1), `objective-c` (1), `ocaml` (1), `perl` (1), `php` (3), `python` (10), `r-lang` (1), `ruby` (2), `rust` (10), `scala` (1), `swift` (4), `typescript` (6), `vb` (2), `zig` (1)

### Architecture & design — 42 skills
Categories: `api` (8), `api-design` (1), `architecture` (17), `design` (10), `protocols` (6)

### Data & databases — 34 skills
Categories: `data` (6), `data-analytics` (2), `data-engineering` (4), `database` (22)

### Cloud, infra & DevOps — 97 skills
Categories: `cloud-aws` (15), `cloud-azure` (17), `cloud-do` (1), `cloud-firebase` (1), `cloud-gcp` (13), `cloud-ibm` (1), `cloud-netlify` (1), `cloud-oci` (1), `cloud-paas` (1), `cloud-supabase` (1), `cloud-vercel` (1), `devops` (36), `edge` (1), `observability` (6), `sre` (1)

### Security & compliance — 51 skills
Categories: `compliance` (3), `safety-critical` (7), `security` (41)

### Testing & quality — 63 skills
Categories: `code-intelligence` (14), `developer-experience` (8), `performance` (14), `review` (10), `testing` (17)

### Agents, AI & MCP — 85 skills
Categories: `agent` (45), `ai` (39), `ai-models` (1)

### Workflow & session tooling — 69 skills
Categories: `automation` (6), `documentation` (10), `productivity` (14), `session` (9), `workflow` (30)

### Business & management — 146 skills
Categories: `economics` (8), `finance` (22), `hr` (15), `legal` (4), `management` (15), `marketing` (7), `operations` (6), `people-skills` (9), `personal-development` (7), `public-finance` (11), `sales` (2), `strategy` (40)

### Industry & sector operations — 285 skills
Categories: `agriculture` (24), `construction` (15), `defense` (12), `education` (13), `energy` (10), `governance` (4), `government` (10), `healthcare` (15), `household` (13), `identity` (9), `industry` (34), `logistics` (18), `manufacturing` (10), `media` (9), `mining` (10), `public-safety` (11), `resilience` (9), `retail` (16), `science` (12), `sustainability` (9), `telecom` (13), `water` (9)

### Robotics & embodied — 55 skills
Categories: `robotics` (55)

### Shared / cross-cutting — 19 skills
Categories: `archetypes` (15), `shared` (4)

### Other — 86 skills
Categories: `accessibility` (1), `aerospace` (2), `android` (1), `ballerina` (1), `biotech` (1), `blockchain` (12), `carbon` (1), `creative` (5), `d` (1), `devex` (1), `educational` (2), `embedded` (2), `engineering` (6), `enterprise` (5), `erp` (2), `fintech` (2), `hospitality` (1), `industrial` (1), `infrastructure` (3), `insurance` (2), `iot` (1), `lisp` (1), `matlab` (1), `odoo` (1), `powershell` (1), `prolog` (1), `quantum` (3), `real-estate` (1), `salesforce` (2), `sas` (1), `scientific` (5), `smart-home` (1), `sql` (3), `terminal` (5), `tizen` (1), `v` (1), `web3` (1), `writing` (3), `xr` (1)

## Common task → category shortcuts

| If the task is about… | Try `category` |
|---|---|
| writing or fixing application code | the language category (`python`, `rust`, `typescript`, `go`, `java`, …) |
| designing an API or service boundary | `api`, `architecture` |
| schema, query, migration work | `database` |
| deploying, CI, containers, k8s | `devops`, `cloud-aws`, `cloud-azure`, `cloud-gcp` |
| vulnerabilities, secrets, auth | `security` |
| writing or fixing tests | `testing` |
| reviewing a diff or PR | `review` |
| slowness, profiling, caching | `performance` |
| building an agent, tool-calling, MCP | `agent`, `ai`, `protocols` |
| a multi-step process (TDD, incident, release) | `workflow` |
| a specific industry's domain rules | the sector category, then `<sector>-sector-operations` |
