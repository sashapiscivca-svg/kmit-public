#!/usr/bin/env python3
"""Збирає site/promo/promo.html із шаблону: підставляє плитки світів із живими істотами (з worlds-fragment.html)."""
import re
from pathlib import Path
root = Path(__file__).resolve().parent.parent
frag = (root / 'assets' / 'worlds-fragment.html').read_text(encoding='utf-8')
tiles = []
for i, m in enumerate(re.finditer(r'<div class="world w-\w+" data-world="(\w+)"><div class="wh"><b>(.*?)</b><span>(.*?)</span></div><div class="wr">(.*?)</div></div>', frag, re.S)):
    svg = re.findall(r'<svg.*?</svg>', m.group(4), re.S)[[2, 3, 4, 5, 2, 3, 4, 5][i]]
    tiles.append(f"s.tileEls.push(layers.appendChild(mk('div', 'abs tile', `<em>{m.group(3)}</em>{svg}<b>{m.group(2)}</b>`)));")
assert len(tiles) == 8
html = (root / 'promo' / 'promo.template.html').read_text(encoding='utf-8').replace('{{tiles}}', '\n    '.join(tiles))
(root / 'promo' / 'promo.html').write_text(html, encoding='utf-8')
print('promo.html', len(html) // 1024, 'KB')
