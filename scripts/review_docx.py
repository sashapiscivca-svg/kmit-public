#!/usr/bin/env python3
"""Документи Word для вичитки вмісту вчителями й методистами, які не користуються застосунком методиста.

    python3 scripts/review_docx.py export <тека виводу> [--packs crates/subjects/packs] [--math-samples 3] [--only id,...]
    python3 scripts/review_docx.py import <файл.docx>... --out <vychytka.csv> [--report <звіт.md>]

`export` створює один .docx на пакет (предмет × клас): уроки й завдання теми за темою, у таблицях
«Поле | Текст | Виправлення | Коментар». Сірий код у першому стовпці збігається з рядком `content-tools review`
(skill|kind|index|field), тож заповнений документ переводиться назад у CSV без ручного переписування.
Для математики 5 класу (завдання створює генератор) додаються приклади згенерованих завдань: їх правлять у коді,
тому `import` переносить зауваження до них лише у звіт.

`import` читає заповнені документи й пише CSV у форматі `content-tools review`: `fix` для виправлень, `remove`, якщо в
рядку «Умова» написано «ПРИБРАТИ». Застосування: `cargo run -p content-tools -- apply crates/subjects/packs <csv>`.
Коментарі без виправлення, позначки «Тему перевірено» й зауваження до генератора йдуть у звіт (Markdown).

Потрібен python-docx (`pip install python-docx`).
"""
from __future__ import annotations

import argparse
import csv
import datetime
import io
import json
import re
import subprocess
import sys
from pathlib import Path

from docx import Document
from docx.enum.section import WD_ORIENT
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Cm, Pt, RGBColor

ROOT = Path(__file__).resolve().parents[1]
HEADER = ["pack", "skill", "kind", "index", "field", "text", "status", "correction", "comment"]
CODE_RE = re.compile(r"^\[([a-z0-9_]+):([A-Za-z0-9_-]+)\|(skill|bank|typed|lesson|gen|checked)\|([^|\]]*)\|([^\]]*)\]$")
REMOVE_WORDS = {"прибрати", "видалити", "прибрати завдання"}

FONT = "Arial"
INK = RGBColor(0x1F, 0x29, 0x37)
MUTED = RGBColor(0x8A, 0x93, 0x9E)
GREEN = RGBColor(0x0B, 0x6B, 0x3A)
ACCENT = RGBColor(0x0F, 0x5F, 0x57)
HEAD_FILL = "E8F3F1"
TASK_FILL = "F1F4F7"
YELLOW_FILL = "FFFBEA"

# Ширини стовпців (альбомна A4, поля 1,5 см): поле, текст, виправлення, коментар.
WIDTHS = [Cm(3.6), Cm(11.2), Cm(7.9), Cm(4.0)]


def grade_of(pack_id: str) -> int:
    m = re.search(r"_g(\d+)$", pack_id)
    return int(m.group(1)) if m else 5


def load_packs(folder: Path) -> list[dict]:
    packs = [json.loads(p.read_text(encoding="utf-8")) for p in sorted(folder.glob("*.json"))]
    return sorted(packs, key=lambda p: (grade_of(p["id"]), p["title"]))


# ---------------------------------------------------------------------------------------------------------------
# Дрібниці python-docx: заливка, межі, ширини.
# ---------------------------------------------------------------------------------------------------------------
def shade(cell, fill: str) -> None:
    tc_pr = cell._tc.get_or_add_tcPr()
    shd = OxmlElement("w:shd")
    shd.set(qn("w:val"), "clear")
    shd.set(qn("w:color"), "auto")
    shd.set(qn("w:fill"), fill)
    tc_pr.append(shd)


# Порядок дочірніх елементів w:tblPr за схемою OOXML: Word вважає файл пошкодженим, якщо порядок інший.
TBLPR_ORDER = ["tblStyle", "tblpPr", "tblOverlap", "bidiVisual", "tblStyleRowBandSize", "tblStyleColBandSize", "tblW", "jc",
               "tblCellSpacing", "tblInd", "tblBorders", "shd", "tblLayout", "tblCellMar", "tblLook"]


def put_tblpr(tbl_pr, el) -> None:
    name = el.tag.split("}")[1]
    later = TBLPR_ORDER[TBLPR_ORDER.index(name) + 1:]
    for child in tbl_pr:
        if child.tag.split("}")[1] in later:
            child.addprevious(el)
            return
    tbl_pr.append(el)


