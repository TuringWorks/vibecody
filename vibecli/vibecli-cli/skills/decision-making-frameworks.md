---
triggers: ["decision making", "decision framework", "DACI", "trade-off analysis", "prioritization framework"]
tools_allowed: ["read_file", "write_file", "bash"]
category: strategy
---

# Decision-Making Frameworks

When structuring decisions and prioritization:

1. Use the DACI model (Driver, Approver, Contributors, Informed) to clarify roles for every significant decision, ensuring exactly one driver owns the process and one approver has final authority.
2. Classify decisions as reversible (two-way doors) or irreversible (one-way doors), applying lightweight processes for reversible decisions and thorough analysis only for those that are difficult to undo.
3. Build decision matrices with weighted scoring by listing options as rows, criteria as columns, assigning weights to criteria by importance, and scoring each option to produce a transparent, comparable ranking.
4. Timebox decisions to avoid analysis paralysis, setting a deadline proportional to the decision's impact and reversibility, and accepting that a good decision made on time outperforms a perfect decision made too late.
5. Document decisions and their rationale using Architecture Decision Records (ADRs) or similar formats, capturing context, options considered, trade-offs, and the chosen path for future reference.
6. Distinguish between consensus-based and consent-based decision-making: consensus requires everyone to agree, while consent requires no one to have a principled objection, enabling faster progress.
7. Practice disagree and commit culture where team members voice dissent during the decision process but fully support the outcome once a decision is made, avoiding passive resistance.
8. Perform cost of delay analysis to prioritize by urgency, quantifying the financial or strategic impact of postponing each option to surface decisions where speed matters more than perfection.
9. Apply opportunity cost thinking by explicitly considering what you give up with each choice, recognizing that every yes to one initiative is an implicit no to alternatives competing for the same resources.
10. Gather input asynchronously through written proposals (RFCs, design docs) before synchronous meetings, giving all contributors time to think deeply and ensuring quieter voices are heard.
11. Revisit decisions when significant new data emerges, establishing clear triggers for reopening past choices without creating instability through constant second-guessing.
12. Communicate decisions clearly by sharing the what (chosen option), why (rationale and trade-offs), who (DACI roles), and when (effective date), ensuring affected parties understand and can act on the outcome.
