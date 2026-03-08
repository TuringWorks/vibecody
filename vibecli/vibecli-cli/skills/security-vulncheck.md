---
triggers: ["VulnCheck", "vulncheck", "vulncheck API", "exploit intelligence", "vulncheck index", "canary intelligence", "vulncheck nvd2"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# VulnCheck Exploit Intelligence Platform

When working with VulnCheck:

1. Authenticate with the VulnCheck API by setting your token as an environment variable and testing connectivity: `export VULNCHECK_API_TOKEN=vc1_...` then `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" https://api.vulncheck.com/v3/index` to list all available intelligence indices.

2. Query the vulncheck-nvd2 index for enriched NVD data that includes faster updates than NIST: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/vulncheck-nvd2?cve=CVE-2024-XXXX" | jq '.data[0]'` to get CVSS scores, references, and affected CPEs.

3. Monitor exploit PoC availability by querying the exploits index: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/initial-access?cve=CVE-2024-XXXX" | jq '.data[] | {name, date_added, exploit_type}'` to determine if a vulnerability has weaponized exploits in the wild.

4. Leverage CVSS temporal scoring from VulnCheck to adjust base scores with real-world exploit maturity: compare `base_score` against VulnCheck's `temporal_score` field which accounts for exploit code availability, active exploitation status, and remediation level for accurate prioritization.

5. Enrich vulnerability records with CPE data by cross-referencing VulnCheck's CPE index: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/cpe?product=apache&vendor=apache" | jq '.data[] | .cpe23'` to match affected software versions in your asset inventory.

6. Consume canary intelligence feeds to detect early-warning exploitation signals before CVE assignment: poll the VulnCheck canary endpoint on a cron schedule with `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/canary" | jq '.data[] | select(.last_seen > "2026-03-01")'`.

7. Query initial access intelligence to identify vulnerabilities actively used for network entry by threat actors: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/initial-access" | jq '[.data[] | select(.ransomware == true)] | length'` to count ransomware-associated initial access vectors.

8. Integrate EPSS (Exploit Prediction Scoring System) data from VulnCheck to probability-rank vulnerabilities: fetch EPSS scores alongside CVSS with `jq '.data[0] | {cve, cvss_base: .cvss_v3_score, epss: .epss_score}'` and prioritize CVEs with EPSS > 0.3 (top 30% exploitation likelihood) first.

9. Enrich your vulnerability data with CISA KEV status through VulnCheck's unified API: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/index/vulncheck-kev" | jq '.data[] | select(.cve == "CVE-2024-XXXX") | {cve, date_added, due_date}'` to check if a CVE is in the KEV catalog without hitting CISA directly.

10. Set up offline backup ingestion by periodically downloading full index snapshots: `curl -s -H "Authorization: Bearer $VULNCHECK_API_TOKEN" "https://api.vulncheck.com/v3/backup/vulncheck-nvd2" -o nvd2-backup.json.gz` and load into local PostgreSQL or Elasticsearch for air-gapped environments.

11. Forward VulnCheck intelligence to your SIEM by building a collector script that polls indices and outputs CEF or JSON to syslog: pipe results through `jq -c '.data[]'` and send via `logger -n $SIEM_HOST -P 514 -t vulncheck` or use a Logstash HTTP input to correlate exploit intel with asset telemetry.

12. Automate VulnCheck-based triage by writing a script that combines initial-access, KEV, and EPSS data: fetch all three indices, join on CVE ID, and output a priority matrix where KEV + EPSS > 0.5 + initial-access = P0 immediate action, enabling data-driven patching decisions over gut-feel severity alone.
