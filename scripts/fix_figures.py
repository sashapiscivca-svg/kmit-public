#!/usr/bin/env python3
"""Підганяє малюнки уроків під обмеження застосунку (діаграма: тексти ≤ 60 символів, мінімум/максимум рядків).
Використання: scripts/fix_figures.py <тека одиниці>. Втрачений зміст друкується (що обрізано)."""
import json, sys
from pathlib import Path

def cut(t):
    t = str(t).strip()
    if len(t) <= 60:
        return t
    c = t[:57].rsplit(' ', 1)[0].rstrip(' ,;:-')
    return c + '…'

def fix(fg, log):
    k = fg.get('kind')
    if k == 'timeline':
        ev = fg['events'][:8]
        if len(fg['events']) > 8: log.append('timeline: обрізано до 8 подій')
        for e in ev:
            e['label'], e['text'] = cut(e['label']), cut(e['text'])
        fg['events'] = ev
    elif k in ('cycle', 'flow'):
        mx = 8 if k == 'cycle' else 7
        if len(fg['steps']) > mx: log.append(f'{k}: обрізано до {mx}')
        fg['steps'] = [cut(x) for x in fg['steps'][:mx]]
    elif k == 'mindmap':
        br = []
        for b in fg['branches'][:6]:
            items = [cut(i) for i in b.get('items', []) if str(i).strip()][:3]
            if not items:
                continue
            br.append({'name': cut(b['name']), 'items': items})
        if len(fg['branches']) > 6 or any(len(b.get('items', [])) > 3 for b in fg['branches']): log.append('mindmap: обрізано гілки/пункти')
        fg['center'] = cut(fg['center']); fg['branches'] = br
    elif k == 'compare':
        fg['head'] = [cut(x) for x in fg['head']][:4]
        w = len(fg['head'])
        fg['rows'] = [[cut(x) for x in r][:w] + [''] * 0 for r in fg['rows'][:9]]
    elif k == 'bars':
        fg['items'] = [{'label': cut(i['label']), 'value': i['value']} for i in fg['items'][:8]]
    elif k == 'sets':
        for key in ('only_a', 'both', 'only_b'):
            fg[key] = [cut(x) for x in fg[key]][:4]
        fg['a'], fg['b'] = cut(fg['a']), cut(fg['b'])

for f in sorted(Path(sys.argv[1]).glob('skill-*.json')):
    d = json.loads(f.read_text(encoding='utf-8'))
    log = []
    for l in d.get('lessons', []):
        for b in l['blocks']:
            if b.get('type') == 'figure':
                before = json.dumps(b['figure'], ensure_ascii=False, sort_keys=True)
                fix(b['figure'], log)
                if json.dumps(b['figure'], ensure_ascii=False, sort_keys=True) != before:
                    log.append(f"змінено малюнок {b['figure'].get('kind')}")
    if log:
        f.write_text(json.dumps(d, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        print(f.name, '; '.join(log))
