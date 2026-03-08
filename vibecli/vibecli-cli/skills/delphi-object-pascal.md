---
triggers: ["Delphi", "Object Pascal", "Free Pascal", "Lazarus", "RAD Studio", "FireMonkey", "VCL", "Embarcadero", "FPC"]
tools_allowed: ["read_file", "write_file", "bash"]
category: delphi
---

# Delphi / Object Pascal

When writing Delphi/Object Pascal code:

1. Use modern Delphi features: generics (`TList<T>`, `TDictionary<TKey, TValue>`), anonymous methods (`procedure(const Value: Integer)`), inline variables (`var I := 0`), and type inference — available since Delphi 2009+.
2. Follow the `T` prefix convention for types: `TCustomer`, `TOrderList`, `ICustomerService` (interface) — use `F` prefix for private fields, no prefix for properties; PascalCase for everything.
3. Use interfaces for dependency injection and testability: `ILogger = interface procedure Log(const Msg: string); end;` — Delphi interfaces are reference-counted; implement with `TInterfacedObject` for automatic lifetime management.
4. Handle memory carefully: Delphi uses manual memory management for objects — always pair `Create` with `Free` in `try...finally`: `Obj := TMyClass.Create; try ... finally Obj.Free; end;` Use `FreeAndNil(Obj)` to prevent dangling pointers.
5. Use `TStringList`, `TList<T>`, and `TObjectList<T>` for collections — `TObjectList<T>` with `OwnsObjects := True` auto-frees contained objects; use `TDictionary<K,V>` for key-value lookups.
6. For database access: use FireDAC (modern) over BDE (legacy) — `FDQuery.SQL.Text := 'SELECT * FROM users WHERE id = :id'; FDQuery.ParamByName('id').AsInteger := UserId; FDQuery.Open;` — always use parameters.
7. Build cross-platform UIs with FireMonkey (FMX): target Windows, macOS, iOS, Android from one codebase; use VCL for Windows-only desktop apps with native look — LiveBindings connect UI to data sources declaratively.
8. Use `try...except` for exception handling: `try ... except on E: EDatabaseError do ShowMessage(E.Message); end;` — catch specific exceptions; use `raise` to re-raise; define custom exceptions inheriting from `Exception`.
9. Use units for modular code: `unit MyUnit; interface ... implementation ... end.` — declare public types/functions in `interface`, private in `implementation`; minimize `uses` clauses to reduce coupling.
10. For REST APIs: use `TRESTClient`, `TRESTRequest`, `TRESTResponse` components — or `TNetHTTPClient` for lower-level control; parse JSON with `TJSONObject` from `System.JSON`.
11. Write unit tests with DUnitX: `[Test] procedure TestCalculation; begin Assert.AreEqual(4, Add(2, 2)); end;` — use `[Setup]` and `[TearDown]` attributes for fixture lifecycle.
12. For Free Pascal/Lazarus: use `{$mode objfpc}` or `{$mode delphi}` for Delphi compatibility; Lazarus provides LCL (cross-platform) equivalent to VCL; FPC supports Linux, macOS, Windows, ARM, and embedded targets.
