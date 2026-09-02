---
name: "Engineering Practices Program"
description: "Engineering Practices Program: running an enterprise-wide engineering standards and maturity program — writing standards that get adopted, a maturity model that means something, the practice council that governs it, and the adoption data that decides whether a standard survives. Use when the task involves engineering standards, community of practice, maturity model, practice governance, technology practices, engineering guidelines, or standards adoption across many teams."
category: devex
triggers: ["engineering standards", "practices program", "community of practice", "maturity model", "engineering guidelines", "standards adoption", "practice council", "technology practices", "engineering governance"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Engineering practices program

A practices program in a 1,000-engineer organisation succeeds or fails on one
question: **is the compliant path faster than the non-compliant path?** Every
mechanic below serves that question.

## Measure what exists before writing what should

```
vibecli --devex practices --path <repo>          # per-practice, per-signal
vibecli --devex practices --path <repo> --json   # for rolling up across repos
```

The scan reports a **detected** level, capped at 3 ("defined"). It will not
report level 4. A file proves a practice is *present*; it cannot prove the
practice is *followed*, *reviewed*, or *improving* — which is exactly what
separates the top level of every maturity model worth using from the one below
it. That last level is attested by people. Do not build a report that fills it
in automatically; the moment a maturity score can be earned with `touch`, the
whole model is decorative.

## The maturity model

Five levels. The names matter less than the fact that each one has an
observable test.

| Level | Name | Test | Who can assert it |
|---|---|---|---|
| 0 | Absent | No artifact exists | Scan |
| 1 | Initial | An artifact exists somewhere | Scan |
| 2 | Managed | It is in the repo, versioned, and named in the build | Scan |
| 3 | Defined | It is the documented standard and the golden path implements it | Scan (presence) + owner (intent) |
| 4 | Optimizing | Adoption is measured, deviations are reviewed, the standard has been revised in response | **People only** |

Publish the tests, not just the level names. A maturity model whose levels are
adjectives gets argued about at every review; one whose levels are tests gets
checked.

## Writing a standard that gets adopted

A standard is a product with users. It needs the same things.

1. **Name the failure it prevents, with a real incident.** A standard that
   cannot cite the outcome it exists to avoid will be treated as taste.
2. **State the level of obligation.** MUST / SHOULD / MAY, used precisely. Most
   standards documents fail here: everything reads as mandatory, so nothing is.
3. **Ship the paved road in the same change.** The template, the CI check, the
   library — whatever makes compliance the default. A standard published
   without its paved road is a tax announcement.
4. **Give it an owner and a review date.** Unowned standards accumulate; the
   corpus becomes unreadable; teams stop reading any of it.
5. **Define the exception path.** Teams will have legitimate reasons to
   deviate. A documented, low-friction exception with a stated expiry keeps
   deviations visible. No exception path means silent deviation, which is worse
   in every respect.
6. **Define how adoption is measured before publishing.** If you cannot say
   what number will tell you this worked, you are not ready to publish.

## The practice council

- **Composition**: principal/staff engineers from each domain, plus the
  platform team. Not a management forum — the people whose technical judgement
  the org already follows.
- **Cadence**: fortnightly, one standard per session, time-boxed.
- **Decision rights**: the council ratifies; domains implement; the director
  arbitrates deadlocks and owns the funding consequences.
- **Output**: every session produces a written decision, including "we
  considered this and chose not to standardise", which is the most
  under-recorded and most useful kind.
- **The rule that keeps it alive**: the council reviews **adoption data** for a
  previously ratified standard at every session, not only new proposals. A
  council that only ever adds is a committee generating homework.

## Adoption mechanics, cheapest first

| Mechanism | Cost to teams | When to use |
|---|---|---|
| Golden-path template | ~zero | Always. This is the primary mechanism. |
| Default in a shared library | ~zero | When the standard is a code-level choice |
| CI check, warning only | low | New standard, first quarter |
| CI check, blocking | medium | Once the paved road covers >80% of cases |
| Scorecard visibility | low | Continuously, aggregated at team level |
| Policy exception review | high | Only for standards with real safety or regulatory weight |

Work down the list. Reaching for a blocking CI check before the template exists
is the single most common way these programs earn their reputation.

## Retiring a standard

A program that has never withdrawn a standard is not measuring adoption.

Retire when: adoption stalled below the threshold you set at publication and
the reason is cost rather than awareness; the failure it prevented is now
prevented structurally; or the technology moved. Announce the withdrawal as
loudly as the publication, and say what replaced it.

## Reporting

Roll the per-repo scan up to a domain view and report three things:

1. **Coverage** — how many repositories were scanned, out of how many exist.
   Without this number the rest is unreadable.
2. **Distribution per practice** — how many repos at each level, not a mean. A
   mean of 2.0 could be everyone at 2 or half at 0 and half at 4, and those are
   completely different problems.
3. **Movement since last period** — with the count of repos that moved, not
   just the delta in the average.

Panel: **Cloud & Platform → Developer Excellence**, Practices tab.
Route: `GET /devex/practices`.
