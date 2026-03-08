---
triggers: ["ASVS", "application security verification", "penetration testing", "threat modeling", "STRIDE", "PASTA", "security requirements", "security testing", "AppSec verification"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Application Security Verification

When working with application security verification:

1. Select the appropriate OWASP ASVS level based on application risk: Level 1 (opportunistic) for low-risk apps with automated scanning, Level 2 (standard) for most business applications requiring manual testing against 286 controls, Level 3 (advanced) for critical systems like financial or healthcare apps requiring deep architecture review and all 286 controls verified.

2. Scope penetration tests by defining clear boundaries: document in-scope URLs, API endpoints, authentication methods, test accounts, and excluded systems; use `nmap -sV -p- target.example.com -oX nmap-scope.xml` for port discovery, then configure OWASP ZAP with a context file limiting the spider and scanner to authorized targets only.

3. Build a security requirements traceability matrix mapping each requirement to ASVS controls: create a spreadsheet or YAML file linking user stories (e.g., "users must authenticate with MFA") to specific ASVS items (V2.8.1), test cases that verify the control, and evidence artifacts; track coverage percentage as `verified_controls / applicable_controls * 100`.

4. Perform STRIDE threat modeling during design phase: for each component in the data flow diagram, systematically evaluate Spoofing (authentication), Tampering (integrity), Repudiation (logging), Information Disclosure (confidentiality), Denial of Service (availability), and Elevation of Privilege (authorization); document threats in a table with likelihood, impact, and mitigations.

5. Apply PASTA (Process for Attack Simulation and Threat Analysis) for risk-centric modeling: define business objectives (Stage 1), enumerate technical scope (Stage 2), decompose the application (Stage 3), analyze threats (Stage 4), identify vulnerabilities (Stage 5), enumerate attack patterns (Stage 6), and calculate risk with business impact analysis (Stage 7) for executive-level risk communication.

6. Define security acceptance criteria for every user story in the backlog: add criteria like "input validation rejects payloads matching OWASP Top 10 patterns", "session tokens expire after 15 minutes of inactivity", "API rate limiting enforced at 100 req/min per user"; verify with targeted tests: `curl -X POST -d '<script>alert(1)</script>' $URL | grep -c 'script'` should return 0.

7. Test API security systematically using automated tools: `nuclei -u https://api.example.com -t http/cves/ -t http/misconfiguration/ -json -o nuclei-api.json` for known vulnerabilities, `jwt_tool $TOKEN -M at` for JWT testing, and `arjun -u https://api.example.com/endpoint` for hidden parameter discovery; validate OWASP API Security Top 10 controls.

8. Verify authentication and authorization controls thoroughly: test for broken access control by accessing resources with different user roles using `curl -H "Authorization: Bearer $LOW_PRIV_TOKEN" $ADMIN_ENDPOINT`, verify MFA enforcement by attempting bypass of second factor, check password policies with mutation testing, and confirm account lockout after failed attempts.

9. Test business logic vulnerabilities that automated scanners miss: identify state-dependent workflows (checkout, approval chains, role escalation), test for race conditions with `parallel curl -X POST $ENDPOINT ::: $(seq 100)`, verify quantity/price manipulation in e-commerce flows, and test for IDOR by enumerating object IDs across user sessions.

10. Build a security regression test suite that runs in CI: convert confirmed vulnerabilities into repeatable test cases using `nuclei -t custom-templates/` with organization-specific templates, `zap-baseline.py -t $URL -c zap-rules.conf` for baseline scans, and language-specific security test libraries like `org.owasp.encoder` assertions in unit tests.

11. Integrate security testing into the development workflow at multiple stages: pre-commit (secrets scanning, SAST linting), PR review (automated SAST with Semgrep, dependency scanning), staging deployment (DAST with ZAP, API fuzzing), and pre-release (manual penetration test, ASVS checklist review); gate progression on zero critical and high findings.

12. Document and track security verification results using structured reporting: generate ASVS compliance reports with pass/fail per control, maintain a vulnerability register with `gh issue list --label security --json number,title,state` tracking remediation status, produce executive summaries showing risk score trends, and archive penetration test reports with evidence for compliance audits.
