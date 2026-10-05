//! `tube serve` serves one page and hands out signed links to a private
//! video store; `tube upload` puts a video there.
//!
//! Nothing in the bucket is readable without a signature, so the only
//! way to a video is through here: the page asks for an id, this signs
//! a URL good for a few hours, and the browser then streams straight
//! from storage. Proxying the bytes would put the whole file through
//! this service for nothing, since a signed URL honours range requests
//! just the same.
//!
//! The signing key is a service account key mounted as a file, from
//! Secret Manager in production, so nothing here talks to a secret store.

mod dotenv;
mod http;
mod key;
mod signed_url;
mod timestamp;
mod upload;

use std::env;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use http::Response;
use key::Signer;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_KEY_FILE: &str = "/secrets/signer.json";
/// Long enough to watch twice over, short enough that a leaked link rots.
const DEFAULT_LINK_SECONDS: u64 = 6 * 60 * 60;
const ID_LENGTH: std::ops::RangeInclusive<usize> = 6..=32;
const PAGE: &[u8] = include_bytes!("../index.html");

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn is_id(s: &str) -> bool {
    ID_LENGTH.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn links(signer: &Signer, bucket: &str, lifetime: u64, id: &str) -> Response {
    let sign = |object: String| {
        signed_url::signed_url(signer, bucket, &object, lifetime, now())
    };
    let body = format!(
        r#"{{"video":"{}","meta":"{}"}}"#,
        sign(format!("{id}.mp4")),
        sign(format!("{id}.json"))
    );
    Response {
        status: 200,
        headers: vec![
            ("content-type", "application/json"),
            ("cache-control", "private, no-store"),
        ],
        body: body.into_bytes(),
    }
}

fn page() -> Response {
    Response {
        status: 200,
        headers: vec![
            ("content-type", "text/html; charset=utf-8"),
            ("cache-control", "public, max-age=300"),
            ("x-robots-tag", "noindex, nofollow"),
        ],
        body: PAGE.to_vec(),
    }
}

fn serve() {
    let bucket = env::var("TUBE_BUCKET").expect("TUBE_BUCKET is not set");
    let key_file = env::var("TUBE_KEY").unwrap_or(DEFAULT_KEY_FILE.into());
    let lifetime = env_or("TUBE_LINK_SECONDS", DEFAULT_LINK_SECONDS);
    let port = env_or("PORT", DEFAULT_PORT);
    let signer = key::parse(&fs::read_to_string(&key_file).expect("read key file"))
        .expect("parse key file");

    http::serve(port, move |path| match path.strip_prefix("/s/") {
        Some(id) if is_id(id) => links(&signer, &bucket, lifetime, id),
        Some(_) => Response { status: 404, headers: vec![], body: vec![] },
        None => page(),
    })
}

fn main() {
    dotenv::load();
    let args: Vec<String> = env::args().collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[1..] {
        ["serve"] => serve(),
        ["upload", file] => upload::upload(file, None),
        ["upload", file, title] => upload::upload(file, Some(title)),
        _ => {
            eprintln!("usage: tube serve | tube upload FILE [TITLE]");
            std::process::exit(2);
        }
    }
}
