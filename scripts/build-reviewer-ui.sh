#!/usr/bin/env bash
# Збирає інтерфейс застосунку методиста в app/reviewer-dist: власні файли (app/reviewer) + спільний api.js, шрифт Manrope
# і той самий перелік ліцензій app/ui/licenses.txt (екран «Ліцензії та подяки»; оновлюється scripts/licenses.py).
# Дитячого інтерфейсу в збірці методиста немає.
set -euo pipefail
cd "$(dirname "$0")/.."
D=app/reviewer-dist
rm -rf "$D"
mkdir -p "$D/fonts"
cp app/reviewer/index.html app/reviewer/reviewer.js app/reviewer/reviewer.css app/reviewer/kit.js "$D/"
cp app/ui/api.js app/ui/licenses.txt "$D/"
cp app/ui/fonts/Manrope-*.ttf app/ui/fonts/OFL-Manrope.txt "$D/fonts/"
echo "OK: $D"
