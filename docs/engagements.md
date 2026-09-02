# Engagements — the four-phase delivery spine

VibeCody ships the tools that produce almost every artifact a client engagement
promises: architecture specs, threat models, CI gates, SLO dashboards, cost
models, runbooks. What it did not have was the object that says *which of them
this engagement has actually produced*, and *whether the phase they belong to
may be closed*.

That is what an **engagement** is. One record, four phases, a list of promised
deliverables, and a set of gates that decide whether a phase can be exited.

- **Panel:** Delivery → Engagement (VibeCoder)
- **Routes:** `/engagements/*` on the VibeCLI daemon (bearer auth required)
- **Store:** `~/.vibecli/engagements.db`
- **Source:** `vibecli/vibecli-cli/src/engagement.rs`,
  `vibecli/vibecli-cli/src/engagement_routes.rs`

---

## The four phases

| # | Phase | Cadence | Purpose |
|---|---|---|---|
| 01 | Discover & Assess | *not published* | Map the current state, interview the people who operate it, and separate the problems worth solving from the noise. No engagement starts with a solution already chosen. |
| 02 | Prove | 4–8 weeks | A narrow pilot on your data, on your infrastructure, judged against success criteria agreed up front. If the approach is wrong, this is where we find out cheaply. |
| 03 | Build & Harden | Scope-dependent | Production delivery alongside your engineers — infrastructure as code, CI/CD, test coverage, observability, and a security review that happens during the build rather than after it. |
| 04 | Operate & Transfer | Ongoing or fixed | Managed operation for as long as it is useful, and a deliberate handover after that. The goal is for your team to own the system, not to keep us on the invoice. |

