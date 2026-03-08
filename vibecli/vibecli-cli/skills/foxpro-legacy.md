---
triggers: ["FoxPro", "Visual FoxPro", "VFP", "dBASE", "xBase", "FoxPro migration", "DBF files"]
tools_allowed: ["read_file", "write_file", "bash"]
category: legacy
---

# Visual FoxPro / xBase

When maintaining or migrating Visual FoxPro code:

1. VFP is end-of-life (no Microsoft support since 2015) — prioritize migration to modern platforms; use VFP knowledge for maintaining existing systems, data extraction, and planning migration strategies.
2. VFP data is stored in DBF/CDX/FPT files: `USE customers EXCLUSIVE; BROWSE` to view data; `SELECT * FROM customers WHERE state = 'CA' INTO CURSOR results` for SQL queries — VFP supports both xBase commands and SQL syntax.
3. Use SQL syntax over xBase commands for new code: `SELECT c.name, SUM(o.total) FROM customers c JOIN orders o ON c.id = o.cust_id GROUP BY c.name` is clearer than `SCAN/ENDSCAN` loops with `SEEK`/`LOCATE`.
4. Handle tables carefully: `USE table IN 0` opens in next work area; `SELECT table` switches context; always `CLOSE TABLES ALL` in cleanup — VFP's work area model requires explicit table management.
5. VFP classes: `DEFINE CLASS Customer AS Custom; name = ''; PROCEDURE Init; LPARAMETERS tcName; THIS.name = tcName; ENDPROC; ENDDEFINE` — VFP has full OOP with inheritance, polymorphism, and visual form designer classes.
6. Migration strategies: export data with `COPY TO data.csv TYPE CSV` or connect directly from Python/C# via ODBC; rewrite business logic in C#/.NET or Python; use DBF libraries (`dbfread` for Python, `DbfDataReader` for .NET) for data access.
7. Common pitfalls: VFP uses 1-based arrays and strings; `EMPTY()` checks for empty/null/zero; `TYPE()` returns variable type as string; `VARTYPE()` is preferred (returns 'C', 'N', 'D', 'L', etc.); dates require `{}` delimiters: `{^2025-01-15}`.
8. For data extraction: `SELECT * FROM table INTO TABLE newtable` creates a copy; `APPEND FROM other.dbf` imports data; `EXPORT TO file.xlsx TYPE XL8` for Excel — always back up DBF/CDX/FPT files together as a set.
