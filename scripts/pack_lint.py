#!/usr/bin/env python3
"""Лінтер авторського вмісту (дзеркало перевірок cargo test -p subjects, без компіляції).

Використання:
  scripts/pack_lint.py <файл.json | тека частин> [--known ID,ID] [--lang-mixed-ok]

Тека частин = одна «одиниця» (предмет × клас): усі *.json усередині зливаються для перевірки передумов,
унікальності id тем і порядку. Код виходу 0, якщо помилок немає. Попередження не провалюють перевірку.
"""
import json
import re
import sys
from pathlib import Path

SCHEMA = "kmit-pack-1"
MIN_ITEMS = 25
GENDER_BAD = ["(-ла)", "(-ла", "(ла)", "(лась)", "(-лась)", "(-а)", "(а)", "(ка)", "(на)", "(ий)"]
FIG_KEYS = {
    "timeline": ["events"], "compare": ["head", "rows"], "cycle": ["steps"], "flow": ["steps"],
    "mindmap": ["center", "branches"], "bars": ["unit", "items"], "sets": ["a", "b", "only_a", "both", "only_b"],
    "fraction_bar": ["num", "den"], "decimal_grid": ["hundredths"], "rect": ["w", "h"],
    "angle": ["deg", "label"], "number_line": ["min", "max", "parts", "marks"],
}
CYR = re.compile(r"[А-Яа-яЇїІіЄєҐґ]")
LAT = re.compile(r"[A-Za-z]")


def load(path):
    p = Path(path)
    files = (sorted(p.glob("skill-*.json")) or sorted(f for f in p.glob("*.json") if f.name != "plan.json")) if p.is_dir() else [p]
    packs = []
    for f in files:
        packs.append((f, json.loads(f.read_text(encoding="utf-8"))))
    return packs


def hint_leaks(hint, correct):
    """Дзеркало tutor_core::bank::hint_leaks: для відповідей до 3 символів лише збіг із цілим словом."""
    h = hint.lower()
    c = correct.strip().lower()
    if not c:
        return False
    if len(c) <= 3:
        return c in re.split(r"[^\w']+", h)
    return c in h


def figure_rows(fg):
    k = fg.get("kind")
    if k == "timeline":
        return [[e.get("label", ""), e.get("text", "")] for e in fg.get("events", [])]
    if k == "compare":
        return [fg.get("head", [])] + list(fg.get("rows", []))
    if k in ("cycle", "flow"):
        return [[x] for x in fg.get("steps", [])]
    if k == "mindmap":
        return [[fg.get("center", "")]] + [[b.get("name", "")] + list(b.get("items", [])) for b in fg.get("branches", [])]
    if k == "bars":
        return [[i.get("label", ""), str(i.get("value", ""))] for i in fg.get("items", [])]
    if k == "sets":
        return [[fg.get("a", ""), fg.get("b", "")], list(fg.get("only_a", [])), list(fg.get("both", [])), list(fg.get("only_b", []))]
    return None


def figure_problem(kind, fg, rows):
    """Дзеркало tutor_core::figure::diagram_is_sane (інакше застосунок відхиляє весь пакет)."""
    if rows is None:
        return None
    if len(rows) > 10:
        return f"забагато рядків ({len(rows)} > 10)"
    for r in rows:
        for c in r:
            if not str(c).strip() or len(str(c)) > 60:
                return f"порожній чи задовгий (> 60) текст «{str(c)[:30]}…»"
    n = len(rows)
    if kind == "timeline" and not (2 <= n <= 8):
        return f"хронологія має 2-8 подій (є {n})"
    if kind == "compare" and not (n >= 2 and 2 <= len(rows[0]) <= 4 and all(len(r) == len(rows[0]) for r in rows)):
        return "compare: ≥ 2 рядки, заголовок 2-4 колонки, усі рядки однакової ширини"
    if kind == "cycle" and not (3 <= n <= 8):
        return f"cycle має 3-8 кроків (є {n})"
    if kind == "flow" and not (2 <= n <= 7):
        return f"flow має 2-7 кроків (є {n})"
    if kind == "mindmap" and not (2 <= n <= 7 and len(rows[0]) == 1 and all(2 <= len(r) <= 4 for r in rows[1:])):
        return "mindmap: центр + 1-6 гілок, кожна з назвою й 1-3 пунктами"
    if kind == "bars":
        if not (2 <= n <= 8):
            return "bars: 2-8 стовпчиків"
        for r in rows:
            try:
                if not (0 <= int(r[1]) <= 1_000_000):
                    return "bars: значення 0..1000000 цілі"
            except ValueError:
                return "bars: значення має бути цілим"
    if kind == "sets" and not (n == 4 and len(rows[0]) == 2 and all(len(r) <= 4 for r in rows[1:])):
        return "sets: a, b і до 4 елементів у кожній області"
    return None


