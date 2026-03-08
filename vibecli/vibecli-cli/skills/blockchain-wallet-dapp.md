---
triggers: ["dApp", "dapp", "MetaMask", "WalletConnect", "wagmi", "RainbowKit", "wallet connect", "web3 frontend", "EIP-712", "ENS"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: blockchain
---

# Wallet Integration and dApp Frontend

When working with dApp frontends and wallet integration:

1. Use wagmi + viem as the foundation: `npm install wagmi viem @tanstack/react-query`; configure with `createConfig({ chains: [mainnet, sepolia], transports: { [mainnet.id]: http(RPC_URL) } })` and wrap your app in `<WagmiProvider config={config}><QueryClientProvider>...</QueryClientProvider></WagmiProvider>`.
2. Add RainbowKit for a polished wallet connection UI: `npm install @rainbow-me/rainbowkit`; wrap with `<RainbowKitProvider>` and use `<ConnectButton />` component — it handles MetaMask, WalletConnect, Coinbase Wallet, and 100+ wallets with a single integration and built-in chain switching.
3. Read contract data with wagmi hooks: `const { data } = useReadContract({ address, abi, functionName: 'balanceOf', args: [userAddress] })` — this auto-refreshes on new blocks; for multiple reads, use `useReadContracts` with a batch of contract calls to reduce RPC requests.
4. Write to contracts with `useWriteContract`: `const { writeContract, data: hash } = useWriteContract()` then `writeContract({ address, abi, functionName: 'transfer', args: [to, amount] })`; track confirmation with `useWaitForTransactionReceipt({ hash })` and show pending/success/error states in the UI.
5. Implement EIP-712 typed data signing for gasless operations: `const { signTypedData } = useSignTypedData()` with domain separator `{ name, version, chainId, verifyingContract }` and typed message structure — verify signatures on-chain with `ECDSA.recover(hash, signature)` for permit/meta-transaction patterns.
6. Resolve ENS names with `useEnsName({ address })` to display human-readable names and `useEnsAddress({ name: 'vitalik.eth' })` for lookups; show ENS avatars with `useEnsAvatar({ name })`; always display the full address as a fallback and allow users to input either ENS names or addresses.
7. Handle multi-chain switching gracefully: use `useSwitchChain()` to prompt network changes and `useChainId()` to detect the current chain; display chain-specific data and contracts — maintain a config mapping `{ [chainId]: { contractAddress, explorerUrl, rpcUrl } }` for each supported network.
8. Implement proper error handling for wallet interactions: catch `UserRejectedRequestError` (user denied), `ChainNotConfiguredError` (wrong network), `ContractFunctionRevertedError` (on-chain revert) — parse revert reasons from custom errors using `decodeErrorResult({ abi, data: error.data })` and show user-friendly messages.
9. Manage transaction lifecycle UX: show a pending toast on submission with the tx hash linked to the block explorer, update to confirmed/failed after receipt, and use optimistic updates for balances; implement a transaction history panel using `useWatchPendingTransactions` or local storage.
10. Support mobile wallets with WalletConnect deep links: configure `walletConnectProjectId` in wagmi config; test on mobile by scanning QR codes; handle the mobile browser flow where MetaMask opens an in-app browser — use `window.ethereum` detection to skip WalletConnect when already in a wallet browser.
11. Secure the dApp frontend: never expose private keys in client code; validate all contract return data before displaying; implement Content Security Policy headers to prevent XSS; use `checksumAddress` from viem to normalize addresses; rate-limit RPC calls with `batch: { multicall: true }` in wagmi transport config.
12. Optimize dApp performance: use wagmi's built-in caching with `@tanstack/react-query` stale times; batch contract reads with multicall; lazy-load wallet connection components; prefetch contract data for likely user actions; implement skeleton loading states for on-chain data and use WebSocket transports (`webSocket(WS_URL)`) for real-time updates instead of polling.
