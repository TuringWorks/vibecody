---
triggers: ["Lisp", "Common Lisp", "Scheme", "Racket", "Emacs Lisp", "SBCL", "CLISP", "S-expression", "macro Lisp", "REPL Lisp"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["sbcl"]
category: lisp
---

# Lisp (Common Lisp / Scheme / Racket)

When writing Lisp code:

1. Master S-expression syntax: `(function arg1 arg2)` — everything is a list; `(+ 1 2)` = 3; `(defun square (x) (* x x))` defines a function; `(let ((x 10) (y 20)) (+ x y))` binds local variables — parentheses denote structure, not precedence.
2. In Common Lisp, use SBCL for performance: `sbcl --load program.lisp` — SBCL compiles to native code; use `(declaim (optimize (speed 3) (safety 0)))` for performance-critical sections; `(time (my-function))` for benchmarking.
3. Use higher-order functions: `(mapcar #'square '(1 2 3 4))` → `(1 4 9 16)`; `(remove-if-not #'evenp '(1 2 3 4))` → `(2 4)`; `(reduce #'+ '(1 2 3 4))` → `10` — `#'` is shorthand for `(function ...)`.
4. Write macros for code generation: `(defmacro when (test &body body) \`(if ,test (progn ,@body)))` — macros transform code at compile time; use backquote (`) for templates, comma (,) for evaluation, comma-at (,@) for splicing.
5. Use CLOS (Common Lisp Object System) for OOP: `(defclass point () ((x :initarg :x :accessor point-x) (y :initarg :y :accessor point-y)))` — CLOS supports multiple dispatch: `(defmethod distance ((p1 point) (p2 point)) ...)`.
6. Handle conditions with the condition system: `(handler-case (risky-operation) (error (e) (format t "Error: ~a" e)))` — more powerful than try/catch: `restart-case` + `invoke-restart` allows callers to choose recovery strategies.
7. For Scheme/Racket: `#lang racket` at the top; `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))` — Racket provides batteries-included: `net/url` for HTTP, `json` for JSON, `web-server` for web apps.
8. Use proper list operations: `(car list)` = first, `(cdr list)` = rest, `(cons x list)` = prepend, `(list 1 2 3)` = construct — use `first`/`rest` for readability in Common Lisp; destructuring: `(destructuring-bind (a b &rest c) list ...)`.
9. Leverage the REPL for interactive development: evaluate expressions, redefine functions, inspect objects live — `(describe 'symbol)` for documentation; `(inspect object)` for interactive exploration; `(trace function-name)` for call tracing.
10. Use packages for namespacing: `(defpackage :my-app (:use :cl :cl-ppcre) (:export :main))` — `:use` imports symbols; `:export` makes them public; `(in-package :my-app)` sets the current package.
11. For web development: use Hunchentoot (web server) + CL-WHO (HTML generation) + Postmodern (PostgreSQL) in Common Lisp; use Racket's built-in `web-server/servlet` for Scheme — both support real-time REPL-driven development.
12. Test with FiveAM (Common Lisp): `(def-test addition () (is (= 4 (+ 2 2))))` — or `rackunit` for Racket: `(check-equal? (add 2 2) 4)` — run tests interactively from the REPL during development.
