//! The signer: a service account's email and RSA key, as read from the
//! JSON key file Google hands out.

use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;

pub struct Signer {
    pub email: String,
    pub key: RsaPrivateKey,
}

/// Value of a top-level string field, with JSON escapes undone. The key
/// file is flat, so this is all the JSON parsing it takes.
fn field(json: &str, name: &str) -> Result<String, String> {
    let needle = format!("\"{name}\"");
    let after = json.find(&needle).ok_or(format!("no {name} in key"))?;
    let rest = &json[after + needle.len()..];
    let start = rest.find('"').ok_or(format!("{name} is not a string"))? + 1;
    let mut chars = rest[start..].chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(out),
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => break,
            },
            _ => out.push(c),
        }
    }
    Err(format!("{name} is unterminated"))
}

pub fn parse(json: &str) -> Result<Signer, String> {
    let email = field(json, "client_email")?;
    let pem = field(json, "private_key")?;
    let key = RsaPrivateKey::from_pkcs8_pem(&pem).map_err(|e| e.to_string())?;
    Ok(Signer { email, key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_newlines_become_real_ones() {
        let json = r#"{"a": "x\ny", "client_email": "me@x", "b": 1}"#;
        assert_eq!(field(json, "a").unwrap(), "x\ny");
        assert_eq!(field(json, "client_email").unwrap(), "me@x");
    }

    #[test]
    fn missing_field_is_an_error() {
        assert!(field("{}", "client_email").is_err());
    }
}
