//! Reading a TLS certificate.
//!
//! The fields come from `openssl x509`, not from `openssl s_client`'s chain
//! summary. That distinction is the whole reason this module exists: the
//! summary is a human-readable listing whose shape changes between OpenSSL
//! releases — 3.x prints
//!
//! ```text
//!    v:NotBefore: Sep  1 00:00:00 2026 GMT; NotAfter: Nov 29 23:59:59 2026 GMT
//! ```
//!
//! where 1.x printed no validity dates at all. Scraping it for `"Not After"`
//! found nothing, and "no expiry date" was rendered as **0 days remaining, and
//! therefore expired** — every certificate on every site, including the ones
//! whose chain had just verified two lines above. A parser that cannot find a
//! date must say it could not find one; it must not report zero.
//!
//! `openssl x509 -noout -subject -issuer -dates -serial -ext subjectAltName`
//! prints one documented `key=value` per line, and the same ones on every
//! version, which is what this parses.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// What a certificate says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CertFields {
    pub subject: String,
    pub issuer: String,
    /// Exactly as printed, e.g. `Sep  1 00:00:00 2026 GMT`. Empty when absent.
    pub not_before: String,
    pub not_after: String,
    pub san: Vec<String>,
    pub serial: String,
}

/// Why a certificate is or is not usable right now.
///
/// Kept apart from a bare `valid: bool` because "invalid" was the answer that
/// hid this bug: a working certificate and an unreadable one looked identical
/// on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validity {
    /// Chain verified and today is inside the validity window.
    Valid,
    /// The window has closed.
    Expired,
    /// The window has not opened yet.
    NotYetValid,
    /// OpenSSL would not verify the chain — its own reason, verbatim.
    ChainRejected(String),
    /// The dates could not be read, so nothing is being claimed about them.
    Unknown,
}

impl Validity {
    /// What the panel shows.
    pub fn label(&self) -> String {
        match self {
            Validity::Valid => "Valid".to_string(),
            Validity::Expired => "Expired".to_string(),
            Validity::NotYetValid => "Not yet valid".to_string(),
            Validity::ChainRejected(reason) => format!("Chain not trusted — {reason}"),
            Validity::Unknown => "Could not read the certificate".to_string(),
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Validity::Valid)
    }
}