def sentences(text):
    return [s.strip() for s in re.split(r"[.!?]", text) if s.strip()]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    known_extra = set()
    mixed_ok = "--lang-mixed-ok" in sys.argv
    for i, a in enumerate(sys.argv):
        if a == "--known" and i + 1 < len(sys.argv):
            known_extra = set(sys.argv[i + 1].split(","))
    if not args:
        print(__doc__)
        return 2
    packs = load(args[0])
    errs, warns = [], []
    E = lambda f, m: errs.append(f"{f.name}: {m}")
    W = lambda f, m: warns.append(f"{f.name}: {m}")

    skills = {}
    for f, p in packs:
        if p.get("schema") != SCHEMA:
            E(f, f"schema має бути {SCHEMA}")
        for k in ("id", "version", "title", "skills", "banks", "lessons"):
            if k not in p:
                E(f, f"немає поля {k}")
        raw = f.read_text(encoding="utf-8")
        for bad in GENDER_BAD:
            if bad in raw:
                E(f, f"гендерна форма {bad}")
        if "’" in raw or "ʼ" in raw:
            E(f, "типографський апостроф ’ чи ʼ: у даних лише прямий '")
        for s in p.get("skills", []):
            sid = s.get("id", "")
            if not re.fullmatch(r"[A-Z0-9]+(-[A-Z0-9]+)*", sid) or len(sid) > 40:
                E(f, f"хибний id теми {sid!r} (великі латинські літери, цифри, дефіси, ≤40)")
            if sid in skills:
                E(f, f"id теми {sid} повторюється ({skills[sid][0].name})")
            if not s.get("title", "").strip() or len(s["title"]) > 120:
                E(f, f"хибна назва теми {sid}")
            skills[sid] = (f, s)
    ids = set(skills)
    # передумови й цикли
    for sid, (f, s) in skills.items():
        for q in s.get("prerequisites", []):
            if q not in ids and q not in known_extra:
                E(f, f"{sid}: невідома передумова {q}")
    done, rest = set(), set(ids)
    while rest:
        step = {x for x in rest if all(q in done or q not in ids for q in skills[x][1].get("prerequisites", []))}
        if not step:
            errs.append(f"цикл у передумовах: {sorted(rest)[:6]}")
            break
        done |= step
        rest -= step

    banks, typed, lessons = {}, {}, {}
    for f, p in packs:
        for b in p.get("banks", []):
            if b["skill"] in banks:
                E(f, f"банк {b['skill']} заданий двічі")
            banks[b["skill"]] = (f, b)
        for b in p.get("typed", []):
            typed[b["skill"]] = (f, b)
        for l in p.get("lessons", []):
            if l["skill"] in lessons:
                E(f, f"урок {l['skill']} заданий двічі")
            lessons[l["skill"]] = (f, l)

    long_para = 0
    total_para = 0
    for sid, (f, s) in skills.items():
        if sid not in banks and sid not in typed:
            E(f, f"{sid}: немає банку завдань")
        if sid not in lessons:
            E(f, f"{sid}: немає уроку")
    for sid in set(banks) | set(typed) | set(lessons):
        if sid not in ids:
            f = (banks.get(sid) or typed.get(sid) or lessons.get(sid))[0]
            E(f, f"{sid}: вміст до теми, якої немає в skills")

    def check_text(f, where, t):
        if not isinstance(t, str):
            E(f, f"{where}: не рядок")
            return
        if "\n\n" in t and False:
            pass
        if t != t.strip():
            W(f, f"{where}: пробіли на краях")
        if not mixed_ok:
            for w in re.findall(r"[\w']+", t):
                if CYR.search(w) and LAT.search(w):
                    E(f, f"{where}: слово зі змішаними латинськими й кириличними літерами «{w}»")

    for sid, (f, b) in banks.items():
        items = b["items"]
        n = len(items)
        if n < MIN_ITEMS:
            E(f, f"{sid}: {n} завдань, потрібно ≥ {MIN_ITEMS}")
        seen, longest = set(), 0
        for i, it in enumerate(items):
            at = f"{sid} #{i} «{it.get('prompt', '')[:50]}»"
            pr = it.get("prompt", "").strip()
            if not pr:
                E(f, f"{at}: порожнє питання")
            if pr.lower() in seen:
                E(f, f"{at}: питання повторюється")
            seen.add(pr.lower())
            if len(pr) > 220:
                E(f, f"{at}: питання довше 220 символів ({len(pr)})")
            wrong, correct = it.get("wrong", []), it.get("correct", "")
            if not (2 <= len(wrong) <= 5):
                E(f, f"{at}: має бути 2-5 хибних варіантів (бажано 3)")
            opts = [w.strip().lower() for w in wrong] + [correct.strip().lower()]
            if len(set(opts)) != len(opts) or any(not o for o in opts):
                E(f, f"{at}: варіанти повторюються або порожні")
            for o in wrong + [correct]:
                if len(o) > 90:
                    E(f, f"{at}: варіант довший за 90 символів «{o[:40]}…»")
            ex = it.get("explanation", "")
            if len(ex.strip()) < 15:
                E(f, f"{at}: пояснення коротше 15 символів")
            hints = it.get("hints", [])
            if len(hints) != 3:
                E(f, f"{at}: потрібно рівно 3 підказки (є {len(hints)})")
            c = correct.strip().lower()
            for h in hints:
                if hint_leaks(h, c):
                    E(f, f"{at}: підказка містить правильну відповідь")
            for k in ("prompt", "correct", "explanation"):
                check_text(f, f"{at}.{k}", it.get(k, ""))
            for o in wrong:
                check_text(f, f"{at}.wrong", o)
            for h in hints:
                check_text(f, f"{at}.hint", h)
            lens = [len(o) for o in wrong + [correct]]
            if len(correct) == max(lens) and lens.count(max(lens)) == 1:
                longest += 1
        firsts = {}
        for it in items:
            for k in (0, 1):
                h = (it.get("hints") or ["", ""])[k:k + 1]
                if h:
                    firsts[(k, h[0])] = firsts.get((k, h[0]), 0) + 1
        worst = max(firsts.values()) if firsts else 0
        if worst > 3:
            E(f, f"{sid}: підказки повторюються: одна й та сама в {worst} із {n} завдань (кожна підказка має бути до конкретного завдання)")
        # однаковий «хвіст» (останні 2 слова) у хибних варіантах багатьох завдань: штучне подовження
        tails = {}
        for it in items:
            for w in it.get("wrong", []):
                t = " ".join(w.split()[-2:]).lower()
                if len(w.split()) >= 4 and t not in ("н. е.", "до н.", "до н. е.", "тис. років", "тис. до", "років тому", "століття н.", "століття до"):
                    tails[t] = tails.get(t, 0) + 1
        if tails and max(tails.values()) > max(4, n // 5):
            t = max(tails, key=tails.get)
            E(f, f"{sid}: хибні варіанти штучно подовжені однаковим хвостом «{t}» ({tails[t]} разів)")
        padded = sum(1 for it in items if "Правильна відповідь:" in it.get("explanation", ""))
        if padded > n // 10:
            E(f, f"{sid}: у {padded} із {n} пояснень механічна добудова «Правильна відповідь: …»; перепиши змістовно (чому так)")
        if longest * 100 > n * 55 + 100:
            E(f, f"{sid}: правильна відповідь найдовша у {longest} із {n}")

    for sid, (f, b) in typed.items():
        for i, it in enumerate(b["items"]):
            if not it.get("accepted"):
                E(f, f"{sid} #{i}: немає accepted")

    for sid, (f, l) in lessons.items():
        blocks = l.get("blocks", [])
        t = {"text": 0, "example": 0, "check": 0, "explain": 0, "hook": 0, "term": 0, "figure": 0, "remember": 0}
        words = 0
        for j, b in enumerate(blocks):
            ty = b.get("type")
            at = f"{sid} блок {j} ({ty})"
            if ty == "text":
                t["text"] += 1
                words += len(b["text"].split())
                check_text(f, at, b["text"])
                total_para += 1
                ss = sentences(b["text"])
                if not ss:
                    E(f, f"{at}: порожній абзац")
                for s in ss:
                    w = s.split()
                    if len(w) > 40:
                        E(f, f"{at}: речення на {len(w)} слів: «{s[:60]}…»")
                    for x in w:
                        if len(x) > 24:
                            E(f, f"{at}: надто довге слово «{x}»")
                if ss and sum(len(s.split()) for s in ss) / len(ss) > 22:
                    long_para += 1
            elif ty == "example":
                if len(b.get("steps", [])) >= 2:
                    t["example"] += 1
                else:
                    E(f, f"{at}: у прикладі має бути ≥ 2 кроки")
                if not b.get("statement") or not b.get("result"):
                    E(f, f"{at}: немає statement чи result")
            elif ty == "quiz":
                t["check"] += 1
                o, c = b.get("options", []), b.get("correct", -1)
                if len(o) < 3 or not (0 <= c < len(o)) or len(set(o)) != len(o):
                    E(f, f"{at}: хибні варіанти (≥3 різні, correct у межах)")
                if not b.get("explanation", "").strip() or not b.get("prompt", "").strip():
                    E(f, f"{at}: немає prompt чи пояснення")
                if re.search(r"\b(перш|друг|трет|четверт)\w*\s+(варіант|відповід)", b.get("explanation", ""), re.I):
                    E(f, f"{at}: пояснення посилається на номер варіанта")
            elif ty == "selfcheck":
                t["check"] += 1
                if not b.get("answer") or not b.get("explanation"):
                    E(f, f"{at}: немає answer чи explanation")
            elif ty == "explain":
                k = b.get("key_ideas", [])
                if 2 <= len(k) <= 6 and all(x.strip() for x in k):
                    t["explain"] += 1
                else:
                    E(f, f"{at}: key_ideas має містити 2-6 непорожніх тверджень")
            elif ty == "hook":
                t["hook"] += 1
            elif ty == "term":
                t["term"] += 1
                if not b.get("term") or not b.get("definition"):
                    E(f, f"{at}: порожнє поняття")
            elif ty == "figure":
                t["figure"] += 1
                fg = b.get("figure", {})
                kind = fg.get("kind")
                if kind not in FIG_KEYS:
                    E(f, f"{at}: невідомий вид малюнка {kind!r}")
                else:
                    for kk in FIG_KEYS[kind]:
                        if kk not in fg:
                            E(f, f"{at}: у малюнку {kind} немає поля {kk}")
                    rows = figure_rows(fg)
                    problem = figure_problem(kind, fg, rows)
                    if problem:
                        E(f, f"{at}: малюнок {kind}: {problem}")
            elif ty == "remember":
                t["remember"] += 1
            else:
                E(f, f"{at}: невідомий тип блока")
            for key in ("text", "statement", "result", "prompt", "definition", "term", "caption"):
                if isinstance(b.get(key), str):
                    check_text(f, f"{at}.{key}", b[key])
        need = [
            (t["text"] >= 5, "≥ 5 абзаців пояснення"), (words >= 150, "≥ 150 слів пояснення"),
            (t["example"] >= 2, "≥ 2 розібрані приклади"), (t["check"] >= 3, "≥ 3 контрольні питання"),
            (t["explain"] >= 1, "≥ 1 «поясни своїми словами»"), (t["hook"] >= 1, "≥ 1 «Подумай»"),
            (t["term"] >= 2, "≥ 2 поняття"), (t["figure"] >= 1, "≥ 1 схема чи малюнок"),
            (t["remember"] >= 1, "підсумок «Запам'ятай»"), (len(l.get("note_prompts", [])) >= 3, "≥ 3 запитання для конспекту"),
        ]
        for ok, what in need:
            if not ok:
                E(f, f"урок {sid}: потрібно {what}")
        if words < 220:
            W(f, f"урок {sid}: лише {words} слів пояснення; для «змістовного курсу» бажано ≥ 350")
        if t["check"] < 4:
            W(f, f"урок {sid}: бажано ≥ 4 контрольні питання")

    if long_para > max(8, total_para // 20):
        E(packs[0][0], f"забагато абзаців із довгими реченнями (середнє > 22 слів): {long_para} із {total_para}")

    n_items = sum(len(b["items"]) for _, b in banks.values())
    print(f"тем: {len(skills)}, завдань: {n_items}, уроків: {len(lessons)}, помилок: {len(errs)}, попереджень: {len(warns)}")
    for m in warns[:15]:
        print("УВАГА", m)
    for m in errs[:60]:
        print("ПОМИЛКА", m)
    if len(errs) > 60:
        print(f"… ще {len(errs) - 60} помилок")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main())
