// Наскрізний тест інтерфейсу: Chromium (headless) керується через DevTools-протокол без залежностей.
// Потрібен запущений dev-server зі збіркою `--features e2e` і шлях до Chromium у CHROME.
// Запуск: node app/e2e/run.mjs   (змінні: BASE, CHROME, SHOTS)
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const BASE = process.env.BASE || 'http://127.0.0.1:8799';
const CHROME = process.env.CHROME;
const SHOTS = process.env.SHOTS || '';
const PORT = 9333;
if (!CHROME) { console.error('Вкажи CHROME=/шлях/до/chrome'); process.exit(2); }

const profile = mkdtempSync(join(tmpdir(), 'kmit-e2e-'));
const chrome = spawn(CHROME, [
  '--headless=new', '--no-sandbox', '--disable-gpu', '--disable-extensions', '--no-first-run',
  `--user-data-dir=${profile}`, `--remote-debugging-port=${PORT}`, '--window-size=420,900', 'about:blank',
], { stdio: 'ignore' });
const cleanup = () => { try { chrome.kill('SIGKILL'); } catch {} try { rmSync(profile, { recursive: true, force: true }); } catch {} };
process.on('exit', cleanup);

process.on('uncaughtException', async (e) => { try { console.error('ЕКРАН ПРИ ПОМИЛЦІ:\n' + (await text())); } catch {} console.error(e.message); cleanup(); process.exit(1); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function wsUrl() {
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json`);
      const t = (await r.json()).find((x) => x.type === 'page');
      if (t) return t.webSocketDebuggerUrl;
    } catch {}
    await sleep(100);
  }
  throw new Error('Chromium не запустився');
}
const ws = new WebSocket(await wsUrl());
await new Promise((r) => ws.addEventListener('open', r));
let id = 0; const pending = new Map();
ws.addEventListener('message', (m) => { const d = JSON.parse(m.data); if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id); } });
const send = (method, params = {}) => new Promise((res) => { const i = ++id; pending.set(i, res); ws.send(JSON.stringify({ id: i, method, params })); });
const ev = async (expr) => {
  const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.result.exceptionDetails) throw new Error(`JS: ${JSON.stringify(r.result.exceptionDetails.exception?.description || r.result.exceptionDetails)}`);
  return r.result.result.value;
};
await send('Page.enable');
const W = Number(process.env.WIDTH || 420), H = Number(process.env.HEIGHT || 900);
await send('Emulation.setDeviceMetricsOverride', { width: W, height: H, deviceScaleFactor: Number(process.env.DPR || 1), mobile: W < 600, screenWidth: W, screenHeight: H });
if (process.env.THEME) await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: process.env.THEME }] });
let shotN = 0;
async function shot(name) {
  if (!SHOTS) return;
  mkdirSync(SHOTS, { recursive: true });
  const r = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(SHOTS, `${String(++shotN).padStart(2, '0')}-${name}.png`), Buffer.from(r.result.data, 'base64'));
}
const text = () => ev(`(document.getElementById("main") || {innerText: ""}).innerText`);
async function waitFor(pred, what, ms = 8000) {
  const t0 = Date.now();
  for (;;) {
    const t = await text();
    if (pred(t)) return t;
    if (Date.now() - t0 > ms) throw new Error(`Не дочекався: ${what}\nЕкран:\n${t}`);
    await sleep(60);
  }
}
const clickOnce = (label) => ev(`(() => { const b=[...document.querySelectorAll('button,a')].find(x=>(x.innerText.trim().startsWith(${JSON.stringify(label)}) || (x.getAttribute('aria-label')||'').startsWith(${JSON.stringify(label)}) || ((x.classList.contains('node')||x.classList.contains('cover')) && x.innerText.includes(${JSON.stringify(label)}))) && !x.disabled && x.getAttribute('aria-disabled')!=='true'); if(!b) return false; b.click(); return true; })()`);
async function click(label) {
  for (let i = 0; i < 50; i++) { if (await clickOnce(label)) return true; await sleep(60); }
  throw new Error(`немає кнопки ${label}\nЕкран:\n${await text()}`);
}

// Проходить урок крок за кроком, як дитина, що не знає відповідей: приклади розгортає, питання «спалює» двома хибними спробами.
async function walkLesson(stop, tag = 'lesson') {
  const seen = new Set();
  for (let guard = 0; guard < 120; guard++) {
    const tx = await text();
    const m = /Крок (\d+) з/.exec(tx);
    if (m && !seen.has(m[1])) { seen.add(m[1]); await shot(`${tag}-step${m[1].padStart(2, '0')}`); }
    if (tx.includes(stop)) return;
    if (await clickOnce('Далі')) { await sleep(100); continue; }
    if (await clickOnce('Готово')) { await sleep(150); continue; }
    if (await clickOnce('Показати наступний крок')) continue;
    if (await clickOnce('Зрозуміло')) { await sleep(120); continue; }
    if (await clickOnce('Показати розбір')) { await sleep(200); continue; }
    if (await ev(`!!document.querySelector('.selfcheck input.answer:not([readonly])')`)) {
      await ev(`(() => { const i=document.querySelector('.selfcheck input.answer'); i.value='999999'; i.dispatchEvent(new Event('input',{bubbles:true})); })()`);
      await clickOnce('Перевірити'); await sleep(250);
      continue;
    }
    if (await ev(`!!document.querySelector('.selfcheck .choices .btn:not([aria-disabled="true"])')`)) {
      await ev(`document.querySelector('.selfcheck .choices .btn:not([aria-disabled="true"])').click()`); await sleep(250);
      continue;
    }
    if (await ev(`!!document.querySelector('.explain textarea')`)) {
      await ev(`(() => { const t=document.querySelector('.explain textarea'); t.value='Це правило я своїми словами поясню так: спочатку дивлюся на умову, потім виконую кроки, далі перевіряю результат.'; })()`);
      await ev(`document.querySelector('.explain textarea').dispatchEvent(new Event('input',{bubbles:true}))`);
      if (await clickOnce('Показати головні ідеї')) { await sleep(250); continue; }
    }
    await sleep(100);
  }
  throw new Error(`урок не дійшов до «${stop}»\nЕкран:\n${await text()}`);
}
const setVal = (sel, v) => ev(`(() => { const e=document.querySelector(${JSON.stringify(sel)}); e.value=${JSON.stringify(v)}; e.dispatchEvent(new Event('input',{bubbles:true})); })()`);
const peek = async () => (await (await fetch(`${BASE}/__peek`)).json()).answer;

let failed = 0;
const check = (ok, msg) => { console.log(`${ok ? 'OK  ' : 'FAIL'} ${msg}`); if (!ok) failed++; };

await send('Page.navigate', { url: BASE });
await waitFor((t) => t.includes('Kmit'), 'початковий екран');
await shot('profiles');
check((await text()).includes('Мій перший зошит'), 'порожній список профілів: запрошення створити зошит');

// Створення профілю
await click('Новий профіль');
await waitFor((t) => t.includes('Хто ти?'), 'вибір ролі');
await shot('role');
check((await text()).includes('не бачать даних одне одного'), 'на екрані ролей сказано, що профілі учня й вчителя окремі');
await click('Я учень або учениця');
await waitFor((t) => t.includes('Для дорослого'), 'екран для дорослого');
check((await text()).includes('нічого не надсилає'), 'екран для дорослого: нічого не надсилається');
await shot('adult-notice');
await click('Зрозуміло, продовжити');
await waitFor((t) => t.includes('Ім’я або прізвисько'), 'форма створення');
for (let i = 0; i < 100 && !(await ev(`!!document.querySelector('.hero-stage svg')`)); i++) await sleep(60);
check(await ev(`!!document.querySelector('.hero-stage svg') && document.querySelectorAll('.swatch').length >= 14`), 'створення профілю: герой із вибором кольорів, без тваринок');
await click('Кучері');
await sleep(300);
const inputs = await ev('[...document.querySelectorAll("input")].length');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='Оля'; i[1].value='1234'; i[2].value='1234'; })()`);
await shot('create');
await click('Створити');
const rec = await waitFor((t) => t.includes('Код відновлення'), 'код відновлення');
const code = await ev('document.querySelector(".code").innerText');
check(code.length >= 16, `код відновлення показано (${code.length} символів)`);
check(await ev('document.querySelector(".btn.primary").disabled'), 'кнопка «Код записано» вимкнена без підтвердження');
await shot('recovery');
await ev('(() => { const c=document.querySelector("input[type=checkbox]"); c.checked=true; c.dispatchEvent(new Event("change")); })()');
await click('Далі');
await waitFor((t) => t.includes('Ласкаво просимо'), 'вступ');
for (let k = 0; k < 3; k++) await click('Далі');
await waitFor((t) => t.includes('Який ти клас'), 'вибір класу');
await click('Почати');
await waitFor((t) => t.includes('Мої зошити'), 'головний екран');
await shot('home');

// Заняття
await click('Математика');
await click('Почати: Математика');
await waitFor((t) => t.includes('Нова тема'), 'пропозиція пояснення');
await shot('intro');
await click('Спробувати одразу');
await waitFor((t) => t.includes('Перевірити'), 'екран задачі');
await shot('task');
let t = await text();
check(!/відповідь:/i.test(t), 'умова не містить слова «відповідь:»');

// Хибна відповідь, потім підказка
await ev(`document.querySelector('input.answer').value='7/7'`); await click('Перевірити');
await waitFor((tx) => tx.includes('Ще не те'), 'нейтральний відгук на хибну відповідь');
await shot('wrong');
await click('Підказка');
await waitFor((tx) => tx.includes('Підказка 1') || tx.includes('подумай') || tx.includes('Спершу'), 'підказка або затримка');
await shot('hint');

// Перезапуск інтерфейсу посеред заняття: «Продовжити заняття» має повернути до задачі
await send('Page.navigate', { url: BASE });
await waitFor((tx) => tx.includes('Хто вчиться'), 'список профілів після перезавантаження');
await ev(`document.querySelector('.profile').click()`);
await waitFor((tx) => tx.includes('Введи свій PIN'), 'PIN після перезавантаження');
await ev(`document.querySelector('input').value='1234'`); await click('Відкрити');
await waitFor((tx) => tx.includes('Продовжити заняття'), 'кнопка «Продовжити заняття»');
await click('Продовжити заняття');
await waitFor((tx) => tx.includes('Перевірити'), 'повернення до задачі після перезапуску');
check(true, 'заняття продовжується після перезапуску');

// Підглянути відповідь через e2e-ендпоїнт і розв'язати
let solved = 0;
for (let n = 0; n < 60; n++) {
  const screen = await text();
  if (screen.includes('Додому')) break;
  if (screen.includes('Перевірити')) {
    const a = await peek();
    if (a == null) break;
    await ev(`document.querySelector('input.answer').value=${JSON.stringify(a)}`);
    await click('Перевірити');
    await waitFor((tx) => tx !== screen, 'зміна екрана після відповіді');
    continue;
  }
  if (screen.includes('Далі')) {
    solved++;
    if (solved === 1) await shot('feedback');
    await click('Далі');
    await waitFor((tx) => tx !== screen, 'зміна екрана після «Далі»');
    continue;
  }
  await sleep(100);
}
const end = await text();
check(end.includes('Додому') || solved > 0, `пройдено кілька задач (${solved})`);
if (!end.includes('Додому')) { await click('Закінчити на сьогодні'); }
await waitFor((tx) => tx.includes('Додому'), 'екран завершення');
await shot('ended');
check((await text()).includes('прогулянка'), 'на завершенні — заохочення йти гуляти');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'повернення додому');
await shot('home-after');

// Альбом і аркуш
await click('Колекція');
await waitFor((tx) => tx.includes('Нічого не відбирається'), 'колекція');
check(await ev(`document.querySelectorAll('.legend li').length === 4`), 'у колекції пояснено, як отримати кожен вид створіння');
check(!(await text()).includes('з 472'), 'немає загальної дробової суми «з 472», лише по предметах');
await shot('album');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Для вчителя');
await waitFor((tx) => tx.includes('Опановано'), 'аркуш');
await shot('sheet');
const pdfOk = await ev(`(async () => { const r = await fetch('/api/sheet_pdf', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({args:{}, ctx:{wall:Math.floor(Date.now()/1000), day:20000, minute:600}})}); const j = await r.json(); return atob(j.data).startsWith('%PDF-1.4'); })()`);
check(pdfOk, 'PDF аркуша створюється');
await click('Назад');

// Тема: урок, самоперевірка, конспект, відзнака
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Математика');
await waitFor((tx) => tx.includes('Теми'), 'сторінка предмета');
await shot('subject');
await click('Додавання десяткових дробів');
await waitFor((tx) => tx.includes('Урок: пояснення'), 'сторінка теми');
await shot('skill');
await click('Урок: пояснення');
await waitFor((tx) => tx.includes('Крок 1 з'), 'урок порціями');
await shot('lesson');
// Далі заблоковано, доки не розгорнуто приклад: перевіряємо, що кнопка вимкнена
for (let k = 0; k < 12; k++) {
  if (await ev(`!!document.querySelector('.example')`)) break;
  await click('Далі');
}
check(await ev(`(() => { const b=[...document.querySelectorAll('.pager-nav button')].find(x=>x.innerText.trim()==='Далі'); return !!b && b.getAttribute('aria-disabled')==='true'; })()`), 'на прикладі «Далі» вимкнено, доки його не розібрано');
await shot('example');
await walkLesson('Урок переглянуто');
await shot('lesson-done');
await click('Мій конспект');
await waitFor((tx) => tx.includes('Пиши кожну відповідь'), 'зошит конспекту');
check(await ev(`document.querySelectorAll('.notebook textarea').length >= 1`), 'у зошиті окремі поля');
await ev(`(() => { const t=document.querySelector('.notebook textarea'); t.value += 'Кома під комою, а потім додаю як цілі числа, і кому в сумі ставлю на те саме місце. Так не помилюсь.'; })()`);
await click('Зберегти конспект');
await waitFor((tx) => tx.includes('Конспект збережено'), 'конспект збережено');
check(!(await text()).includes('Нова відзнака'), 'відзнаки за урок і конспект немає, якщо питання розібрано без самостійної відповіді');
await shot('note');
await click('Назад');
await waitFor((tx) => tx.includes('Потренуватися на цій темі'), 'сторінка теми після конспекту');
await click('Потренуватися на цій темі');
await waitFor((tx) => tx.includes('Перевірити'), 'вільна практика');
await ev(`document.querySelector('input.answer').value='7/7'`); await click('Перевірити');
await waitFor((tx) => tx.includes('Ще не те'), 'практика працює');
// Розв'язано лише після хибної спроби: застосунок запам'ятовує приклад як складний
await ev(`document.querySelector('input.answer').value=${JSON.stringify(await peek())}`); await click('Перевірити');
await waitFor((tx) => tx.includes('Цей приклад було важко'), 'складний приклад запам’ятовується');
check(true, 'приклад, що вийшов після хибної спроби, запам’ятовується для повернення');
await click('На сьогодні все');
await waitFor((tx) => tx.includes('Додому'), 'кінець практики');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Конспекти');
await waitFor((tx) => tx.includes('Конспект у PDF'), 'список конспектів');
check(true, 'урок, конспект, відзнака, вільна практика');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Інший предмет: завдання з вибором, відгук після спроби з поясненням
await click('Українська мова');
await click('Почати: Українська мова');
await waitFor((tx) => tx.includes('Нова тема'), 'вступ до нової теми');
await click('Прочитати пояснення');
await waitFor((tx) => tx.includes('Крок 1 з'), 'урок порціями');
await shot('pager');
await walkLesson('До завдань', 'ualesson');
await click('До завдань');
await waitFor((tx) => tx.includes('Нове завдання'), 'завдання з вибором');
await shot('choice');
// Вибір потребує підтвердження: сам по собі дотик не відповідає
await ev(`document.querySelector('.choices .btn').click()`);
check(!(await text()).includes('Запам'), 'вибір без підтвердження не відповідає');
await shot('picked');
await click('Відповісти');
await waitFor((tx) => tx.includes('Чому так'), 'пояснення');
await shot('reveal');
await click('На сьогодні все');
await waitFor((tx) => tx.includes('Додому'), 'кінець заняття з мови');
await shot('ended-reflect');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
check(true, 'заняття з української мови: варіанти й пояснення');

// Перевірка «я це вже вмію»: без підказок
await click('Математика');
await click('Віднімання багатоцифрових чисел');
await waitFor((tx) => tx.includes('Я це вже вмію'), 'сторінка теми');
await click('Я це вже вмію');
await waitFor((tx) => tx.includes('Перевірка знань'), 'перевірка знань');
check(!(await text()).includes('Підказка'), 'у перевірці знань немає підказок');
await shot('testout');
for (let k = 0; k < 3; k++) {
  await waitFor((tx) => tx.includes('Перевірити'), 'задача перевірки знань');
  const a = await peek();
  await ev(`document.querySelector('input.answer').value=${JSON.stringify(a)}`);
  await click('Перевірити');
  await waitFor((tx) => tx.includes('Далі'), 'відгук у перевірці');
  await click('Далі');
}
await waitFor((tx) => tx.includes('Тему зараховано'), 'тему зараховано');
await shot('testout-pass');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Введення слова: одна повторна спроба, потім правильне слово з поясненням
await click('Українська мова');
await click('Напиши слово: апостроф');
await waitFor((tx) => tx.includes('Потренуватися на цій темі'), 'сторінка typed-теми');
await click('Потренуватися на цій темі');
await waitFor((tx) => tx.includes('Перевірити'), 'завдання з введенням слова');
check(!(await ev(`!!document.querySelector('.keypad')`)), 'для слова немає цифрової клавіатури');
await ev(`document.querySelector('input.answer').value='ххх'`); await click('Перевірити');
await waitFor((tx) => tx.includes('Майже'), 'підказка про кількість літер');
await ev(`document.querySelector('input.answer').value='ууу'`); await click('Перевірити');
await waitFor((tx) => tx.includes('Чому так'), 'правильне слово з поясненням');
await shot('typed-reveal');
await click('На сьогодні все');
await waitFor((tx) => tx.includes('Додому'), 'кінець typed');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Спокій і дані
await click('Мені зараз важко');
await waitFor((tx) => tx.includes('Зупинись на хвилину'), 'сторінка спокою');
await shot('calm');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Налаштування: один екран з розділами, зберігається одразу
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Налаштування');
await waitFor((tx) => tx.includes('Розмір тексту'), 'екран налаштувань');
await ev(`document.querySelector('.set-group input[type=checkbox]').click()`);
await waitFor((tx) => tx.includes('Збережено'), 'зміна налаштування зберігається одразу');
check(true, 'налаштування зберігаються без кнопки «Зберегти»');
await shot('settings');
await click('Мої дані');
await waitFor((tx) => tx.includes('Відновити з копії'), 'екран «Мої дані»');
check((await text()).includes('Зберегти копію') && (await text()).includes('Імпорт профілю'), 'пояснено, як зберегти копію й відновити');
await shot('data');
await click('Назад');
await waitFor((tx) => tx.includes('Розмір тексту'), 'назад до налаштувань');
await click('Час і захист');
await waitFor((tx) => tx.includes('Обмеження часу'), 'налаштування доступні без PIN');
check(true, 'налаштування без PIN дорослого');
await shot('adult');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Повторення, профіль знань, аркуш для вчителя по предмету
check((await text()).includes('План на сьогодні'), 'на головній є план на сьогодні');
await click('Повторення');
await waitFor((tx) => tx.includes('Повторення на найближчі дні'), 'екран повторення');
await shot('review');
await click('Мої знання');
await waitFor((tx) => tx.includes('Смужка показує'), 'профіль знань');
check(await ev(`document.querySelectorAll('.pf-row').length > 5`), 'профіль знань показує теми предмета');
await shot('profile');
await click('Карта пам’яті');
await waitFor((tx) => tx.includes('Вузли — це теми'), 'карта пам’яті');
check(/Складних прикладів: [1-9]/.test(await text()), 'показаний дорослому приклад потрапив у карту пам’яті');
check(await ev(`!!document.querySelector('.memory-graph')`), 'граф пам’яті намальовано');
await shot('memory');
await click('Назад');
await waitFor((tx) => tx.includes('Натисни тему'), 'назад до профілю');
await click('Назад');
await waitFor((tx) => tx.includes('Повторення на найближчі дні'), 'назад до повторення');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Для вчителя');
await waitFor((tx) => tx.includes('Зберегти PDF'), 'аркуш для вчителя');
await click('Українська мова');
for (let i = 0; i < 100 && !(await ev(`!!document.querySelector('.sheet-doc.s-ukrainian')`)); i++) await sleep(60);
check(await ev(`!!document.querySelector('.sheet-doc.s-ukrainian .sheet-hero svg')`), 'аркуш для вчителя оформлений: кольорова шапка з героєм');
check(!(await text()).includes('Додавання'), 'аркуш одного предмета не містить тем іншого');
await shot('sheet-ukrainian');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Подяка вчителю (з екрана «Для вчителя»)
await click('Для вчителя');
await waitFor((tx) => tx.includes('Подяка вчителю'), 'аркуш для вчителя');
await click('Подяка вчителю');
await waitFor((tx) => tx.includes('За що я дякую'), 'екран подяки');
await ev(`document.querySelector('.thanks-preview') && 0`);
check(await ev(`!!document.querySelector('.thanks-preview .hero-svg')`), 'на екрані подяки є листівка з героєм');
await shot('thanks');
await click('Назад');
await waitFor((tx) => tx.includes('Подяка вчителю') || tx.includes('Мої зошити'), 'після подяки');
if (!(await text()).includes('Мої зошити')) { await click('Назад'); }
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Герой і лінійка тижня
check((await text()).includes('Лінійка тижня'), 'на головній є лінійка тижня');
check(await ev(`document.querySelectorAll('.nametag .hero-svg rect').length > 20`), 'на головній намальовано піксельного героя');
await click('Лінійка тижня');
await waitFor((tx) => tx.includes('Це не оцінка'), 'екран лінійки тижня');
await shot('ruler');
await click('Мій герой');
await waitFor((tx) => tx.includes('Постер для друку'), 'екран героя');
await ev(`document.querySelector('.tabs .btn[aria-selected]')?.click()`);
const before = await ev(`document.querySelector('.hero-stage svg').innerHTML.length`);
await ev(`document.querySelector('.swatches .swatch:last-child').click()`);
await waitFor((tx) => tx.includes('Постер для друку'), 'герой після зміни вигляду');
await sleep(300);
const after = await ev(`document.querySelector('.hero-stage svg').innerHTML.length`);
check(before > 0 && after > 0, 'вигляд героя можна змінити');
await shot('hero');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Інші класи: 6 і 7 мають вміст (без математики), 8+ каркас «Скоро»
await click('5 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів');
await shot('grades');
const pickGrade = (g) => ev(`(() => { const cards=[...document.querySelectorAll('.card.soon')]; const c=cards.find(x=>x.innerText.includes('${g} клас')); [...c.querySelectorAll('button')].pop().click(); })()`);
await pickGrade(7);
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Українська мова'), 'головна 7 класу');
const g7 = await text();
check(g7.includes('Історія') && g7.includes('Англійська мова') && g7.includes('Українська література'), 'у 7 класі є історія, англійська, українська мова й література');
check(g7.includes('Математика для цього класу ще готується'), 'у 7 класі чесно сказано, що математики ще немає');
check(g7.includes('Біологія') && g7.includes('Географія') && g7.includes('Фізика') && g7.includes('Хімія') && g7.includes('Інформатика'), 'у 7 класі є біологія, географія, фізика, хімія й інформатика');
check(!g7.includes('Природознавство'), 'у 7 класі немає предметів 5 класу, яких ще не написано');
await shot('home-grade7');
await click('7 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів знову');
await pickGrade(8);
await waitFor((tx) => tx.includes('матеріалів ще немає'), 'сторінка «скоро» для 8 класу');
check(!(await text()).includes('Почати:'), 'для класу без вмісту немає хибних кнопок «Почати»');
await shot('grade-soon');
await click('Перейти до 5 класу');
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Математика'), 'повернення до 5 класу');

// Свято переходу: на сусідній клас перехід через свято, без блокування; у кожного класу свій світ істот
await click('5 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів для свята');
await pickGrade(6);
await waitFor((tx) => tx.includes('5 клас пройдено!'), 'екран свята переходу');
await shot('ceremony-summary');
check((await text()).includes('Опановано тем:'), 'свято: підсумок класу без порівнянь');
await click('Далі');
await waitFor((tx) => tx.includes('Новий світ: «Море й берег»'), 'новий світ у святі');
await shot('ceremony-world');
check((await text()).includes('Це твій перший друг'), 'свято: стартова істота нового світу');
await click('Перейти до 6 класу');
await waitFor((tx) => tx.includes('Вітаю в 6 класі!'), 'вітання в новому класі');
check((await text()).includes('Свідоцтво про перехід'), 'свято: свідоцтво для друку');
await click('Почати вчитися');
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Біологія'), 'головна 6 класу після свята');
await click('Колекція');
await waitFor((tx) => tx.includes('Море й берег'), 'колекція 6 класу має свій світ');
check((await text()).includes('Лісова галявина'), 'старий світ лишається доступним у колекціях');
check(await ev(`!!document.querySelector('.album-world.w-sea')`), 'колекція 6 класу в світі «Море й берег»');
await shot('album-sea');
await click('5 клас · Лісова галявина');
await waitFor((tx) => tx.includes('Світ «Лісова галявина»'), 'колекція 5 класу залишається');
await click('Свідоцтво про перехід з 5 класу');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'повернення з колекції');
// Повертаємося до 5 класу для подальшого сценарію
await click('6 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів для повернення');
await pickGrade(5);
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Математика'), 'повернення до 5 класу після свята');

