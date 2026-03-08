---
triggers: ["SAS", "SAS programming", "SAS macro", "PROC SQL", "SAS dataset", "SAS clinical", "SAS analytics", "DATA step", "PROC MEANS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: sas
---

# SAS Programming

When writing SAS code for analytics, clinical trials, and enterprise reporting:

1. Use the DATA step for data manipulation: `DATA output; SET input; WHERE age > 18; bmi = weight / (height**2); IF bmi > 30 THEN category = 'Obese'; RUN;` — the DATA step reads observations sequentially and is SAS's core processing engine.
2. Use PROC SQL for relational queries: `PROC SQL; CREATE TABLE result AS SELECT a.*, b.dept_name FROM employees a LEFT JOIN departments b ON a.dept_id = b.dept_id; QUIT;` — supports full SQL syntax including subqueries, unions, and window functions.
3. Use SAS macros for reusable code: `%MACRO summarize(dsn, var); PROC MEANS DATA=&dsn; VAR &var; RUN; %MEND; %summarize(mydata, salary);` — `&` resolves macro variables; `%LET` defines them; use `%IF/%THEN` for conditional generation.
4. For statistical analysis: `PROC MEANS` (descriptive stats), `PROC FREQ` (frequency tables, chi-square), `PROC REG` (linear regression), `PROC LOGISTIC` (logistic regression), `PROC MIXED` (mixed models), `PROC LIFETEST` (survival analysis).
5. Handle dates correctly: SAS dates are integers (days since Jan 1, 1960) — use `INPUT(datestr, YYMMDD10.)` to parse; `PUT(sasdate, DATE9.)` to format; `INTCK('MONTH', start, end)` for intervals; `INTNX('MONTH', date, 1)` to increment.
6. For clinical trials (CDISC): create SDTM domains with `DM` (demographics), `AE` (adverse events), `LB` (lab results); produce ADaM datasets `ADSL` (subject-level), `ADAE`, `ADLB`; generate TLFs (Tables, Listings, Figures) per SAP.
7. Use ODS (Output Delivery System) for publication-quality output: `ODS PDF FILE='report.pdf'; PROC REPORT DATA=summary; COLUMN name n mean std; RUN; ODS PDF CLOSE;` — supports PDF, RTF, HTML, Excel, and PowerPoint.
8. Read external data: `PROC IMPORT DATAFILE='data.csv' OUT=mydata DBMS=CSV REPLACE; GETNAMES=YES; RUN;` — use `INFILE` statement in DATA step for fixed-width or complex delimited files with `INPUT` format specifications.
9. Use arrays for repetitive column operations: `ARRAY scores{5} score1-score5; DO i = 1 TO 5; IF scores{i} = . THEN scores{i} = 0; END;` — arrays simplify operations across multiple variables with similar structure.
10. Optimize performance: use `WHERE` instead of `IF` for subsetting (WHERE is processed at read time); use indexes on large datasets; use `PROC SORT NODUPKEY` to remove duplicates; set `OPTIONS COMPRESS=YES;` for large datasets.
11. Debug with `PUT` and `PUTLOG`: `PUT _ALL_;` writes all variables to the log; `PUTLOG 'NOTE: count=' count;` for specific values; use `OPTIONS MPRINT SYMBOLGEN MLOGIC;` to trace macro resolution.
12. For modern SAS: use SAS Viya for cloud-based analytics with CAS (Cloud Analytic Services); SAS Studio for web IDE; `PROC PYTHON` to embed Python code; REST APIs for integration with external systems.
