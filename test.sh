#!/bin/sh
export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

set -e

cargo fmt
( cd test && cargo fmt )
( cd crates/guest && cargo fmt )

# tests the root workspace that includes all wasm code
cargo test -- --nocapture

# cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown -Z unstable-options
