---
triggers: ["EdTech", "LMS", "learning management", "e-learning", "SCORM", "xAPI", "adaptive learning", "course management", "student portal", "grading system", "assessment engine"]
tools_allowed: ["read_file", "write_file", "bash"]
category: education
---

# Education & EdTech Development

When working with education technology and learning management systems:

1. Design LMS architecture with clear separation between content delivery, user management, and analytics layers; use a multi-tenant model with tenant-scoped data isolation so a single deployment serves multiple institutions without data leakage.

2. Implement SCORM 1.2 and SCORM 2004 content packaging by exposing the required JavaScript API (`API` for 1.2, `API_1484_11` for 2004) in an iframe-based content player, persisting `cmi` data model elements (completion_status, score, suspend_data) to a backend store after each `Commit` call.

3. Adopt xAPI (Experience API) for granular learning event tracking by emitting statements in the Actor-Verb-Object-Result format to a Learning Record Store (LRS); normalize activity IRIs across content sources and batch-send statements with retry logic to handle LRS unavailability.

4. Build adaptive learning algorithms that maintain a per-learner knowledge state model (e.g., Bayesian Knowledge Tracing or IRT-based item parameters), select the next content item or question based on estimated mastery probability, and periodically recalibrate item difficulty from aggregate response data.

5. Construct assessment and quiz engines with a question bank organized by topic, difficulty, and Bloom's taxonomy level; support multiple item types (MCQ, fill-in-the-blank, drag-and-drop, code execution) and use item-level randomization plus answer shuffling to reduce cheating surface area.

6. Implement grade book calculations using a configurable weighting scheme (category weights, drop-lowest-N, extra credit flags); compute running totals incrementally on grade entry, store both raw and weighted scores, and expose a grade override audit trail for instructors.

7. Integrate plagiarism detection by submitting student work through a service API (Turnitin, Copyscape, or an open-source fingerprinting engine), storing similarity reports linked to submissions, and surfacing matched-source highlights in the instructor review UI without blocking the student submission flow.

8. Architect video streaming for courses using adaptive bitrate delivery (HLS with multiple renditions); store source video in object storage, trigger transcoding jobs on upload, generate chapter markers and searchable transcripts via speech-to-text, and track per-second watch analytics for engagement metrics.

9. Design discussion forum systems with threaded replies, mentions, and instructor-endorsed answers; implement real-time notifications via WebSocket or SSE, support Markdown and LaTeX rendering for STEM courses, and apply moderation queues with configurable auto-flag rules.

10. Build enrollment and registration workflows that enforce prerequisite chains, capacity limits, and waitlist promotion; use idempotent enrollment operations to prevent double-registration, emit enrollment events to downstream systems (billing, notifications, analytics), and support bulk enrollment via CSV import.

11. Create learning analytics dashboards that aggregate xAPI statements and LMS events into time-series and funnel visualizations; surface at-risk student indicators (low login frequency, declining quiz scores, missed deadlines), and provide drill-down from cohort-level to individual learner views.

12. Develop content authoring tools that output standards-compliant packages (SCORM ZIP or xAPI modules); provide a WYSIWYG block editor for text, media, interactive widgets, and embedded assessments, and support versioning so published courses can be updated without breaking in-progress learner progress.

13. Implement certificate and credential issuance by generating verifiable digital credentials (Open Badges v2 or Verifiable Credentials) upon course completion; sign credentials with the institution's key, host a public verification endpoint, and allow learners to export badges to LinkedIn or digital wallets.

14. Ensure accessibility compliance (WCAG 2.1 AA minimum) across all learner-facing interfaces, including keyboard navigation in the content player, captions and transcripts for all video content, screen-reader-compatible quiz interactions, and sufficient color contrast in the grading UI.

15. Protect student data privacy by enforcing FERPA (or local equivalent) controls: role-based access to PII, audit logging on grade and record access, data retention policies with automated purging, and encrypted storage for sensitive fields such as disability accommodations and disciplinary records.
