#!/usr/bin/env bash
# Запуск браузерної версії для перевірки на ноутбуці. Дані лежать у ~/.local/share/kmit-dev (створюються тут).
# Потім відкрий у браузері http://127.0.0.1:8765/ . Зупинити: Ctrl+C. Сервер не залишається фоновим процесом.
set -euo pipefail
cd "$(dirname "$0")/.."
DATA="${KMIT_DATA:-$HOME/.local/share/kmit-dev}"
exec nice -n 10 cargo run --release -p dev-server --offline -- --port "${PORT:-8765}" --data "$DATA" --ui app/ui
