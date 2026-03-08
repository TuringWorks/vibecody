---
triggers: ["Solana", "solana", "Anchor", "anchor framework", "solana program", "PDA", "SPL token", "Metaplex", "solana CLI", "lamports"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["solana"]
category: blockchain
---

# Solana Program Development

When working with Solana programs:

1. Use Anchor framework (`anchor init myproject`) for all new programs — it provides account validation via `#[derive(Accounts)]`, automatic (de)serialization with Borsh, and IDL generation for client-side type safety; raw Solana SDK is only needed for highly optimized programs.
2. Design accounts carefully: every account must be passed explicitly to instructions; use `#[account(init, payer = user, space = 8 + MyData::INIT_SPACE)]` with `#[derive(InitSpace)]` to calculate exact space, adding 8 bytes for the discriminator.
3. Derive PDAs (Program Derived Addresses) with meaningful seeds: `#[account(seeds = [b"vault", user.key().as_ref()], bump)]`; store the bump in the account struct to avoid recomputing it, and always validate PDA ownership in constraints.
4. Handle cross-program invocations (CPIs) with Anchor's `CpiContext::new(program.to_account_info(), cpi_accounts)` pattern; be aware of the 4-level CPI depth limit and pass remaining accounts via `ctx.remaining_accounts` for dynamic account lists.
5. Work with SPL tokens using `anchor-spl`: `token::transfer(cpi_ctx, amount)` for transfers, `token::mint_to` for minting; always validate the token mint and token account owner in your Accounts struct with `constraint = token_account.mint == expected_mint.key()`.
6. Respect Solana's transaction size limit of 1232 bytes: minimize account count per instruction, use `lookup_tables` (Address Lookup Tables) for transactions needing many accounts, and split complex operations across multiple instructions or versioned transactions.
7. Set compute budget and priority fees for reliable landing: `ComputeBudgetInstruction::set_compute_unit_limit(300_000)` and `ComputeBudgetInstruction::set_compute_unit_price(micro_lamports)` — use `getRecentPrioritizationFees` RPC to estimate competitive fees.
8. Test with `anchor test` which spins up a local validator, or use `solana-test-validator --clone <program_id> --url mainnet-beta` to clone mainnet programs locally; write Anchor tests in TypeScript with `anchor.workspace.MyProgram` for type-safe interaction.
9. Use `solana program deploy target/deploy/my_program.so --program-id <keypair>` for deterministic addresses; set the upgrade authority with `solana program set-upgrade-authority` and consider making programs immutable after audits with `--final`.
10. Manage rent by ensuring accounts meet the minimum balance: `Rent::get()?.minimum_balance(data_len)` — accounts below rent-exempt threshold are garbage collected; use `close = destination` in Anchor to reclaim rent when closing accounts.
11. Integrate Metaplex for NFTs: use `mpl-token-metadata` for metadata accounts and `mpl-bubblegum` for compressed NFTs (cNFTs) that reduce minting cost from ~0.01 SOL to ~0.00005 SOL per NFT using concurrent Merkle trees.
12. Monitor program usage with `solana logs <program_id>` during development; emit structured events with `emit!(MyEvent { field: value })` in Anchor, and use Helius or Triton RPC for production-grade WebSocket subscriptions and transaction parsing.
