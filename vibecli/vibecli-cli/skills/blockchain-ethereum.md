---
triggers: ["Ethereum", "ethereum", "Hardhat", "hardhat", "Foundry", "forge test", "ethers.js", "viem", "anvil", "cast send", "EVM"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["forge"]
category: blockchain
---

# Ethereum Development Ecosystem

When working with Ethereum development:

1. Initialize Foundry projects with `forge init` for pure Solidity or use Hardhat (`npx hardhat init`) when you need JavaScript/TypeScript task pipelines; for hybrid setups, keep Foundry for testing and Hardhat for deployment scripts via `hardhat-foundry` plugin.
2. Configure `foundry.toml` with `[profile.default] optimizer = true, optimizer_runs = 200, via_ir = true` for production builds; use `anvil --fork-url $RPC_URL --fork-block-number 19000000` to test against mainnet state at a pinned block.
3. Use `cast` for quick chain interactions: `cast call <addr> "balanceOf(address)" <user>` for reads, `cast send <addr> "transfer(address,uint256)" <to> <amount> --rpc-url $RPC --private-key $KEY` for writes, and `cast abi-decode` to parse raw return data.
4. Prefer `viem` over `ethers.js` for new projects — it provides tree-shakable modules, type-safe contract interactions via `getContract()`, and built-in multicall support; use `createPublicClient` for reads and `createWalletClient` for transactions.
5. Estimate gas before sending transactions: `await publicClient.estimateGas({ ... })` with viem or `forge script --estimate-gas`; add a 20% buffer for mainnet and monitor `baseFeePerGas` from the latest block to set `maxFeePerGas` appropriately.
6. Understand the transaction lifecycle: pending (mempool) -> included (block) -> confirmed (N blocks deep); use `waitForTransactionReceipt` with a confirmation count, and handle `TransactionReceiptNotFoundError` for dropped transactions.
7. Protect against MEV by submitting sensitive transactions (large swaps, liquidations) through Flashbots Protect RPC (`https://rpc.flashbots.net`) or MEV Blocker, which routes transactions to block builders privately instead of the public mempool.
8. Write deployment scripts in Foundry: `forge script script/Deploy.s.sol --rpc-url $RPC --broadcast --verify --etherscan-api-key $KEY`; use `vm.startBroadcast()` to batch transactions and `--resume` to retry failed deployments.
9. ABI encode constructor args with `cast abi-encode "constructor(address,uint256)" $TOKEN 1000` or use Foundry's `abi.encode()` in scripts; for proxy deployments, encode the `initialize()` calldata separately with `abi.encodeWithSelector`.
10. Fork mainnet for integration tests: `forge test --fork-url $RPC -vvvv` gives full stack traces; use `deal(address, amount)` to set ETH balances and `deal(token, user, amount)` for ERC20 balances in fork tests without needing token whales.
11. Set up multi-chain deployments by parameterizing RPC URLs and chain IDs in `foundry.toml` profiles (`[profile.sepolia]`, `[profile.mainnet]`); use `forge script --multi` for simultaneous multi-chain deployment with deterministic addresses via CREATE2.
12. Verify contracts on block explorers: `forge verify-contract <address> ContractName --chain mainnet --etherscan-api-key $KEY`; for proxy contracts, also call `forge verify-contract` on the implementation and use `cast call <proxy> "implementation()"` to confirm the implementation address.