def set_borders(table, color: str = "C9D1D9") -> None:
    tbl_pr = table._tbl.tblPr
    borders = OxmlElement("w:tblBorders")
    for edge in ("top", "left", "bottom", "right", "insideH", "insideV"):
        el = OxmlElement(f"w:{edge}")
        el.set(qn("w:val"), "single")
        el.set(qn("w:sz"), "4")
        el.set(qn("w:color"), color)
        borders.append(el)
    put_tblpr(tbl_pr, borders)


def fix_widths(table, widths) -> None:
    table.autofit = False  # python-docx сам додає <w:tblLayout w:type="fixed"/> у правильне місце
    grid = table._tbl.tblGrid
    for gc, w in zip(grid.findall(qn("w:gridCol")), widths):
        gc.set(qn("w:w"), str(int(w.twips)))


def repeat_header(row) -> None:
    tr_pr = row._tr.get_or_add_trPr()
    el = OxmlElement("w:tblHeader")
    el.set(qn("w:val"), "true")
    tr_pr.append(el)


def cant_split(row) -> None:
    tr_pr = row._tr.get_or_add_trPr()
    tr_pr.append(OxmlElement("w:cantSplit"))


def keep_with_next(paragraph) -> None:
    paragraph.paragraph_format.keep_with_next = True


def write(cell, text: str, *, bold=False, color=None, size=None, italic=False, first=True):
    """Пише текст у клітинку (першим абзацом або новим)."""
    p = cell.paragraphs[0] if first else cell.add_paragraph()
    run = p.add_run(text)
    run.bold = bold
    run.italic = italic
    if color is not None:
        run.font.color.rgb = color
    if size is not None:
        run.font.size = Pt(size)
    return p


# ---------------------------------------------------------------------------------------------------------------
# Тексти полів у тому ж порядку й з тими ж ключами, що й `content-tools review` (crates/content-tools/src/review.rs).
# ---------------------------------------------------------------------------------------------------------------
LESSON_FIELDS = ["text", "statement", "result", "prompt", "answer", "explanation", "caption", "term", "definition"]


def lesson_label(btype: str, field: str, sub: int | None, block: dict) -> str:
    n = "" if sub is None else f" {sub + 1}"
    if field == "option":
        mark = " ✓ правильний" if block.get("correct") == sub else ""
        return f"Варіант{n}{mark}"
    labels = {
        ("hook", "prompt"): "Запитання на початку",
        ("text", "text"): "Текст",
        ("remember", "text"): "Запам’ятай (підсумок)",
        ("term", "term"): "Термін",
        ("term", "definition"): "Означення",
        ("example", "statement"): "Приклад: умова",
        ("example", "result"): "Приклад: відповідь",
        ("quiz", "prompt"): "Контрольне питання",
        ("quiz", "explanation"): "Пояснення відповіді",
        ("selfcheck", "prompt"): "Самоперевірка: питання",
        ("selfcheck", "answer"): "Самоперевірка: відповідь",
        ("selfcheck", "explanation"): "Самоперевірка: пояснення",
        ("explain", "prompt"): "Поясни своїми словами",
        ("figure", "caption"): "Підпис до схеми",
    }
    if field == "step":
        return f"Приклад: крок{n}"
    return labels.get((btype, field), field)


def lesson_rows(lesson: dict):
    """(код, підпис, текст, правильний?) для уроку. Код None: лише для перегляду, у CSV не потрапляє."""
    skill = lesson["skill"]
    yield f"{skill}|lesson|0|title", "Назва уроку", lesson["title"], False
    for i, b in enumerate(lesson["blocks"]):
        t = b.get("type", "")
        if t == "figure":
            yield None, "Схема (вміст)", figure_text(b.get("figure", {})), False
        for field in LESSON_FIELDS:
            if isinstance(b.get(field), str):
                right = t == "example" and field == "result" or t == "selfcheck" and field == "answer"
                yield f"{skill}|lesson|{i}|{field}", lesson_label(t, field, None, b), b[field], right
        for k, s in enumerate(b.get("steps") or []):
            yield f"{skill}|lesson|{i}|step:{k}", lesson_label(t, "step", k, b), s, False
        for k, s in enumerate(b.get("options") or []):
            yield f"{skill}|lesson|{i}|option:{k}", lesson_label(t, "option", k, b), s, b.get("correct") == k
        if t == "explain" and b.get("key_ideas"):
            yield None, "Ключові ідеї (для вчителя)", "; ".join(b["key_ideas"]), False
    for k, p in enumerate(lesson.get("note_prompts") or []):
        yield f"{skill}|lesson|0|note_prompt:{k}", f"Запитання для конспекту {k + 1}", p, False


