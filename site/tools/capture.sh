#!/usr/bin/env bash
# Знімає екрани застосунку (390×844, DPR 2) у site/assets/shots для лендінгу, презентації й ролика.
# Потрібні CHROME=/шлях/до/chrome, node і зібраний dev-server (cargo build -p dev-server --features e2e).
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${CHROME:?Вкажи CHROME=/шлях/до/chromium}"
D="$(mktemp -d)"
OUT="${OUT:-site/assets/shots}"
cargo build -p dev-server --features e2e --offline
BIN="${CARGO_TARGET_DIR:-target}/debug/dev-server"
nice -n 10 "$BIN" --port 8797 --data "$D/data" --ui app/ui >"$D/server.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; rm -rf "$D"' EXIT
sleep 1
BASE=http://127.0.0.1:8797 WIDTH=390 HEIGHT=844 DPR=2 SHOTS="$OUT" THEME="${THEME:-light}" node site/tools/capture.mjs
