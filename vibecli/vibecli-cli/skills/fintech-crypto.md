---
triggers: ["cryptocurrency", "blockchain", "DeFi", "smart contract", "Solidity", "Web3", "wallet", "NFT", "token", "DEX", "staking"]
tools_allowed: ["read_file", "write_file", "bash"]
category: fintech
---

# Fintech Cryptocurrency and Blockchain Development

When working with cryptocurrency, blockchain, and DeFi systems:

1. Write smart contracts following the checks-effects-interactions pattern: validate all conditions first, update state second, and make external calls last to prevent reentrancy attacks that have caused hundreds of millions in losses (e.g., the DAO hack pattern).

2. Optimize gas consumption by packing storage variables (multiple uint128 in one slot), using `calldata` instead of `memory` for read-only function parameters, caching storage reads in local variables, and preferring mappings over arrays for lookups.

3. Implement wallet integration using EIP-1193 (injected provider) with WalletConnect as a fallback; always request the minimum required permissions, handle chain switching (`wallet_switchEthereumChain`), and gracefully degrade when no wallet is detected.

4. Sign transactions client-side and never expose private keys to the backend; use hardware wallet integration (Ledger/Trezor via their SDKs) for high-value operations and implement multi-sig (Safe/Gnosis) for treasury and admin functions.

5. Follow established token standards precisely: ERC-20 for fungible tokens (include `approve`/`transferFrom` with the known race condition mitigated via `increaseAllowance`), ERC-721 for NFTs with proper `tokenURI` metadata, and ERC-1155 for mixed fungible/non-fungible collections.

6. Integrate Chainlink oracles for off-chain data feeds (price, randomness, API calls); always check the oracle response's `updatedAt` timestamp for staleness, validate the `answeredInRound` against `roundId`, and implement fallback oracles for critical price feeds.

7. Protect against MEV (Maximal Extractable Value) by using private mempools (Flashbots Protect), implementing commit-reveal schemes for sensitive operations, adding slippage tolerance parameters to DEX interactions, and using deadline parameters on swap functions.

8. Index blockchain events for your dApp using The Graph (subgraphs) or a self-hosted indexer; define event handlers for all contract events, maintain derived entities for aggregated data, and use `callHandlers` sparingly due to higher indexing costs.

9. Implement comprehensive smart contract testing with unit tests (Foundry/Hardhat), fuzz testing (`forge fuzz`), invariant testing, and fork testing against mainnet state; target 100% branch coverage and test all revert conditions.

10. Design for Layer 2 scaling by deploying on rollups (Optimism, Arbitrum, zkSync); account for L1-L2 message passing latency, implement cross-chain bridges carefully with proper finality checks, and optimize for L2-specific gas pricing (calldata compression).

11. Conduct or commission security audits before mainnet deployment; use automated tools (Slither, Mythril, Echidna) during development, implement timelocks on admin functions, and establish a bug bounty program for production contracts.

12. Use upgradeable proxy patterns (UUPS or Transparent Proxy) only when necessary; initialize state in an `initialize` function (not constructor), protect the `_authorizeUpgrade` function, and maintain storage layout compatibility across versions by never reordering or removing existing storage variables.

13. Implement proper event emission for all state-changing operations; events are the primary mechanism for off-chain systems to track contract activity and are far cheaper than storage reads for historical data retrieval.

14. Handle multi-chain deployments with deterministic addresses using CREATE2, maintain a deployment registry, and use chain-agnostic message protocols (LayerZero, Axelar) for cross-chain communication with proper security validation.
