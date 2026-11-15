#!/usr/bin/env sh
# Builds the chip and places it next to tca6408a.chip.json, as Wokwi expects.
set -eu
cd "$(dirname "$0")"
cargo build --release
cp target/wasm32-unknown-unknown/release/tca6408a_wokwi_chip.wasm tca6408a.chip.wasm
