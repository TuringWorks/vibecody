---
triggers: ["smart contract audit", "Slither", "Mythril", "Echidna", "contract security", "reentrancy attack", "front running", "smart contract vulnerability", "formal verification", "solidity audit"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# Smart Contract Security Auditing

When working with smart contract security:

1. Run Slither as the first pass on every contract: `slither . --filter-paths "node_modules|lib" --json slither-report.json`; review all high/medium findings — common catches include reentrancy, unchecked return values, and dangerous `delegatecall` patterns that manual review often misses.
2. Use Mythril for symbolic execution: `myth analyze src/Contract.sol --solc-json mythril.config.json --execution-timeout 300`; it explores execution paths to find integer overflows, unprotected self-destructs, and arbitrary storage writes that static analysis cannot detect.
3. Write Echidna property tests for invariant checking: define functions prefixed with `echidna_` that return `bool`, e.g., `function echidna_total_supply_invariant() public returns (bool) { return token.totalSupply() <= MAX_SUPPLY; }`; run with `echidna . --contract TestContract --test-mode assertion`.
4. Fuzz with Foundry for targeted vulnerability discovery: use `forge test --fuzz-runs 10000` and write fuzz tests like `function testFuzz_withdrawNeverExceedsBalance(uint256 amount) public` with `vm.assume(amount <= balance)` to bound inputs; check for arithmetic edge cases at `type(uint256).max`.
5. Audit reentrancy by tracing all external calls (`.call`, `.transfer`, `safeTransfer`, ERC777 hooks) and verifying state is updated before each call; use `ReentrancyGuard` but do not rely on it alone — read-only reentrancy through view functions querying stale state is a separate attack vector.
6. Check access control exhaustively: verify every `onlyOwner`/`onlyRole` modifier is applied to admin functions; look for missing access control on `selfdestruct`, `delegatecall`, proxy upgrade functions, and parameter setters — a single unprotected setter can drain the protocol.
7. Identify front-running vulnerabilities in commit-reveal schemes, AMM trades, and liquidations; mitigate with commit-reveal patterns (`keccak256(abi.encodePacked(value, salt))` in commit phase), deadline parameters, and minimum output amounts (slippage protection).
8. Detect gas griefing attacks where malicious contracts consume all forwarded gas in callbacks: use `{gas: fixedAmount}` for external calls when possible, set reasonable gas limits on callback functions, and never assume external calls will succeed — always check return values.
9. Use Certora Prover for formal verification of critical invariants: write rules in CVL (Certora Verification Language) like `rule totalSupplyNeverDecreases { ... }` and run `certoraRun src/Token.sol --verify Token:specs/token.spec` to mathematically prove properties hold across all possible inputs.
10. Verify upgrade safety: ensure new implementation storage layouts are append-only compatible with previous versions using `forge inspect Contract storage-layout`; check that `initializer` functions cannot be called twice, and validate that `_authorizeUpgrade` has proper access control.
11. Follow a systematic audit checklist: (a) access control on all state-changing functions, (b) input validation and bounds checking, (c) reentrancy on all external calls, (d) oracle manipulation resistance, (e) flash loan attack vectors, (f) integer arithmetic edge cases, (g) event emission completeness, (h) upgrade safety, (i) denial-of-service via unbounded loops, (j) proper use of `msg.sender` vs `tx.origin`.
12. Document findings with severity ratings (Critical/High/Medium/Low/Informational), include proof-of-concept exploit code in Foundry test format, and provide specific remediation code — a finding without a clear fix and PoC is incomplete; generate the final report with contract addresses, commit hashes, and scope boundaries.
