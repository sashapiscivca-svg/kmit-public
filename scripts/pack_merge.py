#!/usr/bin/env python3
"""Зливає content-drafts/<одиниця>/skill-*.json у один пакет. Порядок тем: за plan.json (якщо є), інакше за іменем.

  scripts/pack_merge.py content-drafts/history_g6 [-o crates/subjects/packs/history_g6.json] [--into crates/subjects/packs/history.json]

--subject ID --grade N --title Т задають предмет, клас і коротку назву для обкладинки.
--into додає теми до наявного пакета (для 5 класу): id пакета лишається наявним, версія зростає на 1.
"""
import json, sys
from pathlib import Path

def main():
    a = sys.argv[1:]
    d = Path(a[0])
    out = a[a.index('-o') + 1] if '-o' in a else None
    into = a[a.index('--into') + 1] if '--into' in a else None
    parts = [json.loads(f.read_text(encoding='utf-8')) for f in sorted(d.glob('skill-*.json'))]
    order = []
    if (d / 'plan.json').exists():
        order = [s['id'] for s in json.loads((d / 'plan.json').read_text(encoding='utf-8')).get('skills', [])]
    def opt(name, default=None):
        return a[a.index(name) + 1] if name in a else default
    base = json.loads(Path(into).read_text(encoding='utf-8')) if into else {
        'schema': 'kmit-pack-1', 'id': parts[0]['id'], 'version': 1, 'title': opt('--title', parts[0]['title']),
        'subject': opt('--subject', ''), 'grade': int(opt('--grade', '5')),
        'skills': [], 'banks': [], 'typed': [], 'lessons': [], 'review': {'status': 'draft'}}
    if into:
        base['version'] += 1
    have = {s['id'] for s in base['skills']}
    items = []
    for p in parts:
        items.append((p['skills'][0]['id'], p))
    items.sort(key=lambda x: (order.index(x[0]) if x[0] in order else 10**6, x[0]))
    for sid, p in items:
        if sid in have:
            print('пропущено (вже є):', sid); continue
        base['skills'] += p['skills']; base['banks'] += p.get('banks', [])
        base['typed'] = base.get('typed', []) + p.get('typed', []); base['lessons'] += p.get('lessons', [])
    path = Path(into or out)
    path.write_text(json.dumps(base, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f'{path}: тем {len(base["skills"])}')

main()
