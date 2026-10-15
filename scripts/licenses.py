#!/usr/bin/env python3
"""Збирає ліцензії й повідомлення про авторські права сторонніх компонентів у app/ui/licenses.txt
(екран «Ліцензії та подяки» в застосунку; той самий файл копіюється в збірку методиста).

Що потрапляє до файла:
  * власні ліцензії Kmit і Required Notice;
  * усі крейти Rust, що компонуються в Android-застосунок (aarch64-linux-android): обхід лише звичайних
    залежностей від kmit-app (з усіма features, тобто й зі збіркою методиста), без build-залежностей
    і proc-macro (вони працюють тільки під час збирання); для кожного — рядки Copyright і посилання
    на тексти з файлів LICENSE*/COPYING*/NOTICE* у пакеті;
  * вручну описані компоненти зі scripts/third_party/manual.json: SQLCipher, SQLite, OpenSSL,
    бібліотеки Android/JVM (версії з build.gradle.kts), шрифти;
  * повні тексти ліцензій, кожен лише раз (тексти, що відрізняються тільки рядками Copyright, зливаються).

Запуск: python3 scripts/licenses.py          — перезаписати app/ui/licenses.txt
        python3 scripts/licenses.py --check  — лише перевірити, що файл актуальний (код виходу 1, якщо ні)
Потрібні cargo і завантажені залежності (cargo fetch --manifest-path app/src-tauri/Cargo.toml);
мережа не потрібна. Лише стандартна бібліотека Python.
"""
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEST = ROOT / "app/ui/licenses.txt"
MANUAL = ROOT / "scripts/third_party/manual.json"
TARGET = "aarch64-linux-android"
APP_PACKAGE = "kmit-app"
FULL_TEXTS_MARK = "======== ПОВНІ ТЕКСТИ ЛІЦЕНЗІЙ ========"  # app/ui/extras.js ділить файл за цим рядком

HEADER = """KMIT — ЛІЦЕНЗІЇ ТА ПОДЯКИ

Програмний код Kmit поширюється за ліцензією PolyForm Noncommercial License 1.0.0:
https://polyformproject.org/licenses/noncommercial/1.0.0

Required Notice: Copyright 2026 Oleksandr Shynkarenko (Олександр Шинкаренко)

Навчальний вміст (курси, завдання, пояснення) поширюється за ліцензією Creative Commons
Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0):
https://creativecommons.org/licenses/by-nc-sa/4.0/

Kmit побудовано на відкритих бібліотеках і шрифтах, перелічених нижче. Дякуємо їхнім авторам!
Сторонні компоненти використано без змін. Повні тексти ліцензій — наприкінці переліку;
посилання на них позначено [L1], [L2] … Для компонентів із вибором ліцензії («A OR B») наведено всі варіанти.
(Файл створено скриптом scripts/licenses.py.)"""

LICENSE_FILE_RE = re.compile(r"^(licen[cs]e|copying|notice|copyright|unlicense)", re.I)
# Рядок Copyright: «Copyright 2020 …», «(c) 2020 …», «© …»; не «Copyright Holder», «Copyright notice», «Copyright:».
COPYRIGHT_RE = re.compile(r"^\s*(?:Copyright(?![ \t]*(?:[Nn]otice|[Aa]nd\b|[Hh]olders?\b|[Oo]wner|:?$))|\([cC]\)\s*\d|©)")
MIT_HEADING_RE = re.compile(r"^\s*(?:The\s+)?MIT\s+Licen[sc]e(?:\s*\(MIT\))?\s*\n(?:[=\-]+\s*\n)?", re.I)
TEMPLATE_RE = re.compile(r"<year>|\[yyyy\]|\{yyyy\}|\[name of copyright owner\]|<copyright holders>|<owner>", re.I)


def fail(msg):
    sys.exit(f"licenses.py: {msg}")


def read_text(path):
    return pathlib.Path(path).read_text(encoding="utf-8", errors="replace")


