---
triggers: ["technical writing", "documentation writing", "writing RFCs", "writing ADRs", "clear writing", "engineering blog"]
tools_allowed: ["read_file", "write_file", "bash"]
category: people-skills
---

# Technical Writing

When writing technical documentation and communications:

1. Write for your audience — a design doc for engineers needs different depth than an executive summary for leadership; identify your reader before you write a single word and adjust jargon, detail level, and length accordingly.
2. Follow a proven RFC and design doc structure — include Context (why now), Proposal (what and how), Alternatives Considered (what else and why not), Risks and Mitigations, and a Decision section; this structure forces clear thinking.
3. Practice concise writing ruthlessly — after your first draft, cut fifty percent of the words; remove filler phrases like "in order to" (use "to"), "it should be noted that" (delete entirely), and "at this point in time" (use "now").
4. Use active voice and short sentences — "The service processes 10K requests per second" beats "10K requests per second are processed by the service"; active voice is clearer, shorter, and more direct.
5. Prefer diagrams over paragraphs — a Mermaid sequence diagram or a draw.io architecture diagram communicates in seconds what takes three paragraphs to explain; include both for accessibility.
6. Write READMEs with quick start first — lead with a three-line install and run section before explaining architecture; developers evaluate projects in under sixty seconds and will leave if they cannot get started fast.
7. Write changelogs that explain impact — "Fixed race condition in session handler that caused 500 errors under load" tells users what matters; "Updated handler.rs" tells them nothing.
8. Document APIs with examples over descriptions — a curl command that works is worth more than a paragraph explaining parameters; show the request, show the response, then explain the fields.
9. Structure blog posts for scanners — use descriptive headings, short paragraphs, code blocks, and a TL;DR at the top; most readers skim before deciding to read in depth.
10. Edit through peer review — have someone unfamiliar with the project read your document and note where they get confused; their confusion reveals your blind spots.
11. Version your documentation alongside code — docs in the repo (not a wiki) get updated with pull requests; stale documentation is worse than no documentation because it actively misleads.
12. Write runbooks and playbooks as numbered steps — each step should be a single action with an expected outcome; during an incident at 3 AM, nobody wants to parse prose to figure out what to do next.
