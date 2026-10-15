#!/usr/bin/env python3
"""Build the open dataset of Kmit's grade-5 maths tasks (Hugging Face: kmit-math-5).

    python3 scripts/export_math_dataset.py --out dist/kmit-math-5 [--variants 200]

Runs the Rust generators (`cargo run --example export_tasks`), adds a stable id and the skill title from the maths pack,
and writes `data/test.jsonl`, `stats.json`, the licence and the dataset card (docs/datasets/kmit-math-5/README.md). Every answer key was produced by a
generator, checked against an independent oracle (`validate_task`) and parsed back by the app's answer parser.
"""
from __future__ import annotations

import argparse
import collections
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--variants", type=int, default=200, help="variants generated per skill before de-duplication")
    a = ap.parse_args()

    raw = subprocess.run(
        ["cargo", "run", "-q", "-p", "generators-math", "--release", "--example", "export_tasks", "--", str(a.variants)],
        cwd=ROOT, check=True, capture_output=True, text=True,
    )
    pack = json.loads((ROOT / "crates/subjects/packs/math.json").read_text(encoding="utf-8"))
    titles = {lesson["skill"]: lesson["title"] for lesson in pack["lessons"]}

    rows = []
    for line in raw.stdout.splitlines():
        r = json.loads(line)
        rows.append({"id": f"{r['skill']}-{r['variant']:04d}", "skill": r["skill"], "skill_title": titles.get(r["skill"], r["skill"]),
                     "grade": 5, **{k: r[k] for k in ("variant", "statement", "answer_kind", "answer", "choices", "steps", "hints",
                                                      "typical_errors", "figures")}})
    ids = [r["id"] for r in rows]
    assert len(ids) == len(set(ids)), "duplicate ids"

    data = a.out / "data"
    data.mkdir(parents=True, exist_ok=True)
    with (data / "test.jsonl").open("w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    per_skill = collections.Counter(r["skill"] for r in rows)
    stats = {"tasks": len(rows), "skills": len(per_skill), "variants_per_skill": a.variants,
             "per_skill": {s: {"title": titles.get(s, s), "tasks": n} for s, n in sorted(per_skill.items())},
             "generator_log": raw.stderr.strip()}
    (a.out / "stats.json").write_text(json.dumps(stats, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    shutil.copy(ROOT / "LICENSES" / "CC-BY-4.0.txt", a.out / "LICENSE")   # the dataset is CC BY 4.0 (see LICENSE-CONTENT.md)
    shutil.copy(ROOT / "docs" / "datasets" / "kmit-math-5" / "README.md", a.out / "README.md")   # dataset card
    print(f"{len(rows)} tasks in {len(per_skill)} skills -> {a.out}  ({raw.stderr.strip()})")


if __name__ == "__main__":
    main()
