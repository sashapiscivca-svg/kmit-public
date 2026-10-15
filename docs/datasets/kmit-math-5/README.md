---
license: cc-by-4.0
language:
  - uk
pretty_name: Kmit Math 5 — Ukrainian grade-5 maths tasks with verified answers and hints
task_categories:
  - question-answering
  - text-generation
tags:
  - mathematics
  - education
  - k-12
  - ukrainian
  - word-problems
  - hints
  - misconceptions
  - synthetic
size_categories:
  - 1K<n<10K
configs:
  - config_name: default
    data_files:
      - split: test
        path: data/test.jsonl
---

# Kmit Math 5

**English** · [Українська](#українська)

6,301 Ukrainian maths tasks for grade 5 (the full course of the New Ukrainian School, 35 skills): arithmetic with natural numbers, decimals and fractions, divisibility, GCD and LCM, powers, equations, one- and two-step word problems, motion problems, units, perimeter, area, volume, angles, percentages and averages. Each task comes with a **verified answer key**, three graded hints that never reveal the answer, the typical wrong answers with the misconception behind each one, and structured data for the illustration where the app shows one.

The tasks are produced by the open-source task generators of [Kmit](https://github.com/sashapiscivca-svg/kmit-public), an offline learning assistant for Ukrainian schoolchildren.

## Why the answers can be trusted

The answer is never written by hand or by a language model. For every task:

1. a deterministic generator (Rust) computes the answer;
2. the answer is compared with an **oracle** that computes it a different way (`validate_task`). The same check rejects a task if the numbers in the statement differ from its operands, if a typical wrong answer equals the right one, and in a few other cases;
3. the answer key, written as a pupil would write it (decimal comma when the statement uses decimals, `a/b` for fractions), is parsed back by the app's answer parser and must equal the computed answer.

The wording of statements, hints and misconception explanations was drafted with AI assistance and **has not yet been reviewed by human methodologists**. The numbers and keys are computed; the language may contain stylistic or methodological imperfections.

## Fields

| Field | Type | Description |
|---|---|---|
| `id` | string | `<skill>-<variant>`, stable for a given generator version |
| `skill`, `skill_title` | string | skill code (e.g. `F-ADD-DIFF`) and its Ukrainian title |
| `grade` | int | 5 |
| `variant` | int | generator variant number (deterministic seed) |
| `statement` | string | the task as the pupil sees it, complete without the illustration |
| `answer_kind` | string | `number` for every task in this release |
| `answer` | string | answer key as a pupil writes it: `1220`, `17,7`, `9/10` |
| `choices` | list | empty in this release |
| `steps` | list | intermediate values of the solution |
| `hints` | list of 3 | graded hints R1–R3: from "what is being asked" to the method; they never contain the answer or intermediate results |
| `typical_errors` | list | `{id, value, explanation}`: a typical wrong answer, its misconception code and a child-friendly explanation that does not give the answer |
| `figures` | list | structured illustration data (rule: it shows only data from the statement, never the answer): `fraction_bar {num, den}`, `decimal_grid {hundredths}`, `rect {w, h}`, `angle {deg, label}` |

Example:

```json
{"id": "D-ADD-0001", "skill": "D-ADD", "skill_title": "Додавання десяткових дробів", "statement": "Обчисли: 8,9 + 8,8",
 "answer": "17,7", "hints": ["Ми додаємо десяткові дроби 8,9 і 8,8. Що означає цифра після коми?", "…", "…"],
 "typical_errors": [{"id": "tenths_as_hundredths", "value": "16,17", "explanation": "Схоже, суму десятих записано так, ніби це соті. …"}]}
```

## Intended uses

- **Evaluating language models** on school maths in Ukrainian: exact-match accuracy against `answer` (normalise the decimal comma and fractions with a fraction parser).
- **Research on tutoring**: generating or evaluating hints that do not give the answer away, and diagnosing misconceptions from a wrong answer (`typical_errors`).
- **Teaching materials**, worksheets and educational software.

Not intended for training systems that do children's homework for them.

## Statistics and reproduction

One `test` split: 6,301 tasks, 35 skills, 116–200 tasks per skill (200 variants per skill were generated; repeated statements were removed). 1,570 tasks have an illustration. The per-skill counts are in `stats.json`.

The dataset is regenerated exactly from the Kmit repository:

```bash
git clone https://github.com/sashapiscivca-svg/kmit-public && cd kmit-public
python3 scripts/export_math_dataset.py --out dist/kmit-math-5 --variants 200
```

## Licence and attribution

CC BY 4.0: you may use, share and adapt the dataset for any purpose, including commercial use, as long as you give attribution. Suggested attribution: *"Kmit Math 5" from "Kmit" by Oleksandr Shynkarenko, https://github.com/sashapiscivca-svg/kmit-public, licensed under CC BY 4.0.*

This licence covers this dataset only. The Kmit application and its other learning content keep their own licences (see the repository).

---

## Українська

6 301 задача з математики для 5 класу українською мовою (повний курс Нової української школи, 35 навичок): дії з натуральними числами, десятковими й звичайними дробами, подільність, НСД і НСК, степінь, рівняння, текстові задачі в одну й дві дії, задачі на рух, величини, периметр, площа, об'єм, кути, відсотки й середнє арифметичне. До кожної задачі додано **перевірений ключ**, три поступові підказки, які ніколи не називають відповіді, типові хибні відповіді з поясненням помилки та структуровані дані малюнка, якщо застосунок його показує.

Задачі створено відкритими генераторами застосунку [Kmit](https://github.com/sashapiscivca-svg/kmit-public), офлайн-помічника в навчанні для українських школярів.

**Чому ключам можна довіряти.** Відповідь ніколи не пише людина чи мовна модель. Детермінований генератор на Rust її обчислює. Потім її звіряють з «оракулом», що рахує іншим способом (`validate_task`), а ключ у записі учня (десяткова кома, дроби `a/b`) застосунок розбирає назад, і він має збігтися з обчисленою відповіддю. Формулювання умов, підказок і пояснень помилок підготовлено за допомогою ШІ, і **методисти-люди їх ще не вичитували**. Числа й ключі обчислені, але в текстах можливі стилістичні чи методичні недоліки.

**Призначення:** оцінювання мовних моделей зі шкільної математики українською (точний збіг з `answer`), дослідження підказок, що не видають відповіді, та діагностики помилок (`typical_errors`), навчальні матеріали й освітні програми. Датасет не призначений для систем, які розв'язують домашні завдання замість дітей.

**Відтворення:** `python3 scripts/export_math_dataset.py --out dist/kmit-math-5` у репозиторії Kmit.

**Ліцензія:** CC BY 4.0: датасет можна використовувати, поширювати й змінювати з будь-якою метою, зокрема комерційною, за умови зазначення авторства: *«Kmit Math 5» із «Kmit», автор Олександр Шинкаренко, https://github.com/sashapiscivca-svg/kmit-public, ліцензія CC BY 4.0.* Ця ліцензія стосується лише датасету; застосунок Kmit та інший його навчальний вміст мають власні ліцензії.
