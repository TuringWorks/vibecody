---
triggers: ["Logo", "Logo programming", "turtle graphics", "UCBLogo", "NetLogo", "StarLogo", "Logo language"]
tools_allowed: ["read_file", "write_file", "bash"]
category: educational
---

# Logo Programming

When working with Logo (turtle graphics and educational computing):

1. Logo uses turtle graphics as its core metaphor: `FORWARD 100` (move forward), `RIGHT 90` (turn right), `PENUP`/`PENDOWN` (drawing control), `HOME` (return to center) — the turtle is a cursor that draws as it moves.
2. Define procedures: `TO SQUARE :size REPEAT 4 [FORWARD :size RIGHT 90] END` — `:size` is a parameter (colon prefix); `REPEAT n [commands]` for loops; procedures are Logo's functions.
3. Create recursive patterns: `TO SPIRAL :size :angle IF :size > 200 [STOP] FORWARD :size RIGHT :angle SPIRAL :size + 2 :angle END` — recursion is natural in Logo; use `STOP` or `OUTPUT` for base cases.
4. Draw regular polygons: `TO POLYGON :sides :size REPEAT :sides [FORWARD :size RIGHT 360 / :sides] END` — the exterior angle sum is always 360 degrees; `POLYGON 6 50` draws a hexagon.
5. Use variables: `MAKE "count 0` sets a variable; `:count` reads it; `MAKE "count :count + 1` increments — Logo variables are dynamically scoped by default; use `LOCAL` for local scope.
6. List operations: `FIRST [a b c]` → `a`; `BUTFIRST [a b c]` → `[b c]`; `FPUT "x [a b]` → `[x a b]`; `SENTENCE [a] [b c]` → `[a b c]` — Logo's list processing capabilities come from its Lisp heritage.
7. For NetLogo (agent-based modeling): `breed [turtles turtle]`; `ask turtles [forward 1 right random 60]` — NetLogo extends Logo for simulating complex systems: epidemics, ecosystems, traffic, and social networks.
8. Color and pen control: `SETPENCOLOR [255 0 0]` (RGB red); `SETPENSIZE 3` (line width); `FILL` to flood-fill enclosed areas; `SETBACKGROUND` to change canvas color.
9. Modern Logo implementations: UCBLogo (Brian Harvey's educational version), NetLogo (agent-based), Papert (web-based), Turtle Academy (online) — use for teaching computational thinking, geometry, and programming fundamentals.
10. Educational approach: Logo was designed by Seymour Papert for constructionist learning — students learn by building; start with direct commands, progress to procedures, then recursion; Logo makes abstract math concepts tangible through turtle geometry.