def figure_text(f: dict) -> str:
    k = f.get("kind")
    if k == "compare":
        lines = [" | ".join(f.get("head", []))] + [" | ".join(r) for r in f.get("rows", [])]
        return "Таблиця порівняння:\n" + "\n".join(lines)
    if k in ("flow", "cycle"):
        sep = " → "
        return ("Схема-послідовність: " if k == "flow" else "Цикл: ") + sep.join(f.get("steps", [])) + (" → (знову)" if k == "cycle" else "")
    if k == "mindmap":
        return f"Карта понять «{f.get('center', '')}»: " + "; ".join(f"{b['name']}: {', '.join(b.get('items', []))}" for b in f.get("branches", []))
    if k == "timeline":
        return "Стрічка часу: " + "; ".join(f"{e.get('label', '')} ({e.get('text', '')})" for e in f.get("events", []))
    if k == "bars":
        return f"Діаграма ({f.get('unit', '')}): " + "; ".join(f"{i['label']}: {i['value']}" for i in f.get("items", []))
    if k == "sets":
        return (f"Дві множини: «{f.get('a')}» і «{f.get('b')}». Лише перша: {', '.join(f.get('only_a', []))}. "
                f"Обидві: {', '.join(f.get('both', []))}. Лише друга: {', '.join(f.get('only_b', []))}.")
    if k == "number_line":
        return f"Числова пряма від {f.get('min')} до {f.get('max')}; позначки: " + ", ".join(m.get("label", str(m.get("pos"))) for m in f.get("marks", []))
    if k == "fraction_bar":
        return f"Смужка дробу: {f.get('num')}/{f.get('den')}"
    if k == "decimal_grid":
        return f"Квадрат 10×10, зафарбовано {f.get('hundredths')} сотих"
    if k == "rect":
        return f"Прямокутник {f.get('w')} × {f.get('h')}"
    if k == "angle":
        return f"Кут {f.get('deg')}° ({f.get('label', '')})"
    return json.dumps(f, ensure_ascii=False)


def task_rows(skill: str, kind: str, i: int, it: dict):
    yield f"{skill}|{kind}|{i}|prompt", "Умова", it.get("prompt", ""), False
    if kind == "bank":
        yield f"{skill}|{kind}|{i}|correct", "Правильна відповідь ✓", it.get("correct", ""), True
        for k, w in enumerate(it.get("wrong") or []):
            yield f"{skill}|{kind}|{i}|wrong:{k}", f"Хибна відповідь {k + 1}", w, False
    else:
        for k, a in enumerate(it.get("accepted") or []):
            yield f"{skill}|{kind}|{i}|accepted:{k}", "Правильна відповідь ✓" if k == 0 else f"Теж приймається {k}", a, True
    yield f"{skill}|{kind}|{i}|explanation", "Пояснення", it.get("explanation", ""), False
    for k, h in enumerate(it.get("hints") or []):
        yield f"{skill}|{kind}|{i}|hint:{k}", f"Підказка {k + 1}", h, False


def gen_rows(skill: str, t: dict):
    v = t["variant"]
    yield f"{skill}|gen|{v}|statement", "Умова", t["statement"], False
    if t.get("choices"):
        yield None, "Варіанти", "; ".join(t["choices"]), False
    yield None, "Відповідь (обчислена) ✓", t["answer"], True
    for k, h in enumerate(t.get("hints") or []):
        yield f"{skill}|gen|{v}|hint:{k}", f"Підказка {k + 1}", h, False
    for e in t.get("typical_errors") or []:
        yield f"{skill}|gen|{v}|error:{e.get('id', '')}", f"Типова помилка (відповідь {e.get('value', '')})", e.get("explanation", ""), False


