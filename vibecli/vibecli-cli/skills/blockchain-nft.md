---
triggers: ["NFT", "nft", "ERC721", "ERC1155", "nft mint", "nft metadata", "IPFS", "Arweave", "soul-bound token", "Merkle tree mint", "nft royalty"]
tools_allowed: ["read_file", "write_file", "bash"]
category: blockchain
---

# NFT Development

When working with NFT projects:

1. Choose ERC721 for unique 1-of-1 assets and ERC1155 for collections with editions or mixed fungible/non-fungible tokens — ERC1155 saves gas on batch operations with `safeBatchTransferFrom` and reduces contract deployment cost by consolidating logic.
2. Structure metadata following the OpenSea/ERC721 metadata standard: `{"name": "", "description": "", "image": "", "attributes": [{"trait_type": "", "value": ""}]}` — host the JSON on IPFS with a base URI pattern like `ipfs://Qm.../` and append token IDs.
3. Store assets on IPFS via Pinata or nft.storage for decentralized availability; for permanent storage use Arweave with Bundlr/Irys (`irys upload image.png -t arweave`) — always pin content to multiple providers to prevent loss from single-node failures.
4. Implement lazy minting by storing the creator's signature off-chain and verifying it on-chain during the buyer's mint transaction using `ECDSA.recover(hash, signature)` — this lets creators list NFTs without paying upfront gas.
5. Enforce royalties with EIP-2981: implement `royaltyInfo(tokenId, salePrice)` returning `(receiver, royaltyAmount)` — marketplaces query this function; for on-chain enforcement, use operator filter registries to block non-royalty-paying marketplaces.
6. Build allowlist minting with Merkle trees: generate the root off-chain from a list of addresses using `keccak256(abi.encodePacked(address))` as leaves, store only the 32-byte root on-chain, and verify proofs with OpenZeppelin's `MerkleProof.verify(proof, root, leaf)`.
7. Implement reveal mechanics by initially setting `tokenURI` to a placeholder (unrevealed metadata URI), then updating `baseURI` to the real IPFS folder after mint completes; use Chainlink VRF to generate a provably random offset for fair trait distribution.
8. Create soul-bound tokens (SBTs) per ERC5192: override `_update` (or `_beforeTokenTransfer` in older OZ) to block transfers after minting, and implement `locked(tokenId)` returning `true`; emit `Locked(tokenId)` on mint for marketplace compatibility.
9. Generate on-chain SVG NFTs by building SVG strings in Solidity: `string.concat('<svg xmlns="...">', traits, '</svg>')`, then Base64-encode the JSON metadata with `abi.encodePacked('data:application/json;base64,', Base64.encode(json))` in `tokenURI`.
10. Optimize gas for large mints using ERC721A which amortizes storage writes across sequential mints — minting 5 tokens costs nearly the same as minting 1; use `_mint(to, quantity)` instead of looping single mints.
11. Implement batch operations for ERC1155: `_mintBatch(to, ids, amounts, data)` for minting multiple token types in one transaction; define `uri(id)` to return metadata URLs with `{id}` substitution pattern per the ERC1155 metadata URI standard.
12. Test NFT contracts thoroughly: verify `balanceOf`, `ownerOf`, `tokenURI` correctness; test transfer restrictions for SBTs; simulate allowlist minting with valid and invalid Merkle proofs; check royalty calculations at various sale prices; and test reveal by asserting URI changes pre and post reveal.
