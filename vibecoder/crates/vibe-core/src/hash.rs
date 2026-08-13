//! Non-cryptographic hashing shared across VibeCody.
//!
//! FNV-1a had four independent implementations in `vibecli-cli` alone
//! (`context_streaming`, `workspace_fingerprint`, `open_memory`, and
//! `repro_agent`, the last one named `simple_hash`), all with the same two
//! magic constants written four different ways. Named algorithms with fixed
//! constants belong in one place: a typo in one copy produces a hash that is
//! silently *not* FNV-1a, and nothing fails loudly.
//!
//! **Not for security.** FNV-1a is fast and well-distributed for cache keys,
//! dedup and change detection, and trivially invertible. Anything that needs
//! to resist an adversary wants SHA-256 (`sha2`), not this.

/// FNV-1a 64-bit offset basis.
const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit hash of raw bytes.
pub fn fnv1a_bytes(data: &[u8]) -> u64 {
    data.iter().fold(OFFSET_BASIS, |hash, &byte| {
        (hash ^ byte as u64).wrapping_mul(PRIME)
    })
}

/// FNV-1a 64-bit hash of a string's UTF-8 bytes.
pub fn fnv1a(data: &str) -> u64 {
    fnv1a_bytes(data.as_bytes())
}

/// FNV-1a 64-bit hash rendered as 16 lowercase hex digits, zero-padded.
pub fn fnv1a_hex(data: &str) -> String {
    format!("{:016x}", fnv1a(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Published FNV-1a 64-bit test vectors. These pin the constants: the
    /// whole point of one implementation is that it is the *correct* one.
    #[test]
    fn matches_published_fnv1a_64_vectors() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a("foobar"), 0x8594_4171_f739_67e8);
        assert_eq!(fnv1a("hello world"), 0x779a_65e7_023c_d2e7);
    }

    #[test]
    fn the_string_and_byte_forms_agree() {
        for s in ["", "a", "hello world", "日本語"] {
            assert_eq!(fnv1a(s), fnv1a_bytes(s.as_bytes()), "disagreed on {s:?}");
        }
    }

    #[test]
    fn is_deterministic_and_distinguishes_inputs() {
        assert_eq!(fnv1a("hello world"), fnv1a("hello world"));
        assert_ne!(fnv1a("hello world"), fnv1a("hello worle"));
    }

    #[test]
    fn hex_is_zero_padded_to_sixteen_digits() {
        for s in ["", "a", "foobar", "a much longer input string"] {
            let h = fnv1a_hex(s);
            assert_eq!(h.len(), 16, "{s:?} rendered as {h:?}");
            assert!(h
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        }
    }
}