**Discover has no cadence, and the code says so.** `Phase::cadence()` returns
`None` for it, the API returns `null`, and the panel renders a dash. The
engagement model publishes a duration for the other three phases and not for
this one; inventing "2–4 weeks" would put a commitment in front of a client that
nobody made. See [Modelling Honesty](../AGENTS.md#modelling-honesty--a-model-that-cannot-be-wrong-is-not-a-model).

---

## Deliverables

Creating an engagement seeds it with every deliverable the engagement model
promises — 30 of them across the four phases — each carrying a **tool hint**:
the VibeCody panel that produces it.

The tool hint is the point. VibeCoder has more than three hundred panels, and a
list of promised artifacts is not useful if the operator has to guess which
panel produces "threat model". In the panel each deliverable's tool hint is a
button that opens the owning tab.

A representative slice:

| Phase | Deliverable | Produced with |
|---|---|---|
| Discover | Current-state architecture map | `ArchitectureSpecPanel` |
| Discover | Risk register | `SecurityPosturePanel` |
| Discover | Prioritized roadmap with effort and sequencing | `PlanDocumentPanel` |
| Prove | Working pilot deployed in your environment | `DeployPanel` |
| Prove | Cost model at target production volume | `CostPanel` |
| Prove | Go / no-go recommendation with alternatives | `CounselPanel` |
| Build | Infrastructure as code | `K8sPanel` |
| Build | Alerting and SLO definitions | `HealthMonitorPanel` |
| Build | Threat model | `SecurityReviewPanel` |
| Operate | Runbooks | `PlanDocumentPanel` |
| Operate | Escalation paths | `TeamGovernancePanel` |
| Operate | Documented exit plan | `CompanyPortabilityPanel` |

The full list is `TEMPLATE` in `engagement.rs`, and
`GET /engagements/template` returns it without needing an engagement to exist.

### Deliverable status

`not_started` → `in_progress` → `ready` → `accepted`, plus `waived`.

Only **`accepted`** closes a deliverable — `ready` means we produced it and
reviewed it ourselves, which is not the same as the client signing it off.
**`waived`** means agreed out of scope: it is excluded from the completion
denominator and reported in its own section of the handover pack, never folded
into "done".

### Evidence

A deliverable marked `ready` or `accepted` with **zero attached evidence** is
flagged, counted in `claimed_without_evidence`, and blocks the phase. A
deliverable claimed done with nothing behind it is the most common way an
engagement report lies.

Evidence kinds: `file`, `url`, `run` (an eval run id, job id, or workflow run),
`metric`, `note`.

---

## Gates

A gate is a criterion, a **measurement procedure**, an observation, and a
verdict. The measurement procedure is mandatory — `POST /engagements/{id}/gates`
rejects a gate without one, because a criterion with no stated way of judging it
is settled at review time by whoever argues hardest.

### The five verdicts

| Verdict | Meaning | Satisfies the gate? |
|---|---|---|
| `not_measured` | Nobody has arranged to judge this. | No |
| `pending` | A judgement is scheduled and has not happened. | No |
| `pass` | Judged, and it passed. | Yes |
| `fail` | Judged, and it failed. | No |
| `waived` | Agreed not to apply. | Yes |

`not_measured` and `fail` are kept strictly apart, exactly as in
[`vibecli --eval`](../evals/README.md). Collapsing them into "fail" produces a
report that says the work is broken when the truth is that nobody looked;
collapsing them into "pass" ships an unmeasured claim to a client.

A gate is seeded `not_measured` on creation. A fresh engagement reporting its
gates as passing would assert a fact about the world nobody has checked.

**A passing gate must record what was observed.** `POST .../judge` returns 400
for `verdict: "pass"` with an empty `observed`. Returning a gate to
`not_measured` clears its decider, decision time, and observation — leaving them
would assert that somebody judged it.

Default gates are seeded per phase (`GATE_TEMPLATE`), including:

- *Criteria agreed before the pilot ran* — compare each gate's creation
  timestamp against the pilot's start. A criterion created after the start is
  evidence of the outcome, not a test of it.
- *Pipeline blocks a bad change* — open a PR that deliberately violates each
  gate and record that the pipeline stopped it.
- *Every alert has a runbook* — enumerate configured alerts and join against
  runbooks; any unmatched alert fails.
- *Client engineers have operated the system unaided* — a real change and a real
  incident handled with no vendor participation.

---

## Phase exit

A phase can be closed when **every** in-scope deliverable is `accepted` with
evidence **and** every gate is `pass` or `waived`. Anything else appears in
`blockers`, each with its own kind so the reason is not flattened:

`deliverable_outstanding` · `deliverable_without_evidence` · `gate_failed` ·
`gate_pending` · `gate_not_measured`

`POST /engagements/{id}/advance` refuses and returns the blockers. Passing
`{"force": true}` advances anyway — sometimes the client decides to proceed with
a gate failing — but the response carries `forced: true` and the blockers
survive in the record. An override is visible as an override.

### Completion is `n/a`, never `0%`, when there is nothing in scope

`completion` is `accepted / (total − waived)`. When the denominator is zero it
is `null`, and every renderer shows `n/a`. `0%` reads as "measured, and bad";
`n/a` is the truth.

---

## HTTP API

All routes are authenticated. Use `daemonFetch()` from a VibeCoder panel, the
SDK's `authedFetch()` elsewhere, or `Authorization: Bearer $(cat ~/.vibecli/daemon_token)`
by hand. See [Calling a daemon route](../AGENTS.md#calling-a-daemon-route-from-any-client).

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/engagements/template` | The engagement model: phases, deliverable templates, gate templates. Static. |
| `GET` | `/engagements` | List engagements. |
| `POST` | `/engagements` | Create one, seeded with the full template. |
| `GET` | `/engagements/{id}` | The engagement plus its readiness report for all four phases. |
| `PATCH` | `/engagements/{id}` | Set `status` and/or `phase`. |
| `DELETE` | `/engagements/{id}` | Remove it (cascades to deliverables, evidence, gates). |
| `POST` | `/engagements/{id}/seed` | Top up with template rows added since creation. Never disturbs an accepted deliverable. |
| `GET` | `/engagements/{id}/deliverables?phase=` | List, optionally filtered by phase. |
| `POST` | `/engagements/{id}/deliverables` | Add a deliverable outside the template. |
| `PATCH` | `/engagements/{id}/deliverables/{did}` | Patch `status`, `owner`, `notes`. Omitted fields are untouched. |
| `GET`/`POST` | `/engagements/{id}/deliverables/{did}/evidence` | List / attach evidence. |
| `DELETE` | `/engagements/{id}/evidence/{eid}` | Remove one evidence item. |
| `GET`/`POST` | `/engagements/{id}/gates?phase=` | List / add gates. |
| `POST` | `/engagements/{id}/gates/{gid}/judge` | Record a verdict. |
| `DELETE` | `/engagements/{id}/gates/{gid}` | Remove a gate. |
| `POST` | `/engagements/{id}/scan` | Propose workspace files as evidence (`{"path": …, "attach": false}`). Never changes a status. |
| `POST` | `/engagements/{id}/advance` | Close the current phase; `{"force": true}` overrides. |
| `GET` | `/engagements/{id}/report.md` | Status report, `text/markdown`. |
| `GET` | `/engagements/{id}/handover.md` | Handover pack, `text/markdown`. |

An unrecognised `?phase=` value is a **400**, not a silent "all phases" — a typo
that quietly widens a query is how a client ends up reading another phase's
numbers.

### Example

```bash
TOKEN=$(cat ~/.vibecli/daemon_token)
API=http://localhost:7878

# Create — seeded with 4 phases, every promised deliverable, every gate.
ID=$(curl -s -X POST $API/engagements \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"name":"Acme platform","client":"Acme Corp"}' | jq -r .engagement.id)

# Where does the engagement actually stand?
curl -s $API/engagements/$ID -H "Authorization: Bearer $TOKEN" \
  | jq '.report.phases[] | {title, completion, blockers: (.blockers|length)}'

# The client sees this.
curl -s $API/engagements/$ID/report.md -H "Authorization: Bearer $TOKEN"
```

---

## Workspace scan — finding evidence that already exists

`POST /engagements/{id}/scan`, `vibecli --engagement scan`, or **Scan
workspace** in the panel walks the workspace and proposes which existing files
could back which deliverable — architecture documents, ADRs, Terraform, CI
workflows, coverage reports, SLO rules, threat models, runbooks.

**What a scan is allowed to conclude is deliberately small.** A file existing is
not a deliverable: `docs/threat-model.md` may be a finished threat model, an
empty heading, or a template somebody copied in three years ago. So the scan:

- proposes **candidates**, never findings;
- **never changes a deliverable's status** — with `attach: true` it records
  evidence and nothing else;
- labels what it attaches `detected: <rule>`, so a reviewer can tell a machine's
  guess from a human's assertion;
- **has no rule at all** for deliverables it cannot honestly detect. No filename
  is evidence that a production system works, that you paired with the client's
  engineers, or that anyone signed an SLA. Those keys are listed in the
  response's `undetectable` array, so an empty result is never mistaken for
  "there is nothing there".

Caps are reported rather than silent — a scan that stopped at its depth, file,
or candidate limit sets `truncated` and says so in `notes`. A truncated list
that looked complete would read as "there is nothing else here".

With no `path` and no workspace bound to the engagement, the route is a **400**.
Falling back to the daemon's working directory would scan an unrelated tree and
label the result as this client's evidence.

```bash
vibecli --engagement scan <id> --path /srv/acme            # propose only
vibecli --engagement scan <id> --path /srv/acme --attach   # record as evidence
```

---

## CLI — `vibecli --engagement`

The panel is the comfortable way to run an engagement; the CLI is the way to
*check* one from a pipeline.

```bash
vibecli --engagement list
vibecli --engagement new "Acme platform" --client "Acme Corp"
vibecli --engagement show <id>
vibecli --engagement deliverables <id> --phase build
vibecli --engagement set <id> threat-model accepted
vibecli --engagement evidence <id> threat-model docs/threat-model.md --kind file
vibecli --engagement gates <id> --phase build
vibecli --engagement judge <id> <gate-id> pass --observed "clean provision from scratch"
vibecli --engagement advance <id>
vibecli --engagement report <id> > status.md
vibecli --engagement handover <id> > handover.md
```

### The pipeline check

```bash
vibecli --engagement gate <id> --phase build
```

Exit codes are part of the contract:

| Code | Meaning |
|---|---|
| `0` | The phase is clean: every in-scope deliverable accepted with evidence, every gate `pass` or `waived`. |
| `1` | The phase has blockers, or the operation failed. |
| `2` | Usage error — unknown subcommand, bad phase, bad verdict. |

`1` and `2` are deliberately different. A pipeline that reads "you typed the
phase name wrong" as "the engagement is not ready" fails for a reason nobody can
act on.

When gates are unmeasured the check says so separately, rather than reporting
them as failures:

```
Build & Harden: 7 blocker(s)
  - 'Infrastructure as code' is not_started, not accepted.
  ...
5 gate(s) are unmeasured. That is not a failure — it is an absence of
measurement, and it blocks for that reason.
```

`--engagement judge … pass` refuses without `--observed`, the same rule the HTTP
route enforces. `--engagement set` warns when a deliverable becomes `ready` or
`accepted` with nothing attached.

`VIBECLI_ENGAGEMENT_DB` overrides the store path — useful for pointing a team at
one shared engagement database, and for CI runs that should not touch a
developer's `~/.vibecli`.

---

## Exports

- **`report.md`** — the status report. Phase summary table, deliverables with
  evidence counts and their producing panel, every gate with its criterion,
  measurement procedure and observation, and the blockers per phase. A gate
  with no observation renders *not recorded*, not a blank or a zero.
- **`handover.md`** — the handover pack. How to operate the system, everything
  accepted, and — the section that makes it worth reading — **what is not
  delivered**, plus what was waived and why.

---

## Design notes

The rules this subsystem enforces are the
[evaluation harness's](../evals/README.md) rules applied to delivery, for the
same reason: an engagement report is a measurement, and it fails the same ways.

1. **Verdicts stay disjoint.** Five gate verdicts, five deliverable statuses,
   and no code path that maps one onto another for convenience.
2. **Absent stays absent.** No `unwrap_or_else(Utc::now)` on a timestamp, no
   default observation, no invented cadence. `INSERT OR REPLACE` is not used
   anywhere — patches are `DO UPDATE SET` over named columns, so a partial
   update cannot NULL a column the caller never mentioned.
3. **Unmeasured blocks.** A gate nobody judged does not let a phase close, and
   appears as its own blocker kind.
4. **A percentage over an empty denominator is `n/a`.**
5. **Re-seeding is additive.** `seed_template` inserts only what is missing, so
   a template that grows later reaches existing engagements without discarding
   a client's acceptance record.

Tests live alongside the module (`cargo test -p vibecli --lib engagement`) and
assert each of these directly — including that every phase has deliverables and
gates, that every gate template names a measurement procedure, and that every
deliverable names a producing surface.
