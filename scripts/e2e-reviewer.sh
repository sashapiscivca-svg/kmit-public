#!/usr/bin/env bash
# Наскрізна перевірка збірки методиста (feature reviewer). CHROME=/шлях/до/chrome
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CHROME:?Вкажи CHROME=/шлях/до/chromium}"
D="$(mktemp -d)"
scripts/build-reviewer-ui.sh
cargo build -p dev-server --features fast-kdf,reviewer --offline
BIN="${CARGO_TARGET_DIR:-target}/debug/dev-server"
nice -n 10 "$BIN" --port 8798 --data "$D/data" --ui app/reviewer-dist >"$D/server.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; rm -rf "$D"' EXIT
sleep 1
BASE=http://127.0.0.1:8798 nice -n 10 node app/e2e/reviewer.mjs
