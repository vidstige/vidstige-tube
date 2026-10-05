//! Loads `.env` from the working directory into the environment. Real
//! environment variables win, so Cloud Run's settings are untouched.

use std::env;
use std::fs;

pub fn load() {
    let text = fs::read_to_string(".env").unwrap_or_default();
    let pairs = text.lines().filter_map(|l| l.split_once('='));
    for (name, value) in pairs {
        if env::var_os(name).is_none() {
            env::set_var(name, value);
        }
    }
}
