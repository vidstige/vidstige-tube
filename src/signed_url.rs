//! A V4 signed GET for an object in Cloud Storage, as
//! storage.googleapis.com checks it.

use rsa::pkcs1v15::SigningKey;
use rsa::sha2::{Digest, Sha256};
use rsa::signature::{SignatureEncoding, Signer as _};

use crate::key::Signer;
use crate::timestamp::stamp;

pub const HOST: &str = "storage.googleapis.com";

/// RFC 3986 unreserved characters pass; everything else is percent-encoded.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Query string of the signed request. Keys sort as plain bytes, which
/// these already do once "X-Goog-" is stripped: Algorithm, Credential,
/// Date, Expires, SignedHeaders.
fn query(credential: &str, now: &str, seconds: u64) -> String {
    [
        ("X-Goog-Algorithm", "GOOG4-RSA-SHA256"),
        ("X-Goog-Credential", credential),
        ("X-Goog-Date", now),
        ("X-Goog-Expires", &seconds.to_string()),
        ("X-Goog-SignedHeaders", "host"),
    ]
    .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
    .join("&")
}

pub fn signed_url(
    signer: &Signer,
    bucket: &str,
    object: &str,
    seconds: u64,
    unix_now: u64,
) -> String {
    let now = stamp(unix_now);
    let scope = format!("{}/auto/storage/goog4_request", &now[..8]);
    let credential = format!("{}/{scope}", signer.email);
    let query = query(&credential, &now, seconds);
    let path = object.split('/').map(encode).collect::<Vec<_>>().join("/");
    let resource = format!("/{bucket}/{path}");
    let canonical = [
        "GET",
        &resource,
        &query,
        &format!("host:{HOST}\n"),
        "host",
        "UNSIGNED-PAYLOAD",
    ]
    .join("\n");
    let to_sign = [
        "GOOG4-RSA-SHA256",
        &now,
        &scope,
        &hex(&Sha256::digest(canonical)),
    ]
    .join("\n");
    let key = SigningKey::<Sha256>::new(signer.key.clone());
    let signature = key.sign(to_sign.as_bytes());
    format!(
        "https://{HOST}{resource}?{query}&X-Goog-Signature={}",
        hex(&signature.to_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::signature::Verifier;
    use rsa::RsaPublicKey;

    const TEST_KEY: &str = include_str!("../testdata/signer.json");

    fn signer() -> Signer {
        crate::key::parse(TEST_KEY).unwrap()
    }

    #[test]
    fn reserved_characters_are_percent_encoded() {
        assert_eq!(encode("a b/c@d.e~"), "a%20b%2Fc%40d.e~");
    }

    #[test]
    fn url_carries_the_credential_scope_and_lifetime() {
        let url = signed_url(&signer(), "pail", "x.mp4", 3600, 1_700_000_000);
        assert!(url.starts_with("https://storage.googleapis.com/pail/x.mp4?"));
        assert!(url.contains("X-Goog-Date=20231114T221320Z"));
        assert!(url.contains("X-Goog-Expires=3600"));
        assert!(url.contains("%2F20231114%2Fauto%2Fstorage%2Fgoog4_request"));
    }

    #[test]
    fn signature_verifies_against_the_recomputed_string_to_sign() {
        let signer = signer();
        let url = signed_url(&signer, "pail", "a b.mp4", 60, 1_700_000_000);
        let (unsigned, signature) = url.split_once("&X-Goog-Signature=").unwrap();
        let (_, query) = unsigned.split_once('?').unwrap();
        let canonical = [
            "GET",
            "/pail/a%20b.mp4",
            query,
            "host:storage.googleapis.com\n",
            "host",
            "UNSIGNED-PAYLOAD",
        ]
        .join("\n");
        let to_sign = [
            "GOOG4-RSA-SHA256",
            "20231114T221320Z",
            "20231114/auto/storage/goog4_request",
            &hex(&Sha256::digest(canonical)),
        ]
        .join("\n");
        let bytes: Vec<u8> = (0..signature.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&signature[i..i + 2], 16).unwrap())
            .collect();
        let verifier = VerifyingKey::<Sha256>::new(RsaPublicKey::from(&signer.key));
        let signature = rsa::pkcs1v15::Signature::try_from(bytes.as_slice()).unwrap();
        verifier.verify(to_sign.as_bytes(), &signature).unwrap();
    }
}
