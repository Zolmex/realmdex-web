#!/usr/bin/env bash
set -euo pipefail

# rustup's toolchain dir wins over a homebrew rust that may shadow it
export PATH="$HOME/.cargo/bin:$PATH"

cargo install -q worker-build@0.8.3

cd "$(dirname "$0")/.."
cd crates/worker
worker-build --release
