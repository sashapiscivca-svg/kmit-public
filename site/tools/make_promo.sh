#!/usr/bin/env bash
# Збирає промо-ролик 60 с (1920×1080, 60 к/с, без звуку): кадри з site/promo/promo.html через Chromium, кодування ffmpeg.
# Потрібні CHROME=/шлях/до/chrome, node, ffmpeg, python3. Результат: site/promo/kmit-promo.mp4 (≈ 15–25 хв рендеру).
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CHROME:?Вкажи CHROME=/шлях/до/chromium}"
FR="${FRAMES:-$HOME/.cache/kmit-promo/frames}"
rm -rf "$FR"; mkdir -p "$FR"
python3 tools/build_promo.py
nice node tools/render_promo.mjs "$FR" 60 60
ffmpeg -y -loglevel error -framerate 60 -i "$FR/f%05d.jpg" -vf "scale=in_range=pc:out_range=tv,format=yuv420p" -c:v libx264 -preset slow -crf 14 -color_range tv -colorspace bt709 -color_primaries bt709 -color_trc bt709 -an -movflags +faststart promo/kmit-promo.mp4
ffmpeg -y -loglevel error -i promo/kmit-promo.mp4 -vf "select=eq(n\,2100)" -vframes 1 promo/poster.jpg
rm -rf "$FR"
ls -la promo/*.mp4 promo/poster.jpg
