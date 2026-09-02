---
name: "Principal Engineering Community and Technical Influence"
description: "Principal Engineering Community and Technical Influence: leading senior engineers you do not manage — architecture councils, RFC processes, communities of practice, and technical strategy that principal engineers will actually carry. Covers the RFC lifecycle, council design, how to disagree productively at senior level, and the failure modes of architecture governance. Use when the task involves architecture council, RFC process, principal engineers, technical strategy, community of practice, engineering influence without authority, or architecture review board."
category: devex
triggers: ["architecture council", "RFC process", "principal engineers", "technical strategy", "community of practice", "influence without authority", "architecture review board", "staff engineer", "technical leadership"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Leading engineers you do not manage

Principal and staff engineers are the transmission mechanism for any
enterprise-wide engineering direction. If they carry it, it reaches every team
within two quarters. If they merely comply with it, it reaches slide decks.

## The trade

Senior engineers give attention and advocacy. In exchange they need three
things, and the exchange fails if any one is missing:

1. **Real decision rights** on something that matters, not consultation.
2. **Context they cannot get elsewhere** — the funding constraints, the
   commercial pressure, the political shape of a decision. Withholding this is
   the fastest way to get shallow engagement.
3. **Their objections recorded and answered**, including the ones you overrule.
   A recorded, answered objection buys advocacy; an unrecorded one buys a
   quiet, well-argued alternative implementation.

## The RFC lifecycle

The single highest-leverage mechanism. Keep it lightweight or it will be
routed around.

| Stage | Duration | Exit condition |
|---|---|---|
| Draft | any | Author decides it is readable |
| Open for comment | 2 weeks, fixed | Comment window closes |
| Revision | 1 week | Author responds to every substantive comment in writing |
| Decision | 1 session | Accepted / rejected / deferred, **with a reason** |
| Adopted | — | Golden path updated, standard published |

Rules that keep it working:
- **A fixed comment window.** Open-ended review is where RFCs go to die.
- **Every substantive comment answered in writing.** Silence reads as contempt,
  and the person silenced will remember it for years.
- **Rejections recorded with reasons and kept.** The rejected-RFC archive is
  one of the most valuable documents an engineering org has: it stops the same
  proposal returning every eighteen months.
- **The decider is named in advance.** Consensus-seeking bodies with no named
  decider stall on exactly the decisions that matter most.

## Council design

- **Small.** Seven to nine. Above that people stop preparing.
- **Rotating minority membership.** Keep a couple of seats rotating through
  senior engineers outside the usual set — it prevents the council becoming a
  fixed faction and it is a genuine development opportunity.
- **Standing agenda**: one adoption review, one new proposal, one open topic
  raised by a member. The adoption review first, so it never gets squeezed out.
- **Public minutes.** Every decision, its reason, and its dissents.
- **A written escalation path** for when the council cannot agree. It will
  happen. Deciding the path in advance keeps the deadlock technical rather than
  personal.

## Communities of practice

Different instrument, different purpose: councils decide, communities spread.

- Organised by discipline (backend, data, mobile, SRE) rather than by product.
- Monthly, one talk from inside the organisation, one open floor.
- **Owned by a practitioner, not by the platform team.** A community of
  practice run by the central team becomes a broadcast channel within three
  sessions.
- Success measure: proposals and standards originating *from* the community.
  Attendance is a vanity number.

## Disagreeing well at senior level

- **Separate the disagreement type**: facts, values, or risk appetite. Most
  senior technical disputes are risk-appetite disputes wearing a factual
  costume, and they cannot be resolved with more data.
- **Name the reversibility.** One-way doors deserve slow, wide consultation;
  two-way doors deserve a decision this week and a review date.
- **Write down what would change your mind**, before the discussion. It makes
  the conversation an inquiry instead of a negotiation, and it obliges the
  other side to do the same.
- **Overrule out loud, with the reason and the owner of the consequence.** A
  decision that arrives without an author cannot be revisited and will be
  re-litigated indefinitely.

## The failure modes

1. **The council becomes an approval gate.** Throughput drops, teams route
   around it, and its authority evaporates. Councils should set direction and
   review a sample — not approve every change.
2. **Standards written by people who no longer ship.** Require every standard
   to have an implementing team that used it before it was ratified.
3. **The director does all the talking.** If the senior engineers are not
   arguing with each other, they are not engaged; they are attending.
4. **No consequence either way.** If nothing is different for a team that
   ignores the council entirely, the council is a discussion group.

## Where it lives in VibeCody

- RFCs and decision records → **Architecture** panel (ADR support)
- Council minutes and standards → **Project Hub** documents
- Adoption data for the review slot → `vibecli --devex practices --json`
- Commitments made in council → **Goals** panel
