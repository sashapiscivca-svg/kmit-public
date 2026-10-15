#!/usr/bin/env python3
"""Збирає одноплікові сторінки site/index.html і site/presentation.html: підставляє шрифти, знімки й істот у data:-посилання.

  python3 site/tools/build_site.py
Шаблони: site/src/*.template.html. Знімки: site/assets/web/*.webp (site/tools/capture.sh + оптимізація, див. site/README.md).
Результат самодостатній: без зовнішніх шрифтів, скриптів і трекерів.
"""
import base64, re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FONTS = ROOT.parent / 'app' / 'ui' / 'fonts'
frag = (ROOT / 'assets' / 'worlds-fragment.html').read_text(encoding='utf-8')

def b64(p): return base64.b64encode(Path(p).read_bytes()).decode()

# Істоти за світами: {{creature:світ:номер}}
cards = {}
for m in re.finditer(r'<div class="world w-(\w+)"[^>]*data-world="(\w+)">(.*?)</div></div>', frag, re.S):
    cards[m.group(2)] = re.findall(r'<svg.*?</svg>', m.group(3), re.S)

def render(src, dst):
    s = (ROOT / 'src' / src).read_text(encoding='utf-8')
    s = re.sub(r'\{\{font:([\w-]+)\}\}', lambda m: f"data:font/ttf;base64,{b64(FONTS / (m.group(1) + '.ttf'))}", s)
    s = re.sub(r'\{\{img:([\w-]+)\}\}', lambda m: f"data:image/webp;base64,{b64(ROOT / 'assets' / 'web' / (m.group(1) + '.webp'))}", s)
    s = re.sub(r'\{\{creature:(\w+):(\d+)\}\}', lambda m: cards[m.group(1)][int(m.group(2))], s)
    s = s.replace('{{worlds}}', frag)
    left = re.findall(r'\{\{[^}]+\}\}', s)
    assert not left, left
    (ROOT / dst).write_text(s, encoding='utf-8')
    print(dst, len(s) // 1024, 'KB')

render('landing.template.html', 'index.html')
if (ROOT / 'src' / 'presentation.template.html').exists():
    render('presentation.template.html', 'presentation.html')
