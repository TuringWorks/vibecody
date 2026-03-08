---
triggers: ["DeFi contract", "AMM contract", "flash loan", "ERC4626", "Chainlink oracle", "liquidity pool contract", "yield vault", "governance contract", "DAO voting", "Uniswap"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# DeFi Smart Contract Patterns

When working with DeFi smart contracts:

1. Implement AMMs using the constant product formula `x * y = k`: on each swap, calculate `amountOut = (reserveOut * amountIn * 997) / (reserveIn * 1000 + amountIn * 997)` to include a 0.3% fee; always update reserves after transfers and emit `Swap` events for indexers.
2. Build lending protocols with a shares-based accounting model: track user deposits as shares of a pool rather than raw amounts — `shares = depositAmount * totalShares / totalAssets` — this naturally handles interest accrual as `totalAssets` grows from borrower repayments.
3. Implement flash loans by following the EIP-3156 standard: expose `flashLoan(receiver, token, amount, data)`, transfer tokens to the receiver, call `onFlashLoan` callback, then verify repayment of `amount + fee` in the same transaction; revert if the balance check fails.
4. Use ERC4626 for tokenized vaults: inherit OpenZeppelin's `ERC4626` and implement `totalAssets()` to return the vault's underlying token balance plus any deployed strategy yield; users deposit with `deposit(assets, receiver)` and get proportional shares automatically.
5. Integrate Chainlink oracles safely: call `latestRoundData()` and validate that `answer > 0`, `updatedAt > block.timestamp - MAX_STALENESS`, and `answeredInRound >= roundId`; use multiple oracle sources and implement a fallback mechanism to handle oracle failures.
6. Prevent price manipulation by using TWAPs (time-weighted average prices) instead of spot prices for critical calculations; for Uniswap V3, use `OracleLibrary.consult(pool, twapInterval)` with a 30-minute window minimum to resist single-block manipulation.
7. Build governance with OpenZeppelin Governor: configure `votingDelay` (1 day), `votingPeriod` (1 week), and `proposalThreshold` (token amount to propose); use `TimelockController` with a 48-hour delay so token holders can exit before contentious proposals execute.
8. Implement staking contracts with a rewards-per-token accumulator pattern: maintain `rewardPerTokenStored += (rewardRate * elapsed * 1e18) / totalStaked` and per-user `rewards[user] += balance[user] * (rewardPerToken - userRewardPerTokenPaid[user]) / 1e18` — this avoids iterating over all stakers.
9. Use fixed-point math with sufficient precision: multiply before dividing, use `1e18` as the precision base for most calculations, and apply `mulDiv(x, y, denominator)` from OpenZeppelin's `Math` library to avoid intermediate overflow in `uint256` arithmetic.
10. Protect against reentrancy in DeFi by following checks-effects-interactions strictly: update balances and shares before transferring tokens; use `ReentrancyGuard` on all external-facing deposit/withdraw/swap functions; be especially careful with ERC777 tokens that have transfer hooks.
11. Implement timelocked admin functions for parameter changes: wrap fee updates, oracle address changes, and strategy migrations in a `TimelockController` so users have time to react; emit `ParameterChangeQueued` events with the execution timestamp.
12. Test DeFi contracts with fork tests against real protocol state: `forge test --fork-url $MAINNET_RPC` to interact with deployed Uniswap/Aave/Chainlink contracts; test edge cases like zero liquidity, rounding errors at extreme prices, and sandwich attack scenarios using `vm.roll` and `vm.warp`.