/// Parse the output of `openssl x509 -noout -subject -issuer -dates -serial
/// -ext subjectAltName`.
pub fn parse_x509(text: &str) -> CertFields {
    let mut fields = CertFields::default();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("subject=") {
            fields.subject = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("issuer=") {
            fields.issuer = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("notBefore=") {
            fields.not_before = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("notAfter=") {
            fields.not_after = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("serial=") {
            fields.serial = value.trim().to_string();
        } else if trimmed.starts_with("X509v3 Subject Alternative Name") {
            // The names are on the line *after* the extension's own name.
            if let Some(names) = lines.peek() {
                fields.san = parse_san(names);
            }
        }
    }
    fields
}

/// `DNS:github.com, DNS:www.github.com, IP Address:1.2.3.4` → the DNS names.
fn parse_san(line: &str) -> Vec<String> {
    line.split(',')
        .filter_map(|entry| entry.trim().strip_prefix("DNS:").map(str::to_string))
        .filter(|name| !name.is_empty())
        .collect()
}

/// An OpenSSL validity timestamp, e.g. `Nov 29 23:59:59 2026 GMT`.
///
/// `None` when the text is not a date — which is a different answer from "the
/// epoch", and the difference is what this module was written for.
pub fn parse_openssl_time(text: &str) -> Option<DateTime<Utc>> {
    let text = text.trim().trim_end_matches(" GMT").trim();
    // `%e` is the day padded with a space, which is how OpenSSL prints single
    // digits: `Sep  1`, with two spaces after the month.
    let naive = NaiveDateTime::parse_from_str(text, "%b %e %H:%M:%S %Y")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%b %d %H:%M:%S %Y"))
        .ok()?;
    Utc.from_utc_datetime(&naive).into()
}

/// Whole days from `now` until the certificate expires.
///
/// `None` when the expiry could not be read. Negative once it has passed, so
/// "expired yesterday" and "expires tomorrow" are not the same number.
pub fn days_remaining(not_after: &str, now: DateTime<Utc>) -> Option<i64> {
    let expiry = parse_openssl_time(not_after)?;
    Some((expiry - now).num_days())
}

/// The verification result OpenSSL printed, as `(code, reason)`.
///
/// `s_client` ends with `Verify return code: 0 (ok)`, or a number and the
/// reason it failed.
pub fn verify_result(s_client_output: &str) -> Option<(u32, String)> {
    let line = s_client_output
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("Verify return code:"))?;
    let rest = line.split_once("Verify return code:")?.1.trim();
    let (code, reason) = match rest.split_once(' ') {
        Some((code, reason)) => (code, reason.trim().trim_matches(['(', ')'].as_ref())),
        None => (rest, ""),
    };
    Some((code.parse().ok()?, reason.to_string()))
}

/// Put the pieces together into one answer.
pub fn assess(fields: &CertFields, s_client_output: &str, now: DateTime<Utc>) -> Validity {
    match verify_result(s_client_output) {
        Some((0, _)) => {}
        Some((_, reason)) => return Validity::ChainRejected(reason),
        // No verify line at all: the connection did not get far enough to have
        // an opinion, and neither should this.
        None => return Validity::Unknown,
    }

    let expiry = match parse_openssl_time(&fields.not_after) {
        Some(expiry) => expiry,
        None => return Validity::Unknown,
    };
    if expiry <= now {
        return Validity::Expired;
    }
    match parse_openssl_time(&fields.not_before) {
        Some(start) if start > now => Validity::NotYetValid,
        _ => Validity::Valid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real output, from OpenSSL 3.6.
    const GITHUB: &str = "subject=CN=github.com
issuer=C=GB, O=Sectigo Limited, CN=Sectigo Public Server Authentication CA DV E36
notBefore=Sep  1 00:00:00 2026 GMT
notAfter=Nov 29 23:59:59 2026 GMT
serial=A59EBDB596751DB7F5C095079613953C
X509v3 Subject Alternative Name:
    DNS:github.com, DNS:www.github.com
";

    fn at(text: &str) -> DateTime<Utc> {
        parse_openssl_time(text).expect("test timestamp")
    }

    #[test]
    fn reads_every_field_openssl_prints() {
        let fields = parse_x509(GITHUB);
        assert_eq!(fields.subject, "CN=github.com");
        assert!(fields.issuer.contains("Sectigo"));
        assert_eq!(fields.not_before, "Sep  1 00:00:00 2026 GMT");
        assert_eq!(fields.not_after, "Nov 29 23:59:59 2026 GMT");
        assert_eq!(fields.serial, "A59EBDB596751DB7F5C095079613953C");
        assert_eq!(fields.san, vec!["github.com", "www.github.com"]);
    }

    #[test]
    fn a_single_digit_day_is_padded_with_a_space() {
        // `Sep  1` — two spaces. Parsing it with `%d` fails, and the failure
        // used to become "expired".
        let when = parse_openssl_time("Sep  1 00:00:00 2026 GMT").expect("padded day");
        assert_eq!(when.to_rfc3339(), "2026-09-01T00:00:00+00:00");
        // Some tools print it without the padding; both are the same instant.
        assert_eq!(parse_openssl_time("Sep 1 00:00:00 2026 GMT"), Some(when));
    }

    #[test]
    fn an_unreadable_date_is_unknown_rather_than_zero() {
        // The bug this module exists for: nothing found, so nothing claimed.
        assert_eq!(parse_openssl_time(""), None);
        assert_eq!(
            parse_openssl_time("v:NotBefore: Sep  1 00:00:00 2026"),
            None
        );
        assert_eq!(days_remaining("", Utc::now()), None);
    }

    #[test]
    fn counts_whole_days_either_side_of_the_expiry() {
        let expiry = "Nov 29 23:59:59 2026 GMT";
        assert_eq!(
            days_remaining(expiry, at("Nov 29 00:00:00 2026 GMT")),
            Some(0)
        );
        assert_eq!(
            days_remaining(expiry, at("Nov 19 23:59:59 2026 GMT")),
            Some(10)
        );
        // A certificate that expired last week says so, rather than saying zero.
        assert_eq!(
            days_remaining(expiry, at("Dec  6 23:59:59 2026 GMT")),
            Some(-7)
        );
    }

    #[test]
    fn the_leap_day_is_a_day() {
        // The arithmetic this replaces added `(year - 1969) / 4` and a fixed
        // table of month lengths, and was a day out for most of a leap year.
        assert_eq!(
            days_remaining("Mar  1 00:00:00 2028 GMT", at("Feb 28 00:00:00 2028 GMT")),
            Some(2),
            "2028 is a leap year: the 29th is between them"
        );
        assert_eq!(
            days_remaining("Mar  1 00:00:00 2027 GMT", at("Feb 28 00:00:00 2027 GMT")),
            Some(1)
        );
    }

    #[test]
    fn a_verified_chain_inside_its_window_is_valid() {
        let fields = parse_x509(GITHUB);
        let verified = "    Verify return code: 0 (ok)\n";
        assert_eq!(
            assess(&fields, verified, at("Oct  1 00:00:00 2026 GMT")),
            Validity::Valid
        );
    }

    #[test]
    fn each_way_of_being_unusable_says_which_one_it_is() {
        let fields = parse_x509(GITHUB);
        let ok = "Verify return code: 0 (ok)";

        assert_eq!(
            assess(&fields, ok, at("Jan  1 00:00:00 2027 GMT")),
            Validity::Expired
        );
        assert_eq!(
            assess(&fields, ok, at("Jan  1 00:00:00 2026 GMT")),
            Validity::NotYetValid
        );
        assert_eq!(
            assess(
                &fields,
                "Verify return code: 10 (certificate has expired)",
                at("Oct  1 00:00:00 2026 GMT")
            ),
            Validity::ChainRejected("certificate has expired".into())
        );
        // Nothing was read: the answer is "I do not know", not "invalid".
        assert_eq!(
            assess(&CertFields::default(), ok, Utc::now()),
            Validity::Unknown
        );
        assert_eq!(assess(&fields, "", Utc::now()), Validity::Unknown);
    }

    #[test]
    fn every_verdict_reads_differently() {
        let labels = [
            Validity::Valid.label(),
            Validity::Expired.label(),
            Validity::NotYetValid.label(),
            Validity::ChainRejected("self signed certificate".into()).label(),
            Validity::Unknown.label(),
        ];
        for (i, label) in labels.iter().enumerate() {
            assert!(!label.is_empty());
            assert!(
                labels.iter().skip(i + 1).all(|other| other != label),
                "{label}"
            );
        }
        assert!(Validity::Valid.is_valid());
        assert!(!Validity::Unknown.is_valid());
    }

    #[test]
    fn reads_the_verify_code_openssl_actually_prints() {
        assert_eq!(
            verify_result("Verify return code: 0 (ok)"),
            Some((0, "ok".to_string()))
        );
        assert_eq!(
            verify_result("    Verify return code: 21 (unable to verify the first certificate)"),
            Some((21, "unable to verify the first certificate".to_string()))
        );
        // `20` must not be read as `2`, and `2` must not match a line about `20`.
        assert_eq!(
            verify_result("Verify return code: 20 (unable to get local issuer certificate)")
                .map(|(code, _)| code),
            Some(20)
        );
        assert_eq!(verify_result("no such line"), None);
    }

    #[test]
    fn finds_the_verdict_in_everything_openssl_prints_around_it() {
        // Both of openssl's streams, in the order the command concatenates
        // them: the `verify return:1` callback lines land on stderr, and the
        // line that states the outcome is buried on stdout after the PEM
        // chain. Reading only stderr finds every reassuring line and not the
        // verdict — which reads as "no opinion", and used to read as "invalid".
        let combined = "depth=2 C=US, O=Example Root\n\
             verify return:1\n\
             depth=0 CN=github.com\n\
             verify return:1\n\
             DONE\n\
             CONNECTED(00000007)\n\
             -----BEGIN CERTIFICATE-----\n\
             MIIFdummy\n\
             -----END CERTIFICATE-----\n\
             SSL handshake has read 4381 bytes\n\
             Verify return code: 0 (ok)\n";
        assert_eq!(verify_result(combined), Some((0, "ok".to_string())));
    }

    #[test]
    fn a_certificate_with_no_san_extension_has_no_names() {
        let fields = parse_x509("subject=CN=old.example\nnotAfter=Nov 29 23:59:59 2026 GMT\n");
        assert!(fields.san.is_empty());
        assert_eq!(fields.subject, "CN=old.example");
    }

    #[test]
    fn ip_entries_in_the_san_are_not_hostnames() {
        let fields = parse_x509(
            "X509v3 Subject Alternative Name: \n    DNS:a.example, IP Address:10.0.0.1, DNS:b.example\n",
        );
        assert_eq!(fields.san, vec!["a.example", "b.example"]);
    }
}