def clean(text):
    text = text.replace("\r\n", "\n").replace("\r", "\n").lstrip("﻿").replace("\t", "    ")
    out = []
    for ln in text.split("\n"):
        ln = ln.rstrip()
        if ln == "" and out and out[-1] == "":
            continue
        out.append(ln)
    return "\n".join(out).strip("\n")


def norm(text):
    return re.sub(r"\s+", " ", text).strip()


def vkey(v):
    return [(0, int(x), "") if x.isdigit() else (1, 0, x) for x in re.split(r"[.+\-]", v)]


def uniq(xs):
    out = []
    for x in xs:
        if x not in out:
            out.append(x)
    return out


# ---------- тексти ліцензій (кожен один раз) ----------
class Texts:
    def __init__(self, std):
        self.std = {spdx: (info["title"], clean(read_text(ROOT / info["file"]))) for spdx, info in std.items()}
        self.std_norm = {norm(body): spdx for spdx, (_, body) in self.std.items()}
        self.blocks = {}  # ключ -> {"title", "body", "users": set}

    def _use(self, key, title, body, user):
        self.blocks.setdefault(key, {"title": title, "body": body, "users": set()})["users"].add(user)
        return key

    def use_std(self, spdx, user):
        title, body = self.std[spdx]
        return self._use("std:" + spdx, title, body, user)

    def add(self, raw, user, kind="LICENSE"):
        """Повертає (рядки Copyright, ключ тексту)."""
        text = clean(raw)
        if kind in ("NOTICE", "WHOLE"):  # файл наводиться цілком, без виокремлення рядків Copyright
            title = "NOTICE" if kind == "NOTICE" else guess_title(text)
            return [], self._use("h:" + hashlib.sha256(norm(text).encode()).hexdigest(), title, text, user)
        copyright, body_lines = [], []
        for ln in text.split("\n"):
            if COPYRIGHT_RE.match(ln) and len(ln) < 300:
                if not TEMPLATE_RE.search(ln):
                    copyright.append(ln.strip())
                continue
            body_lines.append(ln)
        if "Apache License" in text and "Version 2.0, January 2004" in text and "TERMS AND CONDITIONS" in text \
                and "Permission is hereby granted" not in text:
            return copyright, self.use_std("Apache-2.0", user)
        if re.search(r"Mozilla Public License,? [Vv]ersion 2\.0", text) and len(text) > 10000:
            return copyright, self.use_std("MPL-2.0", user)
        body = clean("\n".join(body_lines))
        spdx = self.std_norm.get(norm(body)) or self.std_norm.get(norm(MIT_HEADING_RE.sub("", body, count=1)))
        if spdx:
            return copyright, self.use_std(spdx, user)
        key = "h:" + hashlib.sha256(norm(body).encode()).hexdigest()
        return copyright, self._use(key, guess_title(body), body, user)

    def numbered(self):
        """Стабільна нумерація: спершу стандартні тексти, далі решта за назвою й першим користувачем."""
        order = {"std:Apache-2.0": 0, "std:MIT": 1, "std:MPL-2.0": 2}
        items = sorted(self.blocks.items(),
                       key=lambda it: (order.get(it[0], 9), it[1]["title"], sorted(it[1]["users"])[0], it[0]))
        return {key: f"L{i}" for i, (key, _) in enumerate(items, 1)}, items


def guess_title(body):
    t = norm(body).lower()
    checks = [
        ("sil open font license", "SIL Open Font License 1.1"),
        ("unicode license v3", "Unicode License v3"),
        ("unicode, inc. license agreement", "Unicode License Agreement"),
        ("this is free and unencumbered software released into the public domain", "The Unlicense"),
        ("permission is hereby granted, free of charge", "MIT License"),
        ("permission to use, copy, modify, and/or distribute this software for any purpose", "ISC / 0BSD License"),
        ("permission to use, copy, modify, and distribute this software for any purpose", "ISC License"),
        ("neither the name", "BSD 3-Clause License"),
        ("redistribution and use in source and binary forms", "BSD 2-Clause License"),
        ("this software is provided 'as-is'", "zlib License"),
        ("cc0 1.0 universal", "Creative Commons CC0 1.0 Universal"),
        ("creative commons", "Creative Commons"),
    ]
    for needle, title in checks:
        if needle in t:
            return title
    return "Інший текст ліцензії"