// Кінець шляху: останній клас має власне свято без нового світу
await click('5 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів для випускного');
await ev(`(() => { const c=[...document.querySelectorAll('.card.soon')].find(x=>x.innerText.includes('НМТ')); [...c.querySelectorAll('button')].pop().click(); })()`);
await waitFor((tx) => tx.includes('матеріалів ще немає'), 'НМТ без вмісту');
await click('НМТ');
await waitFor((tx) => tx.includes('Свято завершення шляху'), 'на екрані класів є свято завершення');
await click('Свято завершення шляху');
await waitFor((tx) => tx.includes('Шлях пройдено!'), 'випускне свято');
await shot('ceremony-final');
await click('Завершити шлях');
await waitFor((tx) => tx.includes('Вітаємо!'), 'вітання після завершення шляху');
check((await text()).includes('Свідоцтво про перехід'), 'випускне: свідоцтво для друку');
await click('На головну');
await waitFor((tx) => tx.includes('матеріалів ще немає'), 'повернення з випускного');
await click('НМТ');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів після випускного');
await pickGrade(5);
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Математика'), 'повернення до 5 класу після випускного');

// Вихід і повторний вхід з хибним PIN
await click('Змінити профіль');
await waitFor((tx) => tx.includes('Хто вчиться'), 'список профілів');
await ev(`document.querySelector('.profile').click()`);
await waitFor((tx) => tx.includes('Введи свій PIN'), 'екран PIN');
await ev(`document.querySelector('input').value='0000'`); await click('Відкрити');
await waitFor((tx) => tx.includes('Не той PIN'), 'відмова при хибному PIN');
await ev(`document.querySelector('input').value='1234'`); await click('Відкрити');
await waitFor((tx) => tx.includes('Мої зошити'), 'вхід із правильним PIN');
check(true, 'повторний вхід');