# ---------------------------------------------------------------------------------------------------------------
# Документ.
# ---------------------------------------------------------------------------------------------------------------
def new_document(title: str) -> Document:
    doc = Document()
    sec = doc.sections[0]
    sec.orientation = WD_ORIENT.LANDSCAPE
    sec.page_width, sec.page_height = Cm(29.7), Cm(21.0)
    for side in ("left_margin", "right_margin"):
        setattr(sec, side, Cm(1.5))
    sec.top_margin = sec.bottom_margin = Cm(1.5)
    st = doc.styles["Normal"]
    st.font.name = FONT
    st.font.size = Pt(10)
    st.font.color.rgb = INK
    st.element.rPr.rFonts.set(qn("w:eastAsia"), FONT)
    st.paragraph_format.space_after = Pt(2)
    for name, size, color, before in (("Title", 24, INK, 0), ("Heading 1", 16, ACCENT, 18), ("Heading 2", 13, INK, 12), ("Heading 3", 11, ACCENT, 8)):
        s = doc.styles[name]
        s.font.name = FONT
        s.element.rPr.rFonts.set(qn("w:eastAsia"), FONT)
        s.font.size = Pt(size)
        s.font.bold = True
        s.font.color.rgb = color
        s.paragraph_format.space_before = Pt(before)
        s.paragraph_format.space_after = Pt(6)
        s.paragraph_format.keep_with_next = True
    zoom = doc.settings.element.find(qn("w:zoom"))
    if zoom is not None:
        zoom.set(qn("w:percent"), "100")
    doc.core_properties.title = title
    doc.core_properties.author = "Kmit"
    # Нижній колонтитул: назва й номер сторінки.
    p = sec.footer.paragraphs[0]
    r = p.add_run(f"{title}   ·   с. ")
    r.font.size = Pt(8)
    r.font.color.rgb = MUTED
    for kind, text in (("begin", None), (None, "PAGE"), ("end", None)):
        run = p.add_run()
        run.font.size = Pt(8)
        run.font.color.rgb = MUTED
        if kind:
            fc = OxmlElement("w:fldChar")
            fc.set(qn("w:fldCharType"), kind)
            run._r.append(fc)
        else:
            it = OxmlElement("w:instrText")
            it.set(qn("xml:space"), "preserve")
            it.text = text
            run._r.append(it)
    return doc


def bullet(doc, text: str, style="List Bullet"):
    p = doc.add_paragraph(style=style)
    parts = re.split(r"(\*\*[^*]+\*\*)", text)
    for part in parts:
        if not part:
            continue
        r = p.add_run(part[2:-2] if part.startswith("**") else part)
        r.bold = part.startswith("**")
    return p


