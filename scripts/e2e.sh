#!/usr/bin/env bash
# Наскрізна перевірка інтерфейсу в headless Chromium. Потрібні node і Chromium (CHROME=/шлях/до/chrome).
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CHROME:?Вкажи CHROME=/шлях/до/chromium}"
D="$(mktemp -d)"
cargo build -p dev-server --features e2e --offline
BIN="${CARGO_TARGET_DIR:-target}/debug/dev-server"
nice -n 10 "$BIN" --port 8799 --data "$D/data" --ui app/ui >"$D/server.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; rm -rf "$D"' EXIT
sleep 1
BASE=http://127.0.0.1:8799 nice -n 10 node app/e2e/run.mjs
