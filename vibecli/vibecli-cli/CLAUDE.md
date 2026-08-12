# VibeCLI CLI crate — guidelines

### Module declaration pattern

In `vibecli/vibecli-cli/src/`, both `lib.rs` (`pub mod foo;`) and `main.rs` (`mod foo;`) must declare a module before it can be used in its respective crate artifact. When adding a new `.rs` file, register it in the crate(s) that **consume** it — and only those.

**Declaring in `main.rs` a module the binary never references is not free.** `lib.rs` and `main.rs` are separate crate artifacts, so a `mod foo;` in `main.rs` compiles `foo.rs` a *second* time, into the binary, whether or not anything there uses it. 204 modules (~179 kloc) had accumulated that way; dropping their `main.rs` declarations took `cargo check -p vibecli` from **150 s to 34 s** with no other change. `pub mod` in `lib.rs` alone keeps a module fully available to the library, its tests, and every other crate. Add the `main.rs` line when `main.rs` (or a sibling binary module) actually names the module — not by reflex.