def intro(doc, pack: dict, n_topics: int, n_tasks: int, has_gen: bool) -> None:
    grade = grade_of(pack["id"])
    doc.add_paragraph(f"{pack['title']}, {grade} клас", style="Title")
    p = doc.add_paragraph()
    r = p.add_run(f"Матеріали застосунку Kmit для вичитки. Пакет «{pack['id']}», версія {pack['version']}, "
                  f"статус: чернетка (ще не перевірено методистом). Тем: {n_topics}"
                  f"{', завдань: ' + str(n_tasks) if n_tasks else ''}. "
                  f"Документ створено {datetime.date.today():%d.%m.%Y}.")
    r.font.color.rgb = MUTED
    doc.add_paragraph("Як працювати з документом", style="Heading 2")
    for t in [
        "Читайте стовпець **«Текст»**. Так цей текст бачить дитина. Правильні відповіді позначено зеленим і знаком ✓.",
        "Якщо текст треба змінити, напишіть у стовпці **«Виправлення»** **увесь новий текст повністю**, у тому вигляді, в якому його має побачити дитина (не «замінити X на Y»).",
        "Пояснення чи джерело (підручник, сторінка, пункт програми) пишіть у стовпці **«Коментар»**. Можна залишити коментар і без виправлення.",
        "Щоб **прибрати завдання** повністю, напишіть слово **ПРИБРАТИ** у стовпці «Виправлення» в рядку «Умова» цього завдання.",
        "Якщо все правильно, нічого не пишіть. Наприкінці кожної теми заповніть рядок **«Тему перевірено»**: ПІБ і дату.",
        "**Не змінюйте** стовпці «Поле» й «Текст», не видаляйте рядки й таблиці. Сірі коди в дужках потрібні, щоб ваші виправлення потрапили в застосунок автоматично. Не вмикайте режим рецензування (виправлення з позначками), просто пишіть у порожніх клітинках.",
        "Збережіть файл у форматі Word (.docx) і надішліть розробникові. Перейменовувати файл можна.",
    ]:
        bullet(doc, t, "List Number")
    doc.add_paragraph("На що звернути увагу", style="Heading 2")
    for t in [
        "**Факти й ключі:** правильна відповідь справді правильна, хибні справді хибні, немає двох правильних варіантів.",
        "**Програма:** тема й обсяг відповідають модельній програмі МОН для свого класу й підручникам із грифом МОН; термінологія як у підручниках.",
        "**Мова:** правопис, природність, звертання до дитини на «ти»; без форм, що залежать від статі («ти зробив(-ла)»).",
        "**Підказки** навідні й **ніколи не називають відповідь**. **Хибні варіанти** правдоподібні (типові помилки), правильний не завжди найдовший.",
        "**Пояснення** пояснює «чому», а не повторює відповідь, і не посилається на «перший / другий варіант» (застосунок перемішує варіанти).",
        "**Довжина:** умова до 220 символів, варіант відповіді до 90: довші тексти погано вміщуються на екрані телефона.",
        "Схеми в уроках подано текстом. Якщо схема неправильна, опишіть це в коментарі до підпису схеми.",
    ]:
        bullet(doc, t)
    if has_gen:
        doc.add_paragraph("Математика: завдання з генератора", style="Heading 2")
        doc.add_paragraph(
            "Завдання з математики створює програма: числа щоразу інші, а відповідь вона обчислює сама й перевіряє незалежно. "
            "Тому для кожної теми показано лише кілька прикладів. Виправлення до них розробник вносить у програму, тож "
            "пишіть, що саме не так із формулюванням, підказкою чи поясненням помилки, і це виправиться в усіх подібних завданнях.")


def add_table(doc, head: bool = True):
    table = doc.add_table(rows=0, cols=4)
    table.alignment = WD_TABLE_ALIGNMENT.CENTER
    set_borders(table)
    fix_widths(table, WIDTHS)
    if head:
        cells = table.add_row().cells
        for c, t, w in zip(cells, ["Поле", "Текст", "Виправлення (новий текст повністю)", "Коментар"], WIDTHS):
            c.width = w
            write(c, t, bold=True, color=ACCENT, size=9)
            shade(c, HEAD_FILL)
        repeat_header(table.rows[0])
    return table


def add_row(table, code: str | None, label: str, text: str, right: bool, pack_id: str) -> None:
    cells = table.add_row().cells
    for c, w in zip(cells, WIDTHS):
        c.width = w
    write(cells[0], label, bold=True, size=9, color=GREEN if right else None)
    if code:
        write(cells[0], f"[{code_key(pack_id, code)}]", size=6.5, color=MUTED, first=False)
    lines = text.split("\n")
    p = write(cells[1], lines[0], color=GREEN if right else None, bold=right)
    for extra in lines[1:]:
        p.add_run().add_break()
        p.add_run(extra)
    if code is None:
        shade(cells[2], "F6F6F6")
        write(cells[2], "лише перегляд: зауваження в коментарі", italic=True, color=MUTED, size=8)
    else:
        shade(cells[2], YELLOW_FILL)


def code_key(pack_id: str, code: str) -> str:
    # Код у документі: пакет|skill|kind|index|field. Пакет потрібен, бо в одного вчителя може бути кілька документів.
    return f"{pack_id}:{code}"


def task_header(table, title: str) -> None:
    row = table.add_row()
    cells = row.cells
    merged = cells[0].merge(cells[3])
    write(merged, title, bold=True, size=10)
    shade(merged, TASK_FILL)
    cant_split(row)
    # Заголовок завдання не лишається сам унизу сторінки.
    for p in merged.paragraphs:
        keep_with_next(p)


def checked_row(doc, pack_id: str, skill: str) -> None:
    table = doc.add_table(rows=1, cols=2)
    set_borders(table, "0F5F57")
    w = [Cm(8.0), Cm(18.7)]
    fix_widths(table, w)
    c0, c1 = table.rows[0].cells
    c0.width, c1.width = w
    write(c0, "Тему перевірено (ПІБ, дата):", bold=True, color=ACCENT)
    write(c0, f"[{pack_id}:{skill}|checked|0|*]", size=6.5, color=MUTED, first=False)
    shade(c0, HEAD_FILL)
    shade(c1, YELLOW_FILL)
    doc.add_paragraph()


