# Презентація, лендінг і промо-ролик

Усе зібране з **реальних екранів застосунку** (тестові дані) і не містить трекерів, зовнішніх шрифтів чи скриптів.

| Файл | Що це |
|---|---|
| `index.html` | **Лендінг**: один самодостатній файл (шрифти й знімки вбудовано), світла й темна теми. Контакт для пілоту задано в `CONTACT_EMAIL` внизу файлу `src/landing.template.html`; після зміни перезберіть (див. нижче) |
| `presentation.html`, `Kmit-presentation.pdf` | **Презентація** пілоту (13 слайдів, 16:9). У браузері: стрілки або клік; PDF для показу й розсилки |
| `promo/promo.template.html`, `promo/poster.jpg` | Джерело **промо-ролика** (60 с, 1920×1080, без звуку) і кадр для прев'ю. Сам ролик (`promo/kmit-promo.mp4`) у репозиторій не входить; його можна зібрати `site/tools/make_promo.sh` |
| `assets/shots/` | знімки екранів 780×1688 (DPR 2) з реального застосунку |
| `assets/web/` | ті самі знімки в WebP для вбудовування |
| `assets/worlds-fragment.html` | істоти восьми світів (SVG) із `app/ui/art.js` |

## Як перебудувати
```bash
# 1) нові знімки екранів (якщо змінився інтерфейс)
CHROME=/шлях/до/chrome site/tools/capture.sh
python3 - <<'PY'   # стиснути в WebP (560 px)
from PIL import Image; import glob, os
for f in glob.glob('site/assets/shots/*.png'):
    im = Image.open(f).convert('RGB'); w = 560; im.resize((w, round(im.height*w/im.width))).save('site/assets/web/'+os.path.basename(f)[:-4]+'.webp', 'WEBP', quality=84, method=6)
PY
# 2) лендінг і презентація (вбудовує шрифти, знімки й істот)
python3 site/tools/build_site.py
# 3) PDF презентації
chrome --headless=new --no-pdf-header-footer --print-to-pdf=site/Kmit-presentation.pdf file://$PWD/site/presentation.html
# 4) ролик (кадри через Chromium, кодування через ffmpeg; ≈ 15–25 хв)
CHROME=/шлях/до/chrome site/tools/make_promo.sh
```
Джерела: `src/*.template.html` (лендінг і презентація), `promo/promo.template.html` (ролик, часова шкала в `window.__seek(t)`).

## Правила змісту
Лише те, що справді є в застосунку й підтверджено: жодних вигаданих відгуків, цифр чи результатів навчання; «чернетка» вмісту названа прямо; без обіцянок результатів НМТ; без реальних дітей на знімках (профіль «Оля» тестовий). Вміст створено за допомогою ШІ й ще не вичитано методистами: так і пишемо, без тверджень на кшталт «матеріалам можна довіряти». Перед публікацією лендінгу перегляньте текст юристом разом із політикою конфіденційності.
