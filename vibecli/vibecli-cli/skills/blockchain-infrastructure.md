---
triggers: ["blockchain node", "Geth", "Reth", "Erigon", "The Graph", "subgraph", "Ponder", "Flashbots", "MEV", "blockchain indexing", "IPFS pinning", "validator node"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# Blockchain Node and Infrastructure

When working with blockchain infrastructure:

1. Choose the right execution client: Geth (reference Go implementation, ~1TB archive, ~550GB snap sync), Reth (Rust, fastest sync, ~1TB archive with excellent performance), or Erigon (Go, most storage-efficient at ~2TB archive with flat DB) — all support the same JSON-RPC API, so choose based on disk/performance tradeoffs.
2. Run a full node with `reth node --datadir /data/reth --http --http.api eth,net,web3,debug,trace --ws` or `geth --http --http.api eth,net,web3,debug --syncmode snap --datadir /data/geth` — enable `debug` and `trace` APIs only on internal networks, and bind to `127.0.0.1` (not `0.0.0.0`) to prevent unauthorized access.
3. Use RPC providers (Alchemy, Infura, QuickNode) for development and as fallbacks: configure multiple providers with automatic failover in your application — `const client = createPublicClient({ transport: fallback([http(PRIMARY_RPC), http(BACKUP_RPC)]) })` with viem's built-in fallback transport.
4. Index blockchain data with The Graph: define a `subgraph.yaml` with data sources mapping contract events to handlers, write AssemblyScript mappings in `src/mapping.ts` that create/update entities, then deploy with `graph deploy --studio my-subgraph` — use `startBlock` to avoid indexing from genesis.
5. Use Ponder for TypeScript-native indexing: configure `ponder.config.ts` with contract ABIs and addresses, write event handlers in `src/index.ts` like `ponder.on("Token:Transfer", async ({ event, context }) => { ... })` — Ponder provides hot reloading, type safety, and built-in GraphQL API without AssemblyScript.
6. Set up event indexing for custom backends: subscribe to contract events with `publicClient.watchContractEvent({ address, abi, eventName, onLogs })` for real-time processing; for historical data, use `getLogs({ address, fromBlock, toBlock })` with block range chunking (max 2000-10000 blocks per query depending on RPC provider).
7. Operate archive nodes for historical state queries: archive nodes store all historical state trie data (~12TB+ for Ethereum); use Erigon or Reth for the most efficient archive storage; access historical state with `eth_call` at specific block numbers using the `blockNumber` parameter.
8. Protect against MEV with Flashbots: submit bundles via `flashbots_sendBundle` to `relay.flashbots.net` with transactions ordered to avoid sandwich attacks; use `eth_sendPrivateTransaction` for single transactions that skip the public mempool; implement MEV-Share for users to capture their own MEV.
9. Query blockchain analytics with Dune: write SQL against decoded tables like `SELECT * FROM erc20_ethereum.evt_Transfer WHERE "to" = 0x...`; for real-time data, use Dune's API with `GET /api/v1/query/{query_id}/results`; Nansen provides labeled addresses and smart money tracking for on-chain intelligence.
10. Run IPFS infrastructure for NFT/metadata storage: `ipfs daemon` with pinning service integration; use `ipfs pin remote add --service=pinata $CID` to replicate across providers; for production, run a dedicated IPFS cluster with `ipfs-cluster-service` for redundancy and automatic garbage collection management.
11. Operate Chainlink nodes for oracle services: run `chainlink node start` with PostgreSQL backend; register as an operator on the Chainlink network; implement External Adapters for custom data feeds; for consumers, use `AggregatorV3Interface(priceFeedAddress).latestRoundData()` with staleness checks.
12. Set up validator nodes: for Ethereum, run both execution (Geth/Reth) and consensus (Lighthouse/Prysm/Teku) clients with 32 ETH staked; use `ethdo validator exit` for voluntary exits; monitor with `prometheus` + `grafana` dashboards tracking attestation effectiveness, sync committee participation, and proposal slots — set alerts for missed attestations.