def math_samples(n: int) -> dict[str, list[dict]]:
    if n <= 0:
        return {}
    raw = subprocess.run(
        ["cargo", "run", "-q", "-p", "generators-math", "--release", "--offline", "--example", "export_tasks", "--", str(max(n * 4, 8))],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    out: dict[str, list[dict]] = {}
    for line in raw.splitlines():
        t = json.loads(line)
        lst = out.setdefault(t["skill"], [])
        if len(lst) < n and all(x["statement"] != t["statement"] for x in lst):
            lst.append(t)
    return out


def file_name(pack: dict) -> str:
    title = re.sub(r"[^\w\s’'-]", "", pack["title"]).strip().replace(" ", "-")
    return f"{grade_of(pack['id'])}-клас_{title}.docx"


def export(args) -> None:
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    packs = load_packs(Path(args.packs))
    if args.only:
        wanted = set(args.only.split(","))
        packs = [p for p in packs if p["id"] in wanted]
    gen = math_samples(args.math_samples) if any(p["id"] == "math" for p in packs) else {}
    for pack in packs:
        pid = pack["id"]
        lessons = {l["skill"]: l for l in pack["lessons"]}
        banks = {b["skill"]: b["items"] for b in pack.get("banks", [])}
        typed = {b["skill"]: b["items"] for b in pack.get("typed", [])}
        topics = [(s["id"], s["title"]) for s in pack["skills"]] or [(l["skill"], l["title"]) for l in pack["lessons"]]
        has_gen = pid == "math" and bool(gen)
        n_tasks = sum(len(banks.get(s, [])) + len(typed.get(s, [])) for s, _ in topics)
        title = f"{pack['title']}, {grade_of(pid)} клас"
        doc = new_document(title)
        intro(doc, pack, len(topics), n_tasks, has_gen)
        doc.add_paragraph("Зміст", style="Heading 2")
        for n, (_, t) in enumerate(topics, 1):
            doc.add_paragraph(f"{n}. {t}").paragraph_format.space_after = Pt(0)
        for n, (sid, stitle) in enumerate(topics, 1):
            h = doc.add_paragraph(f"Тема {n}. {stitle}", style="Heading 1")
            h.paragraph_format.page_break_before = True
            if pack["skills"]:
                t = add_table(doc)
                add_row(t, f"{sid}|skill|0|title", "Назва теми", stitle, False, pid)
            if sid in lessons:
                doc.add_paragraph("Урок", style="Heading 2")
                t = add_table(doc)
                for code, label, text, right in lesson_rows(lessons[sid]):
                    add_row(t, code, label, text, right, pid)
            items = [("bank", i, it) for i, it in enumerate(banks.get(sid, []))] + [("typed", i, it) for i, it in enumerate(typed.get(sid, []))]
            if items:
                doc.add_paragraph(f"Завдання ({len(items)})", style="Heading 2")
                t = add_table(doc)
                for k, (kind, i, it) in enumerate(items, 1):
                    task_header(t, f"Завдання {k}" + (" (дитина вписує слово)" if kind == "typed" else ""))
                    for code, label, text, right in task_rows(sid, kind, i, it):
                        add_row(t, code, label, text, right, pid)
            if has_gen and gen.get(sid):
                doc.add_paragraph("Приклади завдань із генератора", style="Heading 2")
                t = add_table(doc)
                for k, task in enumerate(gen[sid], 1):
                    task_header(t, f"Приклад {k}")
                    for code, label, text, right in gen_rows(sid, task):
                        add_row(t, code, label, text, right, pid)
            doc.add_paragraph()
            checked_row(doc, pid, sid)
        path = out / file_name(pack)
        doc.save(path)
        print(f"{path}: тем {len(topics)}, завдань {n_tasks}")


# ---------------------------------------------------------------------------------------------------------------
# Зворотний шлях: заповнений документ → CSV для `content-tools apply` і звіт.
# ---------------------------------------------------------------------------------------------------------------
def cell_text(cell) -> str:
    return "\n".join(p.text for p in cell.paragraphs).strip()


def original_texts(folder: Path) -> dict[tuple, str]:
    texts: dict[tuple, str] = {}
    for pack in load_packs(folder):
        pid = pack["id"]
        for s in pack["skills"]:
            texts[(pid, s["id"], "skill", "0", "title")] = s["title"]
        for kind, key in (("bank", "banks"), ("typed", "typed")):
            for b in pack.get(key, []):
                for i, it in enumerate(b["items"]):
                    for code, _, text, _ in task_rows(b["skill"], kind, i, it):
                        sk, kd, idx, field = code.split("|")
                        texts[(pid, sk, kd, idx, field)] = text
        for l in pack["lessons"]:
            for code, _, text, _ in lesson_rows(l):
                if code:
                    sk, kd, idx, field = code.split("|")
                    texts[(pid, sk, kd, idx, field)] = text
    return texts


def import_(args) -> None:
    texts = original_texts(Path(args.packs))
    rows, report, problems = [], [], []
    for f in args.files:
        doc = Document(f)
        for table in doc.tables:
            for row in table.rows:
                cells = row.cells
                if len(cells) < 2:
                    continue
                code_par = cells[0].paragraphs[-1].text.strip()
                m = CODE_RE.match(code_par)
                if not m:
                    continue
                pid, skill, kind, idx, field = m.groups()
                if kind == "checked":
                    who = cell_text(cells[1])
                    if who:
                        report.append(f"- {pid} {skill}: тему перевірено ({who})")
                    continue
                corr = cell_text(cells[2]) if len(cells) > 2 else ""
                comm = cell_text(cells[3]) if len(cells) > 3 else ""
                if not corr and not comm:
                    continue
                if kind == "gen":
                    report.append(f"- {pid} {skill} генератор, приклад {idx}, {field}: виправлення «{corr}» коментар «{comm}»")
                    continue
                key = (pid, skill, kind, idx, field)
                if key not in texts:
                    problems.append(f"- {Path(f).name}: невідомий код {code_par}")
                    continue
                original = texts[key]
                doc_text = cell_text(cells[1])
                if doc_text.replace("\n", " ") != original.replace("\n", " ").strip():
                    problems.append(f"- {Path(f).name}: {code_par}: текст у документі змінено, перевірте вручну (виправлення: «{corr}»)")
                if corr.strip().lower() in REMOVE_WORDS:
                    if kind in ("bank", "typed") and field == "prompt":
                        rows.append([pid, skill, kind, idx, field, original, "remove", "", comm])
                    else:
                        problems.append(f"- {code_par}: «прибрати» можна лише в рядку «Умова» завдання")
                elif corr:
                    rows.append([pid, skill, kind, idx, field, original, "fix", corr, comm])
                else:
                    report.append(f"- {pid} {skill} {kind} {idx} {field}: коментар без виправлення: «{comm}»")
    buf = io.StringIO()
    w = csv.writer(buf, lineterminator="\n")
    w.writerow(HEADER)
    w.writerows(rows)
    Path(args.out).write_text("﻿" + buf.getvalue(), encoding="utf-8")
    fixes = sum(1 for r in rows if r[6] == "fix")
    removes = len(rows) - fixes
    print(f"{args.out}: виправлень {fixes}, прибрати {removes}; у звіті {len(report)} рядків, проблем {len(problems)}")
    if args.report:
        Path(args.report).write_text(
            "# Звіт із документів вичитки\n\n## Позначки й коментарі\n\n" + ("\n".join(report) or "немає") +
            "\n\n## Перевірити вручну\n\n" + ("\n".join(problems) or "немає") + "\n", encoding="utf-8")
    elif problems:
        print("\n".join(problems), file=sys.stderr)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("export", help="створити документи")
    e.add_argument("out")
    e.add_argument("--packs", default=str(ROOT / "crates/subjects/packs"))
    e.add_argument("--math-samples", type=int, default=3, help="прикладів згенерованих завдань на тему математики (0: без них)")
    e.add_argument("--only", help="лише ці пакети (id через кому)")
    i = sub.add_parser("import", help="перетворити заповнені документи на CSV")
    i.add_argument("files", nargs="+")
    i.add_argument("--out", required=True)
    i.add_argument("--report")
    i.add_argument("--packs", default=str(ROOT / "crates/subjects/packs"))
    a = ap.parse_args()
    export(a) if a.cmd == "export" else import_(a)


if __name__ == "__main__":
    main()
