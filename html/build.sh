#!/bin/sh
set -eu

cd "$(dirname "$0")"
cargo build -p cvar-html --release --target wasm32-unknown-unknown
cp ../target/wasm32-unknown-unknown/release/cvar_html.wasm public/cvar.wasm
echo 'Built public/cvar.wasm'
