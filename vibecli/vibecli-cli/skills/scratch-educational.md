---
triggers: ["Scratch", "Scratch programming", "Scratch MIT", "block programming", "visual programming", "Scratch game", "Scratch animation", "CS education Scratch"]
tools_allowed: ["read_file", "write_file", "bash"]
category: educational
---

# Scratch Programming

When working with Scratch (MIT's visual programming language) for education and prototyping:

1. Organize code by sprite: each sprite should have its own scripts that handle its behavior — use "when green flag clicked" for initialization; "when I receive [message]" for inter-sprite communication via broadcast.
2. Use custom blocks (My Blocks) to avoid duplicate code: define `move and bounce` with inputs for speed and direction — custom blocks are Scratch's equivalent of functions; use "run without screen refresh" for performance.
3. Structure game loops with "forever" + "if": `forever { if <key [space] pressed?> then { change y by 10 } }` — use "wait until" for sequential logic; "repeat until" for loops with exit conditions.
4. Use variables and lists for data: "score", "lives", "level" as global variables; use lists for inventories, high scores, or tile maps — "set [variable] to (value)" initializes; "change [variable] by (amount)" increments.
5. Implement collision detection: "if <touching [sprite]?>" for sprite-sprite collision; "if <touching color [#color]?>" for color-based detection; "if <color [#color] is touching [#color]?>" for pixel-precise collision.
6. Create animations with costume switching: "next costume" in a loop with "wait (0.1) seconds" for frame animation; use "switch costume to [name]" for state-based appearance changes (idle, walking, jumping).
7. Use clones for repeated objects: "create clone of [myself]" spawns independent copies; "when I start as a clone" initializes clone behavior; "delete this clone" removes it — use for bullets, particles, enemies, and collectibles.
8. Implement smooth movement: use "glide (secs) to x: y:" for smooth transitions; "point towards [sprite]" + "move (steps)" for following behavior; use sine/cosine from operators for circular motion.
9. Add sound and music: "play sound [name] until done" for sequential; "start sound [name]" for overlapping; use "set [pitch/volume] effect to" for variation — import .wav or .mp3 files; use the sound editor for trimming.
10. Debug with "say" blocks: `say (join [x = ] (x position))` displays variable values visually — use "ask and wait" for input; check the stage size (480x360) and coordinate system (center = 0,0).
11. Share and remix on scratch.mit.edu: use "See Inside" to study other projects; remix to build on others' work; add clear instructions and credits — the Scratch community values sharing and attribution.
12. For educators: align with CS Fundamentals concepts (sequences, loops, conditionals, events, parallelism, operators, data) — use pair programming; start with guided projects, progress to open-ended challenges.
