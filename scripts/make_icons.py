#!/usr/bin/env python3
"""Малює значки застосунків (PIL): Kmit (рукописна «K» на зошитовому аркуші) і Kmit Методист (аркуш із позначкою).

  scripts/make_icons.py kmit      -> app/src-tauri/icons/icon.png (512, для магазину) і app/src-tauri/gen/android/app/src/main/res/*
  scripts/make_icons.py reviewer  -> app/src-tauri/icons-reviewer/res/* (підставляється скриптом збірки методиста)
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent / 'app' / 'src-tauri'
DENS = {'mdpi': 1, 'hdpi': 1.5, 'xhdpi': 2, 'xxhdpi': 3, 'xxxhdpi': 4}
KINDS = {
    'kmit': dict(bg=(47, 91, 234), bg2=(27, 42, 107), page=(255, 255, 255), grid=(201, 214, 245), mark=(224, 69, 58), margin=(224, 69, 58)),
    'reviewer': dict(bg=(14, 106, 98), bg2=(8, 66, 60), page=(255, 255, 255), grid=(190, 222, 218), mark=(255, 210, 63), margin=(14, 106, 98)),
}
S = 1024  # робоче полотно, потім зменшуємо (згладжування)


FONT = Path(__file__).resolve().parent.parent / 'app' / 'ui' / 'fonts' / 'Caveat-700.ttf'
INK = (27, 42, 107)


def art(kind, cfg):
    """Лише малюнок (без фону) на прозорому полотні S×S, у межах 66 % (безпечна зона адаптивного значка):
    аркуш у клітинку з рукописною «K» (шрифт Caveat, як у застосунку) і червоним хвилястим підкресленням."""
    im = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    pw, ph = 520, 600
    x0, y0 = (S - pw) // 2, (S - ph) // 2 - 6
    d.rounded_rectangle((x0 + 14, y0 + 18, x0 + pw + 14, y0 + ph + 18), 50, fill=(0, 0, 0, 60))
    d.rounded_rectangle((x0, y0, x0 + pw, y0 + ph), 50, fill=cfg['page'])
    step = 65
    for i in range(1, 9):
        d.line((x0 + 26, y0 + i * step + 12, x0 + pw - 26, y0 + i * step + 12), fill=cfg['grid'], width=5)
    for i in range(1, 8):
        d.line((x0 + i * step + 10, y0 + 30, x0 + i * step + 10, y0 + ph - 30), fill=cfg['grid'], width=5)
    font = ImageFont.truetype(str(FONT), 620)
    ink = INK if kind == 'kmit' else cfg['bg2']
    bb = d.textbbox((0, 0), 'K', font=font)
    tw, th = bb[2] - bb[0], bb[3] - bb[1]
    tx, ty = x0 + (pw - tw) // 2 - bb[0], y0 + 36 - bb[1]
    d.text((tx, ty), 'K', font=font, fill=ink)
    # хвилясте підкреслення («як учитель підкреслив»)
    import math
    uy, ux0, ux1 = y0 + ph - 92, x0 + 90, x0 + pw - 90
    pts = [(x, uy + 13 * math.sin((x - ux0) / 17)) for x in range(ux0, ux1 + 1, 4)]
    d.line(pts, fill=cfg['mark'], width=15, joint='curve')
    if kind != 'kmit':  # методист: маленька позначка-галочка на куті аркуша
        cx, cy, r = x0 + pw - 24, y0 + ph - 24, 100
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=cfg['mark'], outline=cfg['page'], width=14)
        d.line([(cx - 44, cy + 2), (cx - 12, cy + 36), (cx + 48, cy - 38)], fill=cfg['bg2'], width=28, joint='curve')
    return im


def compose(kind, cfg, shape):
    """Значок із фоном: shape = 'square' (закруглений) | 'round'."""
    bg = Image.new('RGBA', (S, S), cfg['bg'])
    g = ImageDraw.Draw(bg)
    for y in range(S):  # м'який перехід до темнішого низу
        t = y / S
        g.line((0, y, S, y), fill=tuple(int(cfg['bg'][i] * (1 - t * 0.6) + cfg['bg2'][i] * t * 0.6) for i in range(3)) + (255,))
    bg.alpha_composite(art(kind, cfg).resize((S, S)))
    mask = Image.new('L', (S, S), 0)
    m = ImageDraw.Draw(mask)
    if shape == 'round':
        m.ellipse((0, 0, S, S), fill=255)
    else:
        m.rounded_rectangle((0, 0, S, S), 200, fill=255)
    bg.putalpha(mask)
    return bg


def write_res(kind, cfg, out):
    out.mkdir(parents=True, exist_ok=True)
    for name, k in DENS.items():
        d = out / f'mipmap-{name}'
        d.mkdir(parents=True, exist_ok=True)
        legacy = int(48 * k)
        compose(kind, cfg, 'square').resize((legacy, legacy), Image.LANCZOS).save(d / 'ic_launcher.png')
        compose(kind, cfg, 'round').resize((legacy, legacy), Image.LANCZOS).save(d / 'ic_launcher_round.png')
        fg = int(108 * k)
        art(kind, cfg).resize((fg, fg), Image.LANCZOS).save(d / 'ic_launcher_foreground.png')
    any_ = out / 'mipmap-anydpi-v26'
    any_.mkdir(parents=True, exist_ok=True)
    for n in ('ic_launcher', 'ic_launcher_round'):
        (any_ / f'{n}.xml').write_text(
            '<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
            '    <background android:drawable="@drawable/ic_launcher_background"/>\n'
            '    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n</adaptive-icon>\n')
    bgxml = out / 'drawable'
    bgxml.mkdir(parents=True, exist_ok=True)
    (bgxml / 'ic_launcher_background.xml').write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n<shape xmlns:android="http://schemas.android.com/apk/res/android" android:shape="rectangle">\n'
        f'    <gradient android:angle="270" android:startColor="#{cfg["bg"][0]:02X}{cfg["bg"][1]:02X}{cfg["bg"][2]:02X}" '
        f'android:endColor="#{cfg["bg2"][0]:02X}{cfg["bg2"][1]:02X}{cfg["bg2"][2]:02X}"/>\n</shape>\n')


def main():
    kind = sys.argv[1] if len(sys.argv) > 1 else 'kmit'
    cfg = KINDS[kind]
    if kind == 'kmit':
        compose(kind, cfg, 'square').resize((512, 512), Image.LANCZOS).save(ROOT / 'icons' / 'icon.png')
        write_res(kind, cfg, ROOT / 'gen' / 'android' / 'app' / 'src' / 'main' / 'res')
    else:
        write_res(kind, cfg, ROOT / 'icons-reviewer' / 'res')
        compose(kind, cfg, 'square').resize((512, 512), Image.LANCZOS).save(ROOT / 'icons-reviewer' / 'icon-512.png')
    print('OK', kind)


main()
