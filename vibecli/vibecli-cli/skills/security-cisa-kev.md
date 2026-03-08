---
triggers: ["CISA KEV", "known exploited vulnerabilities", "CISA catalog", "KEV catalog", "BOD 22-01", "cisa vulnerability", "exploited vulnerability catalog"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# CISA Known Exploited Vulnerabilities Catalog

When working with the CISA KEV catalog:

1. Fetch the full KEV catalog in JSON format for programmatic processing: `curl -s https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json | jq '.vulnerabilities | length'` to get the current count, or use the CSV feed at `known_exploited_vulnerabilities.csv` for spreadsheet-based workflows.

2. Poll for new KEV entries on a schedule by comparing against a cached version: `curl -s $KEV_URL | jq -r '.vulnerabilities[-10:][].cveID'` to list the 10 most recently added CVEs, and diff against your local `kev-cache.json` to detect additions since your last check; run this via cron every 6 hours.

3. Ensure BOD 22-01 compliance for federal agencies by tracking remediation deadlines: `jq '.vulnerabilities[] | select(.dueDate >= "2026-03-01" and .dueDate <= "2026-03-31") | {cveID, dueDate, product}' kev.json` to list all CVEs with due dates in the current month and verify patches are applied before the deadline.

4. Integrate KEV data with your patch management system by cross-referencing CVE IDs against your CMDB: extract affected vendor/product pairs with `jq '.vulnerabilities[] | {cveID, vendorProject, product, requiredAction}'` and match against installed software inventory to generate targeted patch lists.

5. Prioritize patching using KEV as the top signal: any CVE in the KEV catalog has confirmed active exploitation and should override CVSS-only prioritization; filter your scanner output with `jq --slurpfile kev kev.json '[.[] | select(.cve as $c | $kev[0].vulnerabilities[] | .cveID == $c)]'` to isolate KEV-listed findings.

6. Set up automated alerting for new KEV entries by running a cron job that checks the catalog `dateAdded` field: `jq "[.vulnerabilities[] | select(.dateAdded == \"$(date +%Y-%m-%d)\")] | length" kev.json` and send Slack/email notifications with CVE details, affected products, and remediation due dates when new entries appear.

7. Combine KEV with EPSS scores for two-dimensional prioritization: KEV = confirmed exploitation (patch now), EPSS > 0.7 without KEV = likely exploitation soon (patch this sprint), EPSS < 0.1 without KEV = deprioritize; join datasets on CVE ID using `jq` or a Python script merging both JSON feeds.

8. Correlate KEV entries with CVSS base scores to identify severity mismatches: some KEV entries have moderate CVSS (5.0-6.9) but confirmed exploitation, proving that CVSS alone underestimates risk; query `jq '.vulnerabilities[] | select(.cveID | test("2024")) | {cveID, knownRansomwareCampaignUse}'` to find ransomware-linked CVEs.

9. Adapt KEV for private sector use even though BOD 22-01 only mandates federal agencies: treat the KEV as an authoritative exploitation signal, set internal SLAs (e.g., 14 days for internet-facing, 30 days for internal), and report KEV remediation rates to leadership as a key risk metric.

10. Integrate KEV checks into CI/CD pipelines by adding a gate that fails builds if dependencies contain KEV-listed CVEs: `grype . -o json | jq '[.matches[] | select(.vulnerability.id as $v | $KEV_LIST | index($v))]'` where `$KEV_LIST` is a pre-fetched array of KEV CVE IDs, blocking deployment of known-exploited components.

11. Track the KEV `knownRansomwareCampaignUse` field to escalate ransomware-associated CVEs: `jq '[.vulnerabilities[] | select(.knownRansomwareCampaignUse == "Known")] | length' kev.json` gives the count of ransomware-linked CVEs; these warrant immediate executive visibility and accelerated patching regardless of other factors.

12. Generate compliance evidence reports by exporting KEV remediation status: produce a matrix of all KEV CVEs applicable to your environment with columns for CVE ID, date added, due date, patch status, and completion date; automate with `jq` transformations into CSV and archive monthly snapshots for audit trail documentation.
