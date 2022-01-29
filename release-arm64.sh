#!/usr/bin/env bash

set -ex

v=$(grep version Cargo.toml | head -1 | awk '{print $3}' | tr -d '"')

rm -rf 1History*zip 1History*zip.sha256sum
cargo build --release
cp -f target/release/onehistory .
zip 1History_v"${v}"_aarch64-apple-darwin.zip onehistory README.org
sha256sum 1History_v"${v}"_aarch64-apple-darwin.zip > 1History_v"${v}"_aarch64-apple-darwin.zip.sha256sum
