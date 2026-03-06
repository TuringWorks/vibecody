---
triggers: ["encryption", "hashing", "TLS", "key management", "AES", "RSA", "cryptography", "HMAC", "digital signature"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Cryptography & Key Management

When working with cryptography:

1. Never implement your own crypto — use well-audited libraries (libsodium, ring, openssl)
2. Symmetric encryption: use AES-256-GCM (authenticated encryption) — never ECB mode
3. Hashing: SHA-256 for data integrity, bcrypt/argon2 for passwords, BLAKE3 for speed
4. Use HMAC for message authentication — HMAC-SHA256 minimum
5. Key derivation: use PBKDF2, scrypt, or argon2 — never raw hash for key stretching
6. TLS: enforce TLS 1.2+ minimum, prefer 1.3 — disable SSLv3, TLS 1.0/1.1
7. Certificate pinning: implement for mobile apps connecting to your own APIs
8. Key management: rotate keys periodically, use envelope encryption for data at rest
9. Generate random values with CSPRNG (`/dev/urandom`, `SecureRandom`, `crypto.getRandomValues`)
10. Digital signatures: Ed25519 for speed, RSA-PSS (2048+ bits) for compatibility
11. Store secrets in environment variables or secret managers (Vault, AWS Secrets Manager)
12. Use constant-time comparison for secrets/tokens to prevent timing attacks
