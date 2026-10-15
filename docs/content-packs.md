# Пакети вмісту (формат `kmit-pack-1`)

Вміст предметів (крім математики, яка частково генерується кодом) зберігається як дані: `crates/subjects/packs/<id>.json`. Автори й методисти редагують ці файли або працюють через звіт для огляду (див. [review-workflow.md](review-workflow.md)); застосунок читає їх при запуску.

## Структура
```json
{
  "schema": "kmit-pack-1",
  "id": "history",            // латиниця малими й '_'
  "version": 1,               // збільшуй при кожній зміні; новіша версія замінює вбудовану
  "title": "Вступ до історії України",
  "skills":  [{"id": "HI-WHAT", "title": "…", "prerequisites": []}],
  "banks":   [{"skill": "HI-WHAT", "items": [{"prompt": "…", "correct": "…", "wrong": ["…","…"], "explanation": "…", "hints": ["…","…","…"]}]}],
  "typed":   [{"skill": "UA-T-NUMBER", "items": [{"prompt": "…", "accepted": ["…"], "explanation": "…", "hints": ["…"]}]}],
  "lessons": [{"skill": "HI-WHAT", "title": "…", "note_prompts": ["…"], "blocks": [
      {"type": "text", "text": "…"},
      {"type": "example", "statement": "…", "steps": ["…"], "result": "…"},
      {"type": "quiz", "prompt": "…", "options": ["…","…","…"], "correct": 0, "explanation": "…"},
      {"type": "selfcheck", "prompt": "…", "answer": "…", "explanation": "…"},
      {"type": "figure", "figure": {"kind": "fraction_bar", "num": 3, "den": 4}, "caption": "…"},
      {"type": "remember", "text": "…"}]}]
}
```
Малюнки: `fraction_bar{num,den}`, `decimal_grid{hundredths}`, `rect{w,h}`, `angle{deg,label}`, `number_line{min,max,parts,marks[{pos,label}]}`.

## Правила
- Умови (`prompt`) у межах теми унікальні; 2-5 хибних варіантів; підказки не називають відповіді; без форм минулого часу, що залежать від статі дитини (дитині «ти», дорослим «ви»); тексти підручників не копіюються.
- Перевірка: `cargo test -p subjects` (лінт, довжини, читабельність, структура уроків, гендерні форми) перед кожним випуском.
- Застосунок відхиляє пошкоджений пакет (невідомий формат, цикл у передумовах, невідомі теми, хибні малюнки).

## Оновлення без перевипуску застосунку
Підписаний пакет (`<id>.json` + `<id>.sig`, Ed25519) можна встановити в застосунку («Час і захист» → «Оновлення вмісту»): застосунок перевіряє підпис вбудованим відкритим ключем, а версію порівнює з наявною. Інструмент підпису й публікації: `content-tools` ([review-workflow.md](review-workflow.md)). Завантаження з інтернету немає й не планується: застосунок не має дозволу INTERNET, пакет потрапляє на пристрій лише як файл.
