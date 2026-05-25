#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p public/styles
npx --yes sass --no-source-map crates/app/style/index.scss public/styles/index.css
