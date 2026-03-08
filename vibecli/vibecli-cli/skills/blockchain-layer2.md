---
triggers: ["Layer 2", "L2", "optimistic rollup", "ZK rollup", "zkSync", "StarkNet", "OP Stack", "Polygon CDK", "Arbitrum", "Base chain", "EIP-4844"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# Layer 2 and Scaling Solutions

When working with Layer 2 solutions:

1. Choose the right L2 type: optimistic rollups (Arbitrum, Base, OP Mainnet) offer EVM equivalence with 7-day withdrawal delays; ZK rollups (zkSync Era, StarkNet, Polygon zkEVM) provide faster finality with validity proofs but may have EVM compatibility gaps — check opcode support before deploying.
2. Deploy to OP Stack chains (Base, OP Mainnet) using standard Hardhat/Foundry tooling — they are EVM equivalent; set the RPC URL to the L2 endpoint and adjust gas settings: `forge script Deploy.s.sol --rpc-url $BASE_RPC --broadcast --legacy` (some L2s require `--legacy` for pre-EIP-1559 tx format).
3. Deploy to Arbitrum with Foundry: `forge create --rpc-url $ARB_RPC --private-key $KEY src/Contract.sol:Contract`; note that `block.number` on Arbitrum returns the L1 block number — use `ArbSys(address(100)).arbBlockNumber()` for the L2 block number in time-sensitive logic.
4. Build on zkSync Era using `hardhat-zksync` plugins: `npx hardhat compile --network zkSyncMainnet` with `zksolc` compiler; account abstraction is native — deploy smart contract wallets with `IAccountAbstraction` interface for gasless transactions and custom signature validation.
5. Develop for StarkNet using Cairo language: define contracts with `#[starknet::contract]` attribute, implement `#[external(v0)]` functions, and use `felt252` as the native field element type; test with `snforge test` and deploy with `sncast deploy --class-hash $HASH`.
6. Implement cross-chain messaging between L1 and L2: use the native bridge contracts — `L1CrossDomainMessenger.sendMessage(target, data, gasLimit)` on OP Stack, `Inbox.createRetryableTicket` on Arbitrum; always handle message replay protection and verify the cross-domain sender.
7. Leverage EIP-4844 blob transactions for reduced L1 data costs: L2 sequencers post transaction data as blobs (~128KB each) instead of calldata, reducing L2 fees by 10-100x; application developers benefit automatically — no code changes needed, just verify your L2 supports blob posting.
8. Build bridges carefully: lock-and-mint bridges hold assets on L1 and mint representations on L2; use canonical bridges for maximum security; for custom bridges, implement rate limiting, pausability, and multi-sig validation — bridge hacks are the most common source of large DeFi losses.
9. Deploy with Polygon CDK for custom ZK rollup chains: configure the CDK stack with your own sequencer, token, and data availability layer; use the `cdk` CLI to initialize the chain and deploy bridge contracts — this is suitable for app-specific rollups needing dedicated blockspace.
10. Handle L2 gas differences: Arbitrum uses ArbGas with different opcode costs; OP Stack charges L1 data fee + L2 execution fee — call `GasPriceOracle.getL1Fee(txData)` to estimate the L1 component; optimize calldata by compressing inputs since L1 data posting dominates L2 costs.
11. Manage withdrawals from optimistic rollups: initiate with `L2CrossDomainMessenger.sendMessage`, wait the 7-day challenge period, then prove and finalize on L1; for faster exits, use third-party liquidity bridges (Across, Stargate) that front the funds for a fee and claim the withdrawal later.
12. Test L2 contracts with local devnets: `anvil --fork-url $L2_RPC` works for most L2s; for OP Stack, use `op-geth` + `op-node` local devnet; for zkSync, use `era_test_node` (`era_test_node fork mainnet`); always test cross-chain messaging end-to-end on testnets (Sepolia L2s) before mainnet deployment.
