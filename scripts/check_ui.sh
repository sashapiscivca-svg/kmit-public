#!/usr/bin/env bash
# Перевірка інтерфейсу без залежностей: синтаксис модулів і відсутність нових можливостей мови, яких не знають старі Android WebView.
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
for f in app/ui/*.js app/reviewer/*.js; do
  node --input-type=module --check < "$f" || { echo "СИНТАКСИС: $f"; bad=1; }
done
if grep -nE '^await |\.at\(|\.replaceAll\(|structuredClone\(|\?\?=|\|\|=|&&=|\.findLast\(|Object\.groupBy|\.toSorted\(|\.toReversed\(' app/ui/*.js app/reviewer/*.js; then
  echo "Знайдено можливості, яких немає в старих WebView (див. вище)"; bad=1
fi
exit $bad
