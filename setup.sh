#!/usr/bin/env bash
set -e

cargo install --locked trunk
cargo install --locked wasm-bindgen-cli
npm install -g sass
npm install -g wasm-opt