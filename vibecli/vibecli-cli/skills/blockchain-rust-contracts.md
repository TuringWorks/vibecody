---
triggers: ["CosmWasm", "cosmwasm", "NEAR contract", "ink!", "substrate contract", "rust smart contract", "cosmos SDK", "NEAR SDK", "wasm contract"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: blockchain
---

# Rust Smart Contracts (CosmWasm/NEAR/Ink!)

When working with Rust smart contracts:

1. Structure CosmWasm contracts with three entry points: `instantiate(deps, env, info, msg)` for initialization, `execute(deps, env, info, msg)` for state-changing operations, and `query(deps, env, msg)` for reads — each takes typed message enums (`InstantiateMsg`, `ExecuteMsg`, `QueryMsg`) defined with `#[cw_serde]`.
2. Manage CosmWasm storage with `cw_storage_plus`: use `Item<T>` for single values (`const CONFIG: Item<Config> = Item::new("config")`) and `Map<K, V>` for key-value stores (`const BALANCES: Map<&Addr, Uint128> = Map::new("balances")`) — always validate addresses with `deps.api.addr_validate(&addr)`.
3. Build NEAR contracts with `#[near(contract_state)]` struct and `#[near]` impl blocks: mark view functions with `&self` and mutable functions with `&mut self`; use `#[init]` for the constructor, `#[payable]` for functions accepting NEAR tokens, and `#[private]` for callback-only functions.
4. Handle NEAR cross-contract calls with promises: `Promise::new(account_id).function_call(method, args, deposit, gas)` and chain with `.then(Self::ext(env::current_account_id()).callback_method())`; always handle failure in callbacks by checking `env::promise_results_count()` and `env::promise_result(0)`.
5. Write Ink! contracts for Substrate chains with `#[ink::contract]` module containing `#[ink(storage)]` struct, `#[ink(constructor)]` for `new()`, `#[ink(message)]` for callable functions, and `#[ink(event)]` for events — compile with `cargo contract build --release` to generate `.contract` bundle.
6. Implement token standards per ecosystem: CosmWasm uses CW20 (fungible) and CW721 (NFT) specifications — import `cw20-base` as a dependency or implement the `Cw20ExecuteMsg` interface; NEAR uses NEP-141 (fungible) and NEP-171 (NFT); Ink! follows PSP22 (fungible) and PSP34 (NFT) from OpenBrush.
7. Handle gas/compute metering: CosmWasm charges per operation with configurable gas limits in `wasmd`; NEAR charges in TGas (1 TGas = 10^12 gas units) — prepay gas on function calls and refund unused; Ink! uses Substrate's weight system — annotate with `#[ink(message, payable, selector = 0x...)]`.
8. Test contracts with native test frameworks: CosmWasm uses `cw_multi_test` with `App::new()` to simulate a full chain environment including bank and staking modules; NEAR uses `near_workspaces` for sandbox testing with `Worker::sandbox().await`; Ink! provides `#[ink::test]` for off-chain unit tests.
9. Implement cross-contract calls in CosmWasm using `WasmMsg::Execute { contract_addr, msg, funds }` as a submessage with `SubMsg::reply_on_success(msg, REPLY_ID)` and handle responses in `reply(deps, env, msg)` — this pattern provides atomicity and error handling for composed operations.
10. Optimize Wasm binary size: add `[profile.release] opt-level = "z", lto = true, codegen-units = 1, strip = true` to `Cargo.toml`; for CosmWasm run `cargo wasm` then `cosmwasm-check target/wasm32-unknown-unknown/release/contract.wasm` to verify compatibility; target < 800KB for deployment.
11. Deploy CosmWasm contracts in two steps: `wasmd tx wasm store contract.wasm --from wallet --gas auto` to upload code (returns code_id), then `wasmd tx wasm instantiate $CODE_ID '{"owner":"addr"}' --label "my-contract" --admin $ADMIN --from wallet` to create an instance — separate code from instances enables upgrades.
12. Secure Rust contracts: validate all input addresses and amounts (reject zero amounts, check address formats); use `Uint128`/`Uint256` instead of native integers to prevent overflow; implement admin-only functions with ownership checks; add migrate entry points for CosmWasm contracts with `#[entry_point] pub fn migrate(deps, env, msg)` for upgrade paths.
