#!/usr/bin/env python3
"""Сліпа перевірка ключів: агент відповідає на питання, не бачачи позначеної відповіді; розбіжності = ймовірні помилки.

  blind_quiz.py export <тека одиниці> <тека виводу>   -> blind-<ID>.json (prompt + 4 варіанти в перемішаному порядку, без ключа)
  blind_quiz.py compare <тека одиниці> <тека виводу>  -> друкує розбіжності між відповідями агента (answers-<ID>.json) і ключем
Формат answers-<ID>.json: {"answers": [{"i": 0, "pick": "текст обраного варіанта", "sure": true|false, "note": ""}, ...]}
"""
import json, random, sys
from pathlib import Path

def items_of(path):
    d = json.loads(path.read_text(encoding='utf-8'))
    return d['skills'][0]['id'], d['banks'][0]['items']

def main():
    mode, unit, out = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    files = sorted(unit.glob('skill-*.json'))
    if mode == 'export':
        for f in files:
            sid, items = items_of(f)
            rnd = random.Random(sid)
            rows = []
            for i, it in enumerate(items):
                opts = it['wrong'] + [it['correct']]
                rnd.shuffle(opts)
                rows.append({'i': i, 'prompt': it['prompt'], 'options': opts})
            (out / f'blind-{sid}.json').write_text(json.dumps({'skill': sid, 'questions': rows}, ensure_ascii=False, indent=1), encoding='utf-8')
        print(f'{len(files)} файлів у {out}')
    else:
        bad = 0
        for f in files:
            sid, items = items_of(f)
            ans = out / f'answers-{sid}.json'
            if not ans.exists():
                print('НЕМАЄ відповідей:', sid); continue
            got = {a['i']: a for a in json.loads(ans.read_text(encoding='utf-8'))['answers']}
            for i, it in enumerate(items):
                a = got.get(i)
                if a is None:
                    print(f'{sid} #{i}: немає відповіді'); bad += 1
                elif a['pick'].strip().lower() != it['correct'].strip().lower():
                    bad += 1
                    print(f"{sid} #{i}: ключ «{it['correct']}», агент «{a['pick']}» ({a.get('note','')}) | {it['prompt'][:90]}")
        print('розбіжностей:', bad)

main()