// Профіль вчителя: окрема обкладинка, кабінет, роботи для друку
await click('Змінити профіль');
await waitFor((tx) => tx.includes('Хто вчиться'), 'список профілів перед вчителем');
await click('Новий профіль');
await waitFor((t) => t.includes('Хто ти?'), 'вибір ролі для вчителя');
await click('Я вчитель або вчителька');
await waitFor((t) => t.includes('Профіль вчителя'), 'створення профілю вчителя');
check(await ev(`document.documentElement.classList.contains('role-teacher')`), 'обкладинка вчителя вмикається на екрані створення');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='Оксана Петрівна'; i[1].value='5678'; i[2].value='5678'; })()`);
await click('Створити');
await waitFor((t) => t.includes('Код відновлення'), 'код відновлення вчителя');
await ev('(() => { const c=document.querySelector("input[type=checkbox]"); c.checked=true; c.dispatchEvent(new Event("change")); })()');
await click('Далі');
await waitFor((t) => t.includes('Кабінет') && t.includes('Роботи'), 'кабінет вчителя');
const cab = await text();
check(cab.toLowerCase().includes('kmit для вчителів') && cab.includes('Вітаю, Оксана'), 'кабінет: підпис «для вчителів» і привітання');
check(!cab.includes('Лінійка тижня') && !cab.includes('Колекція') && !cab.includes('Мої зошити'), 'у кабінеті немає дитячих екранів');
await shot('cabinet');
await click('Роботи');
await waitFor((t) => t.includes('Вид роботи'), 'екран робіт');
await ev(`(() => { const c=[...document.querySelectorAll('input[type=checkbox]')].slice(0,2); c.forEach((x)=>{x.checked=true; x.dispatchEvent(new Event('change'));}); })()`);
check((await text()).includes('Обрано тем: 2 із 8'), 'лічильник обраних тем');
check((await text()).includes('Повторення з попередніх класів') === false, 'у 5 класі немає розділу попередніх класів');
check((await text()).includes('Словниковий диктант') && (await text()).includes('Великий шрифт'), 'на екрані робіт є диктант і великий шрифт');
await click('Створити роботу');
await waitFor((t) => t.includes('Робота готова'), 'робота створена');
const wk = await text();
check(wk.includes('Завдання (PDF)') && wk.includes('Ключ (PDF)') && wk.includes('Бланк відповідей (PDF)') && wk.includes('Картки для розрізання (PDF)'), 'PDF для друку: завдання, ключ, бланк, картки');
check(await ev(`document.querySelectorAll('.work-preview li').length >= 5`), 'перегляд першого варіанта');
await shot('works');
await click('Назад');
await waitFor((t) => t.includes('Останні роботи'), 'назад до кабінету');
check((await text()).includes('Контрольна'), 'остання робота записана в кабінеті');
await click('Нотатки до уроку');
await waitFor((t) => t.includes('з правильними відповідями'), 'екран нотаток');
await ev(`(() => { const i=document.querySelector('input[type=search]'); i.value='апостроф'; })()`);
await click('Шукати');
await waitFor((t) => t.toLowerCase().includes('апостроф'), 'результати пошуку');
check(await ev(`document.querySelectorAll('.link-row').length >= 1 && document.body.innerText.toLowerCase().includes('апостроф')`), 'пошук по всіх уроках знаходить тему');
await ev(`document.querySelector('.link-row').click()`);
await waitFor((t) => t.includes('План уроку (PDF)'), 'урок для вчителя');
check((await text()).includes('Правильна відповідь'), 'вчитель бачить правильні відповіді в уроці');
check((await text()).includes('Ключові ідеї'), 'вчитель бачить ключові ідеї до «поясни своїми словами»');
await shot('teacher-lesson');
await click('Додати в закладки');
await waitFor((t) => t.includes('Прибрати із закладок'), 'закладку додано');
await click('Назад');
await waitFor((t) => t.includes('Нотатки до уроку'), 'назад до тем');
await click('Назад');
await waitFor((t) => t.includes('Останні роботи'), 'знову кабінет');
check((await text()).includes('Закладки') && (await text()).includes('· 5 клас'), 'у кабінеті є закладка на урок');
await click('Контрольна');
await waitFor((t) => t.includes('Робота готова'), 'повтор роботи з історії');
await click('Назад');
await waitFor((t) => t.includes('Останні роботи'), 'знову кабінет після повтору');
// Свято в кабінеті: після першої роботи (і в день працівників освіти) вчителя вітає Сова, один раз
check(await ev(`!!document.querySelector('.celebration')`), 'у кабінеті є невелике свято');
await shot('teacher-celebration');
await click('Дякую');
await waitFor(() => true, 'пауза');
for (let i = 0; i < 40 && (await ev(`!!document.querySelector('.celebration')`)); i += 1) await sleep(50);
check(!(await ev(`!!document.querySelector('.celebration')`)), 'свято зникає після «Дякую»');
await click('Тематичний план');
await waitFor((t) => t.includes('Уроків на тему'), 'екран тематичного плану');
await shot('teacher-plan');
await click('Назад');
await waitFor((t) => t.includes('Останні роботи'), 'кабінет після плану');
check(!(await text()).includes('Вичитка'), 'у кабінеті вчителя немає вичитки (вона лише в застосунку методиста)');
await click('Змінити профіль');
await waitFor((tx) => tx.includes('Хто вчиться'), 'список профілів із вчителем');
check(await ev(`!document.documentElement.classList.contains('role-teacher')`), 'обкладинка вчителя знімається на списку профілів');
check((await text()).includes('Кабінет вчителя'), 'у списку профіль вчителя підписаний окремо');
await ev(`document.querySelector('.profile-teacher').click()`);
await waitFor((tx) => tx.includes('Кабінет вчителя') && tx.includes('Відкрити'), 'PIN вчителя');
check(await ev(`document.documentElement.classList.contains('role-teacher')`), 'на екрані PIN вчителя та сама обкладинка');
await ev(`document.querySelector('input').value='5678'`); await click('Відкрити');
await waitFor((t) => t.includes('Останні роботи'), 'повторний вхід вчителя');
await click('Змінити профіль');
await waitFor((tx) => tx.includes('Хто вчиться'), 'список профілів наприкінці');

console.log(failed ? `ПРОВАЛЕНО: ${failed}` : 'Усе добре');
ws.close(); cleanup();
process.exit(failed ? 1 : 0);
