---
triggers: ["Prolog", "logic programming", "SWI-Prolog", "SICStus", "Prolog rules", "Prolog facts", "unification", "backtracking", "constraint logic programming", "Datalog"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["swipl"]
category: prolog
---

# Prolog & Logic Programming

When writing Prolog code:

1. Define facts and rules: `parent(tom, bob).` (fact); `grandparent(X, Z) :- parent(X, Y), parent(Y, Z).` (rule) — Prolog searches by unification and backtracking; variables start with uppercase, atoms with lowercase.
2. Use SWI-Prolog for general development: `swipl -g "main" -t halt program.pl` for scripting; `:- use_module(library(lists)).` for standard libraries; `?- query.` at the interactive prompt for exploration.
3. Write deterministic predicates when possible: use cuts (`!`) sparingly to prune search — green cuts (don't change semantics) are acceptable; red cuts (change logic) make code fragile; prefer `if-then-else`: `(Cond -> Then ; Else)`.
4. Use list processing idioms: `[H|T]` for head/tail destructuring; `member(X, List)` for membership; `append(A, B, C)` for concatenation; `maplist(pred, List)` for applying predicates; `foldl(pred, List, Init, Result)` for accumulation.
5. Use DCG (Definite Clause Grammars) for parsing: `sentence --> noun_phrase, verb_phrase. noun_phrase --> [the], noun.` — translates to difference lists; use `phrase(sentence, Tokens)` to parse; excellent for DSLs and natural language.
6. Leverage constraint logic programming: `use_module(library(clpfd))` for integer constraints — `X in 1..9, Y in 1..9, X + Y #= 10, X #< Y, label([X, Y])` solves constraint satisfaction problems like Sudoku, scheduling, and planning.
7. Use `assertz/retract` for dynamic facts: `assertz(visited(Node))` adds facts at runtime; `retract(visited(Node))` removes them — useful for memoization and state tracking; use `abolish(visited/1)` to clear all.
8. Handle I/O with streams: `read_term(Stream, Term, [])` for Prolog terms; `read_line_to_string(user_input, Line)` for text; `format("~w has ~d items~n", [Name, Count])` for formatted output.
9. Debug with `trace/0` and `spy/1`: `?- trace.` enables step-by-step tracing; `?- spy(predicate_name).` sets breakpoints; press `c` (creep), `s` (skip), `l` (leap) during trace — use `guitracer` for graphical debugging in SWI-Prolog.
10. Organize code in modules: `:- module(graph, [path/3, shortest_path/4]).` exports specific predicates; `:- use_module(graph).` imports them — modules prevent name clashes in large programs.
11. Use `findall/3`, `bagof/3`, `setof/3` for collecting solutions: `findall(X, member(X, [1,2,3,2]), Xs)` gives `[1,2,3,2]`; `setof` removes duplicates and sorts; `aggregate_all(count, pred(X), Count)` for counting.
12. For real applications: use SWI-Prolog's HTTP library for web services (`http_server`), ODBC for databases, pengines for distributed Prolog, and Tau Prolog for browser-based execution.
