---
triggers: ["medical research", "clinical trial", "biostatistics", "epidemiology", "survival analysis", "clinical data", "HIPAA", "HL7", "FHIR", "electronic health records", "EHR", "pharmacovigilance", "drug discovery"]
tools_allowed: ["read_file", "write_file", "bash"]
category: scientific
---

# Medical Research & Clinical Data

When working on medical research, clinical trials, and health data analysis:

1. Handle patient data with strict privacy controls: never log, print, or commit PHI (Protected Health Information) — use de-identification before any analysis: replace names with IDs, generalize dates to year/month, remove ZIP codes below 20k population per HIPAA Safe Harbor.
2. Use REDCap or CDISC (SDTM/ADaM) standards for clinical trial data — structure datasets with one row per subject per timepoint; use CDISC variable naming conventions (`USUBJID`, `AVAL`, `AVALC`, `AVISIT`) for regulatory submissions.
3. Perform survival analysis with `lifelines` (Python) or `survival` (R): `KaplanMeierFitter().fit(durations, event_observed)` — report median survival, hazard ratios from Cox regression, and log-rank test p-values with confidence intervals.
4. Use `statsmodels` or R's `lme4` for mixed-effects models: `mixedlm("outcome ~ treatment + time", data, groups="patient_id")` — accounts for repeated measures and patient-level random effects in longitudinal studies.
5. Apply multiple comparison correction for multi-endpoint trials: use Bonferroni for few comparisons, Benjamini-Hochberg FDR for exploratory analyses — report both raw and adjusted p-values.
6. Calculate sample size before starting: `from statsmodels.stats.power import TTestIndPower; TTestIndPower().solve_power(effect_size=0.5, alpha=0.05, power=0.80)` — inadequately powered studies waste resources and can miss real effects.
7. Use CONSORT flow diagrams for RCTs: track enrollment, allocation, follow-up, and analysis — report intention-to-treat (ITT) as primary, per-protocol as secondary analysis.
8. Implement data quality checks: verify ranges (`0 <= age <= 120`), cross-field consistency (`death_date >= admission_date`), missingness patterns (MCAR/MAR/MNAR) — use `missingno.matrix(df)` for visualization.
9. For genomic/omics data: use Bioconductor's `DESeq2` for differential expression, `limma` for microarray analysis, `PLINK` for GWAS — apply genome-wide significance threshold (p < 5e-8) and report Manhattan plots.
10. Access clinical terminologies via APIs: use SNOMED CT for diagnoses, LOINC for lab tests, RxNorm for medications, ICD-10 for billing codes — map between systems with UMLS Metathesaurus.
11. Use FHIR (Fast Healthcare Interoperability Resources) for EHR integration: `fhirclient` (Python) or `fhirr4` (R) — query patient resources with `Patient.search(family='Smith')` and parse Bundle responses.
12. Build predictive models responsibly: report AUROC, calibration plots, and decision curve analysis — validate on held-out temporal cohorts (not random splits) to simulate real-world deployment.
13. For drug discovery pipelines: use RDKit for molecular fingerprints and SMILES parsing, AutoDock Vina for docking, and `DeepChem` for molecular ML — always validate predictions with wet-lab assays.
14. Document everything per STROBE (observational) or SPIRIT (trial protocol) guidelines — pre-register hypotheses on ClinicalTrials.gov or OSF before data collection to prevent p-hacking.
15. Use `pandas` with `datetime` for time-to-event data: calculate `los = (discharge_date - admission_date).dt.days`; censor at study end date; handle competing risks with `cmprsk` (R) or `lifelines.CoxPHFitter` with cause-specific models.
