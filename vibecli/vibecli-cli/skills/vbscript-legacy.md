---
triggers: ["VBScript", "VBS", "Windows Script Host", "WSH", "ASP Classic", "WMI scripting", "HTA"]
tools_allowed: ["read_file", "write_file", "bash"]
category: vb
---

# VBScript

When maintaining or migrating VBScript code:

1. VBScript is legacy (deprecated by Microsoft) — prefer PowerShell for new Windows automation; for ASP Classic, migrate to ASP.NET; use VBScript knowledge primarily for maintaining existing systems and planning migrations.
2. Declare variables explicitly: `Option Explicit` at the top of every script — VBScript is dynamically typed (everything is `Variant`); without `Option Explicit`, typos create new variables silently.
3. Use `Dim` for variable declaration: `Dim name, count, items()` — `ReDim Preserve items(newSize)` to resize arrays keeping data; use `Dictionary` object for key-value pairs: `Set dict = CreateObject("Scripting.Dictionary")`.
4. Error handling with `On Error Resume Next`: `On Error Resume Next; result = riskyOperation; If Err.Number <> 0 Then WScript.Echo Err.Description; Err.Clear; End If; On Error GoTo 0` — always check and clear errors immediately.
5. Use `CreateObject` for COM automation: `Set fso = CreateObject("Scripting.FileSystemObject")` for files; `Set shell = CreateObject("WScript.Shell")` for running commands; `Set excel = CreateObject("Excel.Application")` for Office automation.
6. File operations with FileSystemObject: `Set f = fso.OpenTextFile("data.txt", 1); Do Until f.AtEndOfStream; line = f.ReadLine; Loop; f.Close` — mode 1=read, 2=write, 8=append; use `fso.FileExists()` before opening.
7. WMI for system administration: `Set wmi = GetObject("winmgmts:\\.\root\cimv2"); Set procs = wmi.ExecQuery("SELECT * FROM Win32_Process"); For Each p In procs; WScript.Echo p.Name; Next` — query hardware, services, processes, and OS info.
8. For ASP Classic pages: `<% Response.Write "Hello " & Request.QueryString("name") %>` — always use `Server.HTMLEncode()` for output and parameterized queries for database access to prevent XSS and SQL injection.
9. String functions: `Len(s)`, `Mid(s, start, length)`, `InStr(s, find)`, `Replace(s, old, new)`, `Split(s, delim)`, `Join(arr, delim)`, `UCase/LCase`, `Trim/LTrim/RTrim` — VBScript strings are 1-based, not 0-based.
10. Date functions: `Now` (current date/time), `DateAdd("m", 1, dtDate)` (add 1 month), `DateDiff("d", date1, date2)` (days between), `FormatDateTime(dt, vbShortDate)` — dates are `Variant` subtypes; use `IsDate()` to validate.
11. Migration path to PowerShell: `WScript.Shell.Run` → `Start-Process`; `FileSystemObject` → `Get-Content/Set-Content`; `WMI` → `Get-CimInstance`; `Dictionary` → `@{}`; `CreateObject("ADODB.Connection")` → `Invoke-SqlCmd`.
12. Run scripts: `cscript script.vbs` for console output; `wscript script.vbs` for GUI dialogs; use `//nologo` to suppress banner; schedule with Task Scheduler for automation.
