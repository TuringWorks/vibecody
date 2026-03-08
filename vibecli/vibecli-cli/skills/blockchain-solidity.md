---
triggers: ["Solidity", "solidity", "smart contract", "ERC20", "ERC721", "ERC1155", "solidity modifier", "solidity event", "pragma solidity", "reentrancy"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["forge"]
category: blockchain
---

# Solidity Smart Contract Development

When working with Solidity smart contracts:

1. Pin the compiler version with `pragma solidity 0.8.24;` (not `^0.8.0`) to ensure deterministic builds and avoid unexpected behavior from compiler upgrades; use the latest stable 0.8.x for built-in overflow checks.
2. Structure contracts with a consistent ordering: state variables, events, errors, modifiers, constructor, external functions, public functions, internal functions, private functions — following the Solidity style guide for readability and auditability.
3. Prefer custom errors (`error InsufficientBalance(uint256 available, uint256 required)`) over `require` strings to save ~50 bytes of deployment gas per error site and provide structured revert data for frontends.
4. Use `memory` for temporary function-local data, `calldata` for read-only external function parameters (saves gas by avoiding copies), and `storage` only when you need to mutate persistent state; never pass `storage` references externally.
5. Apply the checks-effects-interactions pattern: validate inputs first, update state second, make external calls last; complement with `ReentrancyGuard` from OpenZeppelin for any function that transfers ETH or calls untrusted contracts.
6. Emit events for every state change that off-chain systems need to track — events are 375 gas for the LOG opcode plus 375 per indexed topic; index up to 3 parameters for efficient filtering but keep large data unindexed.
7. Pack storage variables by declaring smaller types (uint8, bool, address) adjacent to each other so they share a single 32-byte slot; a `uint128` + `uint128` fits one slot, saving 20,000 gas per avoided SSTORE.
8. Implement upgradeable contracts using the UUPS proxy pattern (`UUPSUpgradeable` from OpenZeppelin) with `_authorizeUpgrade` access control; avoid constructor logic — use `initializer` functions and the `initializer` modifier instead.
9. Follow ERC standards precisely: ERC20 must return `bool` from `transfer`/`approve`, ERC721 must implement `IERC721Receiver` checks on `safeTransferFrom`, ERC1155 must call `onERC1155Received` — use OpenZeppelin base contracts to avoid subtle compliance bugs.
10. Write comprehensive Foundry tests: use `forge test --gas-report` to profile gas, `vm.prank(address)` to simulate callers, `vm.expectRevert(CustomError.selector)` for error testing, and fuzz tests with `function testFuzz_transfer(uint256 amount) public` for edge cases.
11. Add NatSpec documentation (`/// @notice`, `/// @param`, `/// @return`, `/// @dev`) to all external and public functions — this generates user-facing documentation for Etherscan verification and is parsed by tools like `forge doc`.
12. Use `immutable` for values set in the constructor (stored in bytecode, 3-gas PUSH vs 2100-gas SLOAD) and `constant` for compile-time literals; avoid `public` on mappings/arrays unless you need auto-generated getters, as they add bytecode size.
