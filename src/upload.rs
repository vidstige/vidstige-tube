//! Puts a video in the store under a fresh id and prints its link.
//! The copying itself goes through gcloud, which holds the credentials.

use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

const ID_LENGTH: usize = 11;
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Eleven of base62 from /dev/urandom. Bytes at or above 248 are
/// dropped so the modulo is unbiased; 64 bytes leave ample room.
fn fresh_id() -> String {
    let mut bytes = [0u8; 64];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("read /dev/urandom");
    bytes
        .iter()
        .filter(|&&b| b < 4 * ALPHABET.len() as u8)
        .map(|&b| ALPHABET[b as usize % ALPHABET.len()] as char)
        .take(ID_LENGTH)
        .collect()
}

fn json_string(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            c if c < ' ' => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn copy(
    project: &str,
    source: &str,
    target: &str,
    kind: &str,
    cache: &str,
    stdin: &[u8],
) {
    let mut child = Command::new("gcloud")
        .args(["storage", "cp", source, target])
        .arg(format!("--content-type={kind}"))
        .arg(format!("--cache-control={cache}"))
        .arg(format!("--project={project}"))
        .stdin(Stdio::piped())
        .spawn()
        .expect("run gcloud");
    child.stdin.take().unwrap().write_all(stdin).expect("feed gcloud");
    let status = child.wait().expect("wait for gcloud");
    assert!(status.success(), "gcloud failed: {status}");
}

pub fn upload(file: &str, title: Option<&str>) {
    let project = std::env::var("TUBE_PROJECT").expect("TUBE_PROJECT is not set");
    let bucket = std::env::var("TUBE_BUCKET").expect("TUBE_BUCKET is not set");
    let host = std::env::var("TUBE_HOST").unwrap_or("<host>".into());
    let id = fresh_id();
    let video = format!("gs://{bucket}/{id}.mp4");
    let forever = "public, max-age=31536000, immutable";
    copy(&project, file, &video, "video/mp4", forever, b"");
    if let Some(title) = title {
        let meta = format!(r#"{{"title":{}}}"#, json_string(title));
        let target = format!("gs://{bucket}/{id}.json");
        let briefly = "public, max-age=300";
        copy(&project, "-", &target, "application/json", briefly, meta.as_bytes());
    }
    println!("https://{host}/{id}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_eleven_alphanumerics_and_differ() {
        let a = fresh_id();
        let b = fresh_id();
        assert_eq!(a.len(), ID_LENGTH);
        assert!(a.bytes().all(|b| b.is_ascii_alphanumeric()));
        assert_ne!(a, b);
    }

    #[test]
    fn quotes_backslashes_and_newlines_are_escaped() {
        assert_eq!(json_string("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
    }
}
