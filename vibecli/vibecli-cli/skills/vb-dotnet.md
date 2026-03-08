---
triggers: ["Visual Basic", "VB.NET", "VB .NET", "Visual Basic .NET", "VB6", "VBA", "Basic .NET"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: vb
---

# Visual Basic .NET

When writing VB.NET code:

1. Use `Option Strict On` and `Option Explicit On` at the top of every file — prevents implicit narrowing conversions and forces variable declaration; catches type mismatch errors at compile time instead of runtime.
2. Use modern VB.NET patterns: `Dim result = Await HttpClient.GetStringAsync(url)` for async, `Dim items = From x In collection Where x.Active Select x.Name` for LINQ, `Using conn As New SqlConnection(connStr) ... End Using` for resource disposal.
3. Prefer `String.IsNullOrEmpty()` or `String.IsNullOrWhiteSpace()` over `= ""` or `Is Nothing` checks — handles both null and empty cases; use string interpolation `$"Hello {name}"` instead of concatenation.
4. Use `Try...Catch...Finally` for structured error handling: catch specific exceptions first (`Catch ex As SqlException`), generic last; never use `On Error Resume Next` (VB6 legacy) — it hides bugs silently.
5. Define classes with properties: `Public Property Name As String` (auto-property); use `ReadOnly` for immutable properties; implement `INotifyPropertyChanged` for WPF/MVVM data binding.
6. Use `Enum` with `<Flags>` attribute for bit fields; use `Structure` for small value types; prefer `Class` for reference semantics — VB.NET shares the full .NET type system with C#.
7. Collections: use `List(Of T)`, `Dictionary(Of TKey, TValue)`, `Queue(Of T)` from `System.Collections.Generic` — never use untyped `ArrayList` or `Hashtable`; use `IEnumerable(Of T)` for method parameters.
8. For database access: use Entity Framework Core with VB.NET — `Dim users = Await context.Users.Where(Function(u) u.Active).ToListAsync()` — parameterized queries prevent SQL injection.
9. Write unit tests with MSTest or NUnit: `<TestMethod> Public Sub TestAdd() Assert.AreEqual(4, Calculator.Add(2, 2)) End Sub` — use `<TestInitialize>` for setup.
10. Use `Async Function ... As Task(Of T)` and `Await` for all I/O operations — never block with `.Result` or `.Wait()`; VB.NET fully supports the TAP (Task-based Asynchronous Pattern).
11. For WinForms/WPF desktop apps: use data binding over manual UI updates; separate business logic from form code-behind; use the MVVM pattern with WPF.
12. Migrate VB6/VBA code incrementally: use `Microsoft.VisualBasic.Compatibility` namespace temporarily; replace `Variant` with proper types; replace `GoTo` with structured control flow; replace `ReDim Preserve` with `List(Of T)`.
