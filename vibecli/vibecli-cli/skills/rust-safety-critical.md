---
triggers: ["Rust safety critical", "Ferrocene", "Rust automotive", "Rust aerospace", "Rust embedded safety", "Rust DO-178", "Rust ISO 26262", "Rust IEC 61508", "Rust MISRA", "no_std safety", "Rust certification"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: safety-critical
---

# Rust for Safety-Critical Systems

When using Rust for safety-critical development (automotive, aerospace, industrial, medical):

1. Use the Ferrocene toolchain for certified Rust: Ferrocene is a qualified Rust compiler (ISO 26262 ASIL D, IEC 61508 SIL 4) from Ferrous Systems — it provides a Ferrocene Language Specification and qualification documentation required for safety certification.
2. Use `#![no_std]` for bare-metal safety targets: eliminates dependency on the standard library and OS; use `#![no_main]` with a custom entry point; link against `core` and `alloc` (if heap is permitted at init) — no implicit allocations in the hot path.
3. Prohibit dynamic allocation after initialization: use `#![cfg_attr(not(test), deny(unused_allocation))]`; pre-allocate all buffers at startup; use `heapless` crate for fixed-capacity `Vec`, `String`, `Queue`, `Map` backed by stack/static memory — no fragmentation risk.
4. Ban `unsafe` in application code: restrict `unsafe` to HAL (Hardware Abstraction Layer) and driver modules; mark unsafe modules with `#![deny(unsafe_code)]` at the crate root and `#[allow(unsafe_code)]` only on justified modules — every `unsafe` block requires a SAFETY comment documenting the invariant.
5. Leverage Rust's ownership model as a safety mechanism: no use-after-free, no double-free, no data races by construction — these eliminate entire classes of vulnerabilities without runtime overhead; the borrow checker is a compile-time proof of memory safety.
6. Use `no_panic` or prove absence of panics: annotate functions with `#[no_panic]` to compile-error if the compiler cannot prove panic-freedom; alternatively, use `panic = "abort"` and verify no panic paths exist with `cargo-geiger` or manual review.
7. Avoid panicking APIs in safety code: never use `unwrap()`, `expect()`, array indexing `[i]`, or integer overflow in default mode — use `get()`, `checked_add()`, `saturating_mul()`, `Result`/`Option` with explicit error handling via `?` operator.
8. Configure integer overflow behavior: `[profile.release] overflow-checks = true` to detect overflow in release builds; or use `Wrapping<T>`, `Saturating<T>`, or `checked_*` methods explicitly — silent wrap-around in safety code is unacceptable.
9. Use `defmt` for embedded logging and `probe-rs` for debug: `defmt::info!("altitude: {}", alt)` — defmt is highly efficient (deferred formatting, wire-size encoding); logs are essential for post-incident analysis in safety systems.
10. Apply MISRA-like restrictions with Clippy lints: `#![deny(clippy::all, clippy::pedantic, clippy::nursery)]`; additionally enable `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::indexing_slicing` — treat all lint warnings as errors.
11. Write tests at every level: `#[cfg(test)]` unit tests for logic; integration tests in `tests/` for module interaction; use `defmt-test` for on-target testing; achieve MC/DC coverage for certified code — Rust's type system does not replace testing.
12. Use `embedded-hal` traits for portable hardware abstraction: `impl InputPin for GpioPin { fn is_high(&self) -> Result<bool, Error> { ... } }` — swap implementations between real hardware and mocks/simulators without changing application code.
13. Use `RTIC` (Real-Time Interrupt-driven Concurrency) for real-time embedded Rust: `#[task(binds = TIM2, priority = 2, shared = [sensor_data])] fn timer_handler(ctx: timer_handler::Context) { ... }` — statically verified priority ceiling protocol, no deadlocks by construction.
14. For formal verification: use `Kani` (Rust model checker by AWS) to prove absence of panics, overflows, and assertion violations: `#[kani::proof] fn verify_altitude_calc() { let alt: u32 = kani::any(); kani::assume(alt <= 60000); assert!(convert(alt) <= MAX_METERS); }`.
15. Document safety rationale per the applicable standard: for each module, document the Safety Requirement it satisfies, the ASIL/SIL level, assumptions, and verification evidence — Rust's compiler guarantees provide evidence that many classes of defects are structurally impossible.
16. For DO-178C with Rust: Ferrocene provides the Tool Qualification Data (TQD) needed for compiler qualification; use the Ferrocene Language Specification as the language reference for planning and review; pair with formal verification (Kani/Creusot) for DO-333 credit.
