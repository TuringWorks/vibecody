---
triggers: ["Ada", "SPARK", "Ada 2012", "Ada 2022", "GNAT", "Ravenscar", "Jorvik", "Ada tasking", "Ada safety", "SPARK formal verification", "Ada avionics", "Ada defense", "pragma Restrictions"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gnatmake"]
category: safety-critical
---

# Ada & SPARK for Safety-Critical Systems

When writing Ada/SPARK code for aerospace, defense, and safety-critical systems:

1. Use SPARK subset for highest-assurance code: SPARK restricts Ada to a formally verifiable subset — no access types (pointers), no exceptions in SPARK bodies, no tasks (use Ravenscar profile instead), deterministic execution — GNATprove verifies absence of runtime errors statically.
2. Write SPARK contracts on all subprograms: `function Sqrt (X : Float) return Float with Pre => X >= 0.0, Post => Sqrt'Result >= 0.0 and Sqrt'Result ** 2 <= X + 0.001` — contracts are checked by GNATprove at compile time and optionally at runtime.
3. Use `pragma SPARK_Mode (On)` at package level to enable SPARK checking — mix SPARK and full Ada in the same project: SPARK for safety-critical cores, full Ada for I/O and UI; mark non-SPARK bodies with `pragma SPARK_Mode (Off)`.
4. Prove absence of runtime errors with GNATprove: buffer overflow, integer overflow, division by zero, array index out of bounds, null pointer dereference, numeric range violations — these are eliminated statically, not caught at runtime.
5. Use strong typing aggressively: `type Altitude_Feet is range 0 .. 60_000; type Speed_Knots is range 0 .. 600;` — the compiler prevents mixing altitudes with speeds; constraint checks catch out-of-range values at assignment.
6. Apply the Ravenscar profile for real-time tasking: `pragma Profile (Ravenscar)` restricts Ada's full tasking model to a statically analyzable, deterministic subset — fixed priority, no dynamic task creation, no abort, protected objects for synchronization.
7. Use Jorvik profile (Ada 2022) when Ravenscar is too restrictive: allows multiple entries per protected object and relative delay statements — still deterministic and analyzable, but more flexible for complex real-time systems.
8. Define data representations with representation clauses for hardware interfaces:
   ```ada
   type Status_Register is record
     Ready : Boolean;
     Error : Boolean;
     Mode  : Mode_Type;
   end record;
   for Status_Register use record
     Ready at 0 range 0 .. 0;
     Error at 0 range 1 .. 1;
     Mode  at 0 range 2 .. 4;
   end record;
   for Status_Register'Size use 8;
   ```
9. Use generic packages for reusable, type-safe components: `generic type Element is private; package Bounded_Stack is ... end;` — instantiate with `package Altitude_Stack is new Bounded_Stack (Altitude_Feet);` — generics are resolved at compile time with no runtime overhead.
10. Handle errors with return codes, not exceptions, in SPARK code: use discriminated records `type Result (OK : Boolean := True) is record case OK is when True => Value : Data; when False => Error : Error_Code; end case; end record;` — pattern matches force callers to handle both cases.
11. Use `pragma Restrictions` to enforce project-wide constraints: `pragma Restrictions (No_Allocators, No_Recursion, No_Secondary_Stack, No_Exception_Handlers, No_IO)` — the compiler rejects code violating these restrictions.
12. Compile with GNAT and all validity checks: `-gnatVa` enables all validity checks; `-gnatp` suppresses checks (never use in safety code); `-gnata` enables assertions; `-gnatwe` treats warnings as errors — use `-O2 -gnatn` for production with inlining.
13. Write unit tests with AUnit: `procedure Test_Altitude (T : in out Test) is begin Assert (To_Meters (Altitude_Feet'(1000)) = 304, "1000ft = 304m"); end;` — run with `gnattest` for automatic harness generation from package specs.
14. Use Ada's native concurrency for multi-core systems: protected objects for shared data (compiler-enforced mutual exclusion), rendezvous for synchronous communication, entry barriers for condition synchronization — no manual mutex/lock management.
15. For DO-178C certification: Ada/SPARK's strong typing, contract-based programming, and GNATprove can satisfy DO-333 (formal methods supplement) objectives — formal proofs can replace some testing objectives, reducing certification cost for DAL A software.
