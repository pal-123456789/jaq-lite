//! Command-line front end for the `jaq_lite` library.
//!
//! This file stays thin on purpose: argument handling and process exit codes
//! live here, and everything that can be unit tested lives in the library.

#![forbid(unsafe_code)]

fn main() {
    println!("jaq-lite {}", jaq_lite::VERSION);
}
