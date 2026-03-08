---
triggers: ["PL/I", "PL/1", "PL1", "PL/I mainframe", "Enterprise PL/I", "IBM PL/I"]
tools_allowed: ["read_file", "write_file", "bash"]
category: legacy
---

# PL/I (Programming Language One)

When maintaining or working with PL/I code on IBM mainframes:

1. PL/I combines features of FORTRAN (scientific), COBOL (business), and ALGOL (structured) — it supports fixed/float decimal for financial math, pointers for systems programming, and structured data for business records.
2. Program structure: `PROC_NAME: PROCEDURE OPTIONS(MAIN); DCL ... ; ... END PROC_NAME;` — every PL/I program is a procedure; use `OPTIONS(MAIN)` for the entry point; statements end with semicolons.
3. Declare variables with `DCL`: `DCL amount FIXED DECIMAL(9,2);` (financial), `DCL name CHAR(30) VARYING;` (string), `DCL flags BIT(8);` (bit string), `DCL ptr POINTER;` — PL/I has rich data type support including structures.
4. Use structures for records: `DCL 1 customer, 2 name CHAR(30), 2 address, 3 street CHAR(40), 3 city CHAR(20), 3 state CHAR(2), 2 balance FIXED DEC(11,2);` — levels define hierarchy; access with `customer.address.city`.
5. String handling: `LENGTH(str)`, `SUBSTR(str, pos, len)`, `INDEX(str, search)`, `TRANSLATE(str, to, from)`, `TRIM(str)` — PL/I strings can be fixed-length (`CHAR(n)`), varying-length (`CHAR(n) VARYING`), or unaligned.
6. Arithmetic: PL/I handles fixed decimal with full precision control — `DCL result FIXED DEC(15,2) = amount * rate;` — compiler determines intermediate precision based on operand attributes; use explicit declarations to avoid precision loss.
7. Error handling with ON conditions: `ON ENDFILE(INFILE) eof = '1'B;` for file end; `ON CONVERSION ...` for data conversion errors; `ON ZERODIVIDE ...` for division by zero — condition handling is PL/I's exception mechanism.
8. File I/O: `DCL INFILE FILE RECORD INPUT; OPEN FILE(INFILE); READ FILE(INFILE) INTO(record); ... CLOSE FILE(INFILE);` — supports sequential, indexed (VSAM), and regional file organizations.
9. Use `SELECT/WHEN/OTHERWISE/END` for multi-way branching (PL/I's switch/case): `SELECT(status); WHEN('A') CALL process_active; WHEN('I') CALL process_inactive; OTHERWISE CALL handle_unknown; END;`.
10. Built-in functions: `DATE()` for current date, `TIME()` for current time, `MAX(a,b)`, `MIN(a,b)`, `MOD(a,b)`, `ABS(x)`, `CEIL(x)`, `FLOOR(x)` — `DATETIME()` returns full timestamp; use picture formatting for display.
11. Enterprise PL/I (modern compiler): supports 64-bit addressing, XML/JSON processing (`XMLCHAR`, `JSONPUT`), SQL preprocessor for DB2 access, UTF-8/UTF-16 strings — compile with `RULES(IBM)` for strict checking.
12. Migration considerations: PL/I's fixed decimal maps to `DECIMAL` in SQL and `decimal` in C#/Java; structures map to records/classes; ON conditions map to try/catch — gradual migration via wrapping PL/I as web services is common.
