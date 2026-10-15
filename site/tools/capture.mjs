// Знімає екрани застосунку для лендінгу, презентації й ролика (site/assets/shots). Запуск: site/tools/capture.sh
// (на основі наскрізного тесту): Chromium (headless) керується через DevTools-протокол без залежностей.
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
  writeFileSync(join(SHOTS, `${name}.png`), Buffer.from(r.result.data, 'base64'));
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
await click('Новий профіль');
await waitFor((t) => t.includes('Хто ти?'), 'вибір ролі');
await shot('role');
await click('Я учень або учениця');
await waitFor((t) => t.includes('Для дорослого'), 'екран для дорослого');
await click('Зрозуміло, продовжити');
await waitFor((t) => t.includes('Ім’я або прізвисько'), 'форма створення');
for (let i = 0; i < 100 && !(await ev(`!!document.querySelector('.hero-stage svg')`)); i++) await sleep(60);
await click('Кучері');
await sleep(300);
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='Оля'; i[1].value='1234'; i[2].value='1234'; })()`);
await click('Створити');
await waitFor((t) => t.includes('Код відновлення'), 'код відновлення');
await ev('(() => { const c=document.querySelector("input[type=checkbox]"); c.checked=true; c.dispatchEvent(new Event("change")); })()');
await click('Далі');
await waitFor((t) => t.includes('Ласкаво просимо'), 'вступ');
await shot('welcome');
for (let k = 0; k < 3; k++) await click('Далі');
await waitFor((t) => t.includes('Який ти клас'), 'вибір класу');
await click('Почати');
await waitFor((t) => t.includes('Мої зошити'), 'головний екран');
await sleep(300);
await shot('home-empty');

// Заняття: нова тема, задача, хибна відповідь, підказка, відгук
await click('Математика');
await click('Почати: Математика');
await waitFor((t) => t.includes('Нова тема'), 'пропозиція пояснення');
await sleep(250);
await shot('intro');
await click('Спробувати одразу');
await waitFor((t) => t.includes('Перевірити'), 'екран задачі');
await sleep(250);
await shot('task');
await ev(`document.querySelector('input.answer').value='7/7'`); await click('Перевірити');
await waitFor((tx) => tx.includes('Ще не те'), 'нейтральний відгук');
await sleep(250);
await shot('wrong');
await click('Підказка');
await waitFor((tx) => tx.includes('Підказка 1') || tx.includes('подумай') || tx.includes('Спершу'), 'підказка');
await sleep(300);
await shot('hint');
let solvedN = 0;
for (let n = 0; n < 60; n++) {
  const screen = await text();
  if (screen.includes('Додому')) break;
  if (screen.includes('Перевірити')) {
    const a = await peek();
    if (a == null) break;
    await ev(`document.querySelector('input.answer').value=${JSON.stringify(a)}`);
    await click('Перевірити');
    await waitFor((tx) => tx !== screen, 'зміна екрана');
    continue;
  }
  if (screen.includes('Далі')) {
    solvedN++;
    if (solvedN === 1) { await sleep(250); await shot('feedback'); }
    await click('Далі');
    await waitFor((tx) => tx !== screen, 'зміна після «Далі»');
    continue;
  }
  await sleep(100);
}
if (!(await text()).includes('Додому')) { await click('Закінчити на сьогодні'); }
await waitFor((tx) => tx.includes('Додому'), 'екран завершення');
await click('Додому');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Два опановані теми через «я це вже вмію», щоб у колекції з'явилися істоти
async function master(subject, topic) {
  await click(subject);
  await click(topic);
  await waitFor((tx) => tx.includes('Я це вже вмію'), 'сторінка теми');
  await click('Я це вже вмію');
  await waitFor((tx) => tx.includes('Перевірка знань'), 'перевірка знань');
  for (let k = 0; k < 3; k++) {
    await waitFor((tx) => tx.includes('Перевірити'), 'задача');
    const a = await peek();
    await ev(`document.querySelector('input.answer').value=${JSON.stringify(a)}`);
    await click('Перевірити');
    await waitFor((tx) => tx.includes('Далі'), 'відгук');
    await click('Далі');
  }
  await waitFor((tx) => tx.includes('Тему зараховано'), 'тему зараховано');
  await click('Додому');
  await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
}
await master('Математика', 'Віднімання багатоцифрових чисел');
await master('Математика', 'Додавання десяткових дробів');
await sleep(300);
await shot('home');
await click('Колекція');
await waitFor((t) => t.includes('Лісова галявина'), 'колекція');
await sleep(300);
await shot('album');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

await click('Повторення');
await waitFor((tx) => tx.includes('Повторення на найближчі дні'), 'повторення');
await sleep(250);
await shot('review');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');
await click('Лінійка тижня');
await waitFor((tx) => tx.includes('Це не оцінка'), 'лінійка тижня');
await sleep(250);
await shot('ruler');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

await click('Налаштування');
await waitFor((tx) => tx.includes('Менше рухів на екрані') || tx.includes('Час і захист'), 'налаштування');
await sleep(300);
await shot('settings');
await click('Назад');
await waitFor((tx) => tx.includes('Мої зошити'), 'додому');

// Урок: перший крок і розібраний приклад
await click('Математика');
await click('Додавання десяткових дробів');
await waitFor((tx) => tx.includes('Я це вже вмію'), 'сторінка теми');
await sleep(200);
await shot('topic');
await click('Урок');
await waitFor((tx) => /Крок 1 з/.test(tx), 'урок');
await sleep(300);
await shot('lesson-hook');
for (let g = 0; g < 40; g++) {
  if (await ev(`!!document.querySelector('.example')`)) { await sleep(250); await shot('lesson-example'); break; }
  if (await clickOnce('Далі')) { await sleep(150); continue; }
  if (await clickOnce('Зрозуміло')) { await sleep(150); continue; }
  await sleep(120);
}

// Свято переходу: 5 → 6
await ev(`window.location.reload()`);
await sleep(600);
await waitFor((t) => t.includes('Введи свій PIN') || t.includes('Хто вчиться') || t.includes('Мої зошити'), 'після перезавантаження');
if ((await text()).includes('Хто вчиться')) { await ev(`document.querySelector('.profile').click()`); await waitFor((t) => t.includes('Введи свій PIN'), 'PIN'); }
if ((await text()).includes('Введи свій PIN')) { await ev(`document.querySelector('input').value='1234'`); await click('Відкрити'); }
await waitFor((tx) => tx.includes('Мої зошити'), 'головна');
await click('5 клас');
await waitFor((tx) => tx.includes('Скоро'), 'перелік класів');
await shot('grades');
await ev(`(() => { const cards=[...document.querySelectorAll('.card.soon')]; const c=cards.find(x=>x.innerText.includes('6 клас')); [...c.querySelectorAll('button')].pop().click(); })()`);
await waitFor((tx) => tx.includes('5 клас пройдено!'), 'свято');
await sleep(700);
await shot('ceremony-summary');
await click('Далі');
await waitFor((tx) => tx.includes('Новий світ'), 'новий світ');
await sleep(500);
await shot('ceremony-world');
await click('Перейти до 6 класу');
await waitFor((tx) => tx.includes('Вітаю в 6 класі!'), 'вітання');
await sleep(400);
await shot('ceremony-done');
await click('Почати вчитися');
await waitFor((tx) => tx.includes('Мої зошити') && tx.includes('Біологія'), 'головна 6 класу');
await sleep(300);
await shot('home-grade6');

// Кабінет вчителя
await click('Змінити профіль');
await waitFor((tx) => tx.includes('Хто вчиться'), 'профілі');
await click('Новий профіль');
await waitFor((t) => t.includes('Хто ти?'), 'роль');
await click('Я вчитель або вчителька');
await waitFor((t) => t.includes('Профіль вчителя'), 'профіль вчителя');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='Оксана Петрівна'; i[1].value='5678'; i[2].value='5678'; })()`);
await click('Створити');
await waitFor((t) => t.includes('Код відновлення'), 'код вчителя');
await ev('(() => { const c=document.querySelector("input[type=checkbox]"); c.checked=true; c.dispatchEvent(new Event("change")); })()');
await click('Далі');
await waitFor((t) => t.includes('Кабінет') && t.includes('Роботи'), 'кабінет');
await sleep(500);
await shot('cabinet');
await click('Роботи');
await waitFor((t) => t.includes('Вид роботи'), 'роботи');
await ev(`(() => { const c=[...document.querySelectorAll('input[type=checkbox]')].slice(0,2); c.forEach((x)=>{x.checked=true; x.dispatchEvent(new Event('change'));}); })()`);
await click('Створити роботу');
await waitFor((t) => t.includes('Робота готова'), 'робота готова');
await sleep(300);
await shot('work-ready');
console.log('Готово: знімки в', SHOTS);
ws.close(); cleanup();
process.exit(0);
