---
triggers: ["tokenomics", "token economics", "ERC20 token", "vesting schedule", "bonding curve", "token burn", "token distribution", "liquidity bootstrapping", "token launch"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# Token Design and Economics

When working with token economics:

1. Create ERC20 tokens by inheriting OpenZeppelin's `ERC20` with extensions: `ERC20Burnable` for deflationary mechanics, `ERC20Permit` (EIP-2612) for gasless approvals, and `ERC20Votes` for governance — set total supply in the constructor with `_mint(msg.sender, initialSupply)`.
2. Implement linear vesting schedules with a cliff: store `startTime`, `cliffDuration`, `vestingDuration`, and `totalAmount` per beneficiary; calculate claimable as `totalAmount * (block.timestamp - startTime) / vestingDuration - alreadyClaimed` only after the cliff has passed; use OpenZeppelin's `VestingWallet`.
3. Design bonding curves for continuous token pricing: linear `price = basePrice + slope * supply`, exponential `price = basePrice * e^(k * supply)`, or sigmoid curves; implement in Solidity with `bancorFormula` math using fixed-point arithmetic to avoid precision loss at extreme supply ranges.
4. Implement burn mechanics by calling `_burn(msg.sender, amount)` in a dedicated `burn` function or automatically on transfers with a burn fee: `uint256 burnAmount = amount * burnBasisPoints / 10000; _burn(from, burnAmount); _transfer(from, to, amount - burnAmount)`.
5. Build fee-on-transfer tokens by overriding `_update` (OZ v5) or `_transfer` (OZ v4): deduct a percentage for treasury/burn/redistribution; be aware this breaks compatibility with many DeFi protocols — always provide a fee-exempt list for DEX pairs and protocol contracts.
6. Create reflection tokens that redistribute fees proportionally to holders without iterating: use a reflection-to-token ratio (`rTotal / tTotal`), deduct from the reflected total on each transfer, and convert between reflected and actual amounts — this O(1) approach scales to unlimited holders.
7. Implement liquidity bootstrapping pools (LBPs) for fair token launches: deploy a Balancer weighted pool starting at 95/5 (token/collateral) that shifts to 50/50 over 24-72 hours — the declining price curve discourages bots and allows organic price discovery.
8. Design token distribution with typical allocations: 30-40% community/ecosystem, 15-20% team (4-year vest, 1-year cliff), 10-15% investors (2-year vest, 6-month cliff), 5-10% treasury, 5-10% initial liquidity; enforce all vesting on-chain, never trust off-chain agreements.
9. Implement anti-whale mechanisms: set `maxTransactionAmount` and `maxWalletBalance` as percentages of total supply (typically 1-2%); enforce in `_update`/`_transfer` with `require(balanceOf(to) + amount <= maxWalletBalance)` and exempt the owner and liquidity pool addresses.
10. Build snapshot-based governance: use OpenZeppelin's `ERC20Votes` with `_delegate` for delegation, and `ERC20VotesComp` for Compound-style governance; take voting snapshots with `_snapshot()` so token transfers after proposal creation do not affect voting power.
11. Model tokenomics before deployment: simulate supply, emission, burn rate, and price scenarios in a spreadsheet or Python script; calculate fully diluted valuation (FDV), circulating supply over time, and inflation rate — ensure emissions do not outpace protocol revenue to maintain token value.
12. Launch tokens safely: deploy the token contract first, add initial liquidity to a DEX pair by calling `router.addLiquidity` with both token and ETH/stablecoin, lock LP tokens using a timelock contract (minimum 6 months), and renounce ownership of the token contract only after all parameters are finalized.