# ---------- cargo ----------
def cargo_metadata():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--offline", "--locked", "--all-features",
         "--filter-platform", TARGET, "--manifest-path", str(ROOT / "app/src-tauri/Cargo.toml")],
        capture_output=True, text=True, env=os.environ.copy())
    if out.returncode != 0:
        fail("cargo metadata не вдалося (спершу виконайте cargo fetch --locked --manifest-path "
             "app/src-tauri/Cargo.toml):\n" + out.stderr)
    return json.loads(out.stdout)


def linked_packages(meta):
    """Пакети, що компонуються в бінарник: лише звичайні залежності від kmit-app, без proc-macro."""
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    roots = [p["id"] for p in meta["packages"] if p["name"] == APP_PACKAGE and p["source"] is None]
    if not roots:
        fail(f"не знайдено пакет {APP_PACKAGE}")

    def is_proc_macro(p):
        return any("proc-macro" in t["kind"] for t in p["targets"])

    seen, stack = set(), list(roots)
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for d in nodes[pid]["deps"]:
            if any(k["kind"] is None for k in d["dep_kinds"]) and not is_proc_macro(pkgs[d["pkg"]]):
                stack.append(d["pkg"])
    return [pkgs[i] for i in seen]


def crate_dir(p):
    return pathlib.Path(p["manifest_path"]).parent


def crate_license_files(p):
    d = crate_dir(p)
    files = sorted(f for f in d.iterdir() if f.is_file() and LICENSE_FILE_RE.match(f.name))
    if p.get("license_file"):
        lf = (d / p["license_file"]).resolve()
        if lf.is_file() and lf not in [f.resolve() for f in files]:
            files.append(lf)
    return files


# ---------- форматування ----------
def entry(lines, name, version, lic, url, copyright, refs, notes=()):
    lines.append(f"- {name} {version}".rstrip() + f" — {lic}")
    if url:
        lines.append(f"    {url}")
    for n in notes:
        lines.append(f"    {n}")
    for c in uniq(copyright):
        lines.append(f"    {c}")
    if refs:
        lines.append("    Тексти: " + " ".join(f"[{r}]" for r in uniq(refs)))


