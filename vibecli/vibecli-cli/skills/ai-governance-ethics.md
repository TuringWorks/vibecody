---
triggers: ["AI governance", "AI ethics", "responsible AI", "AI safety", "AI bias", "AI regulation", "AI policy"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# AI Governance and Ethics

When implementing responsible AI practices and governance:

1. Detect bias in models by auditing training data for representation gaps and evaluating predictions across demographic groups using disaggregated metrics.
2. Apply fairness metrics appropriate to the use case: demographic parity for equal selection rates, equalized odds for equal error rates, or calibration for risk scoring.
3. Provide transparency and explainability using techniques like SHAP values for feature attribution, LIME for local explanations, and attention visualization for transformer models.
4. Understand EU AI Act compliance levels: classify systems by risk tier (unacceptable, high, limited, minimal) and implement mandatory requirements for high-risk applications.
5. Publish model cards documenting intended use, training data, performance benchmarks, limitations, and ethical considerations; pair with datasheets for datasets.
6. Design human-in-the-loop workflows for high-stakes decisions: ensure humans can review, override, and understand AI recommendations before they take effect.
7. Implement privacy-preserving ML techniques: differential privacy for training data protection, federated learning to keep data on-device, and secure aggregation for model updates.
8. Red team AI systems by simulating adversarial inputs, prompt injection attacks, and edge cases to identify failure modes before deployment.
9. Establish an incident response plan for AI failures: define severity levels, escalation paths, rollback procedures, and post-incident review processes.
10. Conduct AI risk assessments using structured frameworks (NIST AI RMF, ISO 42001) that evaluate impact, likelihood, and mitigation strategies across the AI lifecycle.
11. Implement content moderation and safety filters with layered defenses: input classifiers, output filters, rate limiting, and human review queues for flagged content.
12. Establish an AI governance committee with cross-functional representation (engineering, legal, ethics, product) that reviews deployments, sets policies, and handles escalations.
