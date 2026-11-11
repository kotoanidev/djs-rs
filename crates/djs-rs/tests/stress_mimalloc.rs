//! The `Client` stress tests of `stress.rs` with [mimalloc](https://docs.rs/mimalloc) as the
//! global allocator, to compare with the system allocator (D-080).
//!
//! ```text
//! cargo test -p djs-rs --release --test stress_mimalloc -- --ignored --nocapture --test-threads 1
//! ```

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "stress.rs"]
mod stress;
