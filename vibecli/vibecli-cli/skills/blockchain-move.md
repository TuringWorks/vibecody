---
triggers: ["Move", "move language", "Sui", "Aptos", "move module", "move resource", "sui object", "aptos move", "move prover"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# Move Language (Sui/Aptos)

When working with Move smart contracts:

1. Structure Move modules with `module package_addr::module_name { ... }` containing struct definitions, public functions, and friend declarations; every module must be published as part of a package with a `Move.toml` manifest specifying dependencies and named addresses.
2. Understand Move's four abilities: `copy` (value can be duplicated), `drop` (value can be discarded), `store` (value can be saved in global storage), `key` (value can be used as a global storage key/Sui object) — a struct with no abilities is a linear type that must be explicitly consumed, preventing resource leaks.
3. Define resources on Aptos with `struct Coin has key, store { value: u64 }` and manage them with `move_to<T>(&signer, resource)`, `borrow_global<T>(addr)`, `borrow_global_mut<T>(addr)`, and `move_from<T>(addr)` — the type system guarantees resources cannot be copied or dropped accidentally.
4. Use Sui's object model: define objects with `struct MyObject has key, store { id: UID, data: u64 }` and create with `object::new(ctx)`; objects can be owned (single address), shared (`transfer::share_object`), or immutable (`transfer::freeze_object`) — choose ownership based on concurrency needs.
5. Implement entry functions for transaction entry points: `public entry fun mint(ctx: &mut TxContext)` on Sui or `public entry fun mint(account: &signer)` on Aptos; these are the only functions callable directly from transactions — use `public fun` for functions called by other modules.
6. Write generics for reusable logic: `public fun transfer<T: key + store>(obj: T, recipient: address)` with ability constraints; use phantom type parameters `struct Coin<phantom T>` when the type is only used for type-level distinction without storing values of type `T`.
7. Implement coin/token standards: on Sui use `coin::create_currency<T>(witness, decimals, symbol, name, description, icon_url, ctx)` with a one-time witness pattern; on Aptos use `coin::initialize<CoinType>(account, name, symbol, decimals, monitor_supply)` and register with `coin::register<CoinType>(account)`.
8. Build Programmable Transaction Blocks (PTBs) on Sui for atomic multi-step operations: compose `moveCall`, `splitCoins`, `mergeCoins`, and `transferObjects` commands in a single transaction using the TypeScript SDK — PTBs can chain up to 1024 commands without deploying custom contracts.
9. Use Move Prover for formal verification: write specification blocks `spec module { invariant forall addr: address: global<Balance>(addr).value >= 0; }` and run `aptos move prove` or `sui move prove` to mathematically verify invariants hold across all possible execution paths.
10. Handle events for off-chain indexing: on Sui use `event::emit(MyEvent { field: value })` with `struct MyEvent has copy, drop { field: u64 }`; on Aptos use `event::emit(MyEvent { field: value })` — both chains provide event subscription APIs for building indexers and frontends.
11. Test Move code with built-in test framework: annotate test functions with `#[test]` and helper functions with `#[test_only]`; use `#[expected_failure(abort_code = E_NOT_AUTHORIZED)]` to test error cases; run with `sui move test` or `aptos move test --filter test_name`.
12. Deploy and upgrade packages: on Sui use `sui client publish --gas-budget 100000000` and upgrade with `sui client upgrade`; on Aptos use `aptos move publish --named-addresses myaddr=default`; both support package upgrade policies — set immutable after stabilization to build user trust.
