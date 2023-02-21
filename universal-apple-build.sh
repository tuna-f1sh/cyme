#!/bin/bash
set -eo pipefail

rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
cargo build --release --locked --all-features --target=aarch64-apple-darwin
cargo build --release --locked --all-features --target=x86_64-apple-darwin
mkdir -p target/universal-apple-darwin/release
lipo -create -output target/universal-apple-darwin/release/cyme target/aarch64-apple-darwin/release/cyme target/x86_64-apple-darwin/release/cyme