def generate():
    manual = json.loads(read_text(MANUAL))
    meta = cargo_metadata()
    texts = Texts(manual["standard_texts"])
    by_name = {}
    for p in meta["packages"]:
        if p["source"] is not None:
            cur = by_name.get(p["name"])
            if cur is None or vkey(p["version"]) > vkey(cur["version"]):
                by_name[p["name"]] = p

    def crate(name):
        if name not in by_name:
            fail(f"крейт {name} не знайдено в cargo metadata")
        return by_name[name]

    def resolve_text(spec, user):
        """spec: {"std"} | {"file"} | {"crate", "path"} → (рядки Copyright, ключ тексту)."""
        kind = spec.get("kind", "LICENSE")
        if "std" in spec:
            return [], texts.use_std(spec["std"], user)
        if "file" in spec:
            return texts.add(read_text(ROOT / spec["file"]), user, kind)
        return texts.add(read_text(crate_dir(crate(spec["crate"])) / spec["path"]), user, kind)

    def auto_version(v):
        if not v.startswith("auto:"):
            return v
        what = v[5:]
        if what == "sqlcipher":
            m = re.search(r"#define CIPHER_VERSION_NUMBER (\S+)",
                          read_text(crate_dir(crate("libsqlite3-sys")) / "sqlcipher/sqlite3.c"))
        elif what == "sqlite":
            m = re.search(r'#define SQLITE_VERSION\s+"([^"]+)"',
                          read_text(crate_dir(crate("libsqlite3-sys")) / "sqlcipher/sqlite3.h"))
        elif what == "openssl":
            m = re.search(r"\+(.+)$", crate("openssl-src")["version"])
        elif what.startswith("crate:"):
            return crate(what[6:])["version"]
        else:
            fail(f"невідоме джерело версії {v}")
        if not m:
            fail(f"не вдалося визначити версію {v}")
        return m.group(1)

    def manual_entry(lines, e, user):
        cps, refs = list(e.get("copyright", [])), []
        for spec in e.get("texts", []):
            c, k = resolve_text(spec, user)
            cps += c
            refs.append(k)
        entry(lines, e["name"], auto_version(e.get("version", "")), e["license"], e.get("url"), cps, refs,
              e.get("notes", []))

    linked = linked_packages(meta)
    linked_names = {p["name"] for p in linked}
    third = sorted((p for p in linked if p["source"] is not None), key=lambda p: (p["name"], vkey(p["version"])))
    sections = []  # (назва, рядки, кількість)

    # C-бібліотеки, зібрані build-скриптами
    c_lines = []
    for e in manual["c_libraries"]:
        if e.get("requires_crate") and e["requires_crate"] not in linked_names:
            continue
        manual_entry(c_lines, e, e["name"])
    sections.append(("БІБЛІОТЕКИ C, ЗІБРАНІ РАЗОМ ІЗ ЗАСТОСУНКОМ", c_lines))

    # Android / JVM
    a = manual["android"]
    artifacts = {}
    for spec in a["gradle_files"]:
        path = ROOT / spec["file"] if "file" in spec else crate_dir(crate(spec["crate"])) / spec["path"]
        for m in re.finditer(r'^\s*implementation\("([^:"]+):([^:"]+):([^"]+)"\)', read_text(path), re.M):
            ga, ver = f"{m.group(1)}:{m.group(2)}", m.group(3)
            if ga not in artifacts or vkey(ver) > vkey(artifacts[ga]):
                artifacts[ga] = ver
    km = re.search(r"kotlin-gradle-plugin:([0-9][^\"]*)", read_text(ROOT / a["kotlin_plugin_file"]))
    if not km:
        fail("не знайдено версію kotlin-gradle-plugin")
    artifacts["org.jetbrains.kotlin:kotlin-stdlib"] = km.group(1)
    for x in a["extra_artifacts"]:
        if x["version_like"] in artifacts:
            artifacts[x["coord"]] = artifacts[x["version_like"]]
    an_lines = []
    for ga in sorted(artifacts):
        grp = next((g for g in a["groups"] if ga.startswith(g["prefix"])), None)
        if grp is None:
            fail(f"для бібліотеки Android {ga} немає опису ліцензії в scripts/third_party/manual.json (android.groups)")
        refs = [texts.use_std(grp["license"], ga)] if grp["license"] in texts.std else []
        cps = list(grp.get("copyright", []))
        for spec in a.get("artifact_texts", {}).get(ga, []):
            c, k = resolve_text(spec, ga)
            cps += c
            refs.append(k)
        entry(an_lines, ga, artifacts[ga], grp["license"], grp.get("url"), cps, refs)
    for e in a["extra_components"]:
        manual_entry(an_lines, e, e["name"])
    sections.append(("БІБЛІОТЕКИ ANDROID (JVM)", an_lines))
    an_lines += ["", a["transitive_note"]]

    # Rust
    overrides = manual.get("crate_overrides", {})
    r_lines, warnings = [], []
    for p in third:
        user = f"{p['name']} {p['version']}"
        lic = p.get("license") or "ліцензія не вказана"
        ov = overrides.get(p["name"], {})
        src = crate(ov["texts_from_crate"]) if "texts_from_crate" in ov else p
        cps, refs, notes = [], [], []
        for f in crate_license_files(src):
            c, k = texts.add(read_text(f), user, "NOTICE" if f.name.upper().startswith("NOTICE") else "LICENSE")
            cps += c
            refs.append(k)
        if not refs:  # у пакеті немає файлів ліцензії — стандартні тексти за SPDX-виразом
            for spdx in ("Apache-2.0", "MPL-2.0"):
                if re.search(rf"\b{re.escape(spdx)}\b", lic):
                    refs.append(texts.use_std(spdx, user))
            if not refs:
                warnings.append(f"{user}: у пакеті немає файла ліцензії ({lic})")
        if not cps:
            authors = [re.sub(r"\s*<[^>]*>", "", x).strip() for x in p.get("authors", [])]
            if authors:
                cps.append("Автори (з Cargo.toml): " + ", ".join(authors))
        if "MPL" in lic:
            notes.append(f"Використано без змін. Вихідний код: https://crates.io/crates/{p['name']}/{p['version']}")
        if ov.get("note"):
            notes.append(ov["note"])
        if p["name"] == "libsqlite3-sys":
            notes.append("Містить вихідний код SQLCipher і SQLite (див. розділ 1).")
        if lic == "ліцензія не вказана":
            warnings.append(f"{user}: ліцензію не вказано в Cargo.toml")
        entry(r_lines, p["name"], p["version"], lic, p.get("repository") or "", cps, refs, notes)
    sections.append((f"БІБЛІОТЕКИ RUST (ціль {TARGET})", r_lines))

    # Шрифти
    f_lines = []
    for e in manual["fonts"]:
        manual_entry(f_lines, e, "шрифт " + e["name"])
    sections.append(("ШРИФТИ", f_lines))

    # Нумерація текстів і збирання файла
    ids, items = texts.numbered()
    count = [sum(1 for ln in body if ln.startswith("- ")) for _, body in sections]
    out = [HEADER, "", "ЗМІСТ"]
    for i, ((title, _), n) in enumerate(zip(sections, count), 1):
        out.append(f"{i}. {title} — {n}")
    out.append(f"{len(sections) + 1}. ПОВНІ ТЕКСТИ ЛІЦЕНЗІЙ — {len(items)}")
    for i, ((title, body), n) in enumerate(zip(sections, count), 1):
        out += ["", "", f"{i}. {title} ({n})", ""]
        for ln in body:
            if ln.startswith("    Тексти: "):
                ln = re.sub(r"\[([^\]]+)\]", lambda m: f"[{ids[m.group(1)]}]", ln)
            out.append(ln)
    out += ["", "", FULL_TEXTS_MARK]
    for key, b in items:
        out += ["", "", "-" * 72, f"[{ids[key]}] {b['title']}",
                "Застосовується до: " + ", ".join(sorted(b["users"])), "-" * 72, "", b["body"]]
    text = "\n".join(out) + "\n"
    return text, {"rust": count[2], "android": count[1], "texts": len(items), "warnings": warnings}


def main():
    check = "--check" in sys.argv[1:]
    text, st = generate()
    for w in st["warnings"]:
        print("УВАГА:", w, file=sys.stderr)
    rel = DEST.relative_to(ROOT)
    if check:
        cur = DEST.read_text(encoding="utf-8") if DEST.exists() else ""
        if cur != text:
            print(f"{rel} застарів: виконайте python3 scripts/licenses.py і закомітьте результат", file=sys.stderr)
            sys.exit(1)
        print(f"OK: {rel} актуальний")
        return
    DEST.write_text(text, encoding="utf-8")
    print(f"записано {rel}: {len(text.encode())} байт; крейтів Rust {st['rust']}, "
          f"бібліотек Android {st['android']}, текстів ліцензій {st['texts']}")


if __name__ == "__main__":
    main()
