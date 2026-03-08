---
triggers: ["cryptography", "TLS 1.3", "X.509", "AEAD", "key derivation", "digital signature", "HSM", "post-quantum cryptography", "envelope encryption", "zero knowledge proof"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Applied Cryptography for Developers

When working with applied cryptography:

1. Use AEAD ciphers (AES-256-GCM, ChaCha20-Poly1305, XChaCha20-Poly1305) for symmetric encryption; AEAD provides confidentiality and integrity in a single operation — never use unauthenticated modes like AES-CBC or AES-CTR without a separate MAC.
2. Generate unique nonces/IVs for every encryption operation; for AES-GCM use 12-byte random nonces (safe up to ~2^32 encryptions per key), or use XChaCha20-Poly1305 with 24-byte nonces for virtually unlimited random nonce safety.
3. Derive keys from passwords using Argon2id (preferred), scrypt, or bcrypt — never raw SHA-256; configure Argon2id with at least 64 MB memory, 3 iterations, and 4 parallelism for password hashing, adjusting based on your latency budget.
4. Implement envelope encryption by generating a unique data encryption key (DEK) per resource, encrypting data with the DEK, then wrapping the DEK with a key encryption key (KEK) stored in an HSM or KMS; this limits HSM calls and enables efficient key rotation.
5. Configure TLS 1.3 exclusively in production by disabling TLS 1.0/1.1/1.2; TLS 1.3 reduces handshake latency to 1-RTT (0-RTT for resumption), eliminates vulnerable cipher suites, and mandates forward secrecy via ephemeral ECDHE key exchange.
6. Manage X.509 certificates with proper chain validation: verify the full chain to a trusted root, check revocation via OCSP stapling (preferred over CRL), enforce `subjectAltName` matching, and set certificate lifetimes to 90 days or less with automated renewal.
7. Use Ed25519 for digital signatures when possible (fast, small keys, no parameter choice); for compatibility with existing systems, use ECDSA with P-256 or RSA-PSS with 3072+ bit keys — always hash-then-sign and never sign raw data.
8. Integrate with HSMs (AWS CloudHSM, Azure Managed HSM, YubiHSM) for root key storage; use PKCS#11 or cloud KMS APIs to perform sign/unwrap operations inside the HSM boundary — keys should be non-extractable and audit-logged.
9. Prepare for post-quantum cryptography by inventorying all cryptographic dependencies; begin testing hybrid key exchange (X25519+ML-KEM-768) for TLS and ML-DSA (Dilithium) for signatures as NIST PQC standards finalize.
10. Implement Shamir's Secret Sharing (`(k, n)` threshold scheme) for splitting master keys or recovery secrets; require `k` of `n` shares to reconstruct, store shares in geographically separate locations, and never transmit multiple shares over the same channel.
11. Apply zero-knowledge proofs for authentication or compliance verification where you need to prove a statement without revealing the underlying data; use established libraries (libsnark, bellman, gnark) and prefer well-audited circuits over custom constructions.
12. Follow cryptographic hygiene: use `crypto/rand` (Go), `secrets` (Python), or OS CSPRNG for randomness — never `math/rand`; zeroize sensitive key material from memory after use; pin library versions and track CVEs in cryptographic dependencies continuously.
