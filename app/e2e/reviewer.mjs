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
], { stdio: ['ignore', 'ignore', 'pipe'] });
// Вивід Chromium зберігаємо, щоб показати його, якщо браузер не запуститься.
let chromeErr = '';
chrome.stderr.on('data', (d) => { chromeErr = (chromeErr + d).slice(-4000); });
const cleanup = () => { try { chrome.kill('SIGKILL'); } catch {} try { rmSync(profile, { recursive: true, force: true }); } catch {} };
process.on('exit', cleanup);

process.on('uncaughtException', async (e) => { try { console.error('ЕКРАН ПРИ ПОМИЛЦІ:\n' + (await text())); } catch {} console.error(e.message); cleanup(); process.exit(1); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function wsUrl() {
  // До 30 с: на повільних CI-серверах перший запуск Chromium буває довгим.
  for (let i = 0; i < 300; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json`);
      const t = (await r.json()).find((x) => x.type === 'page');
      if (t) return t.webSocketDebuggerUrl;
    } catch {}
    await sleep(100);
  }
  throw new Error('Chromium не запустився' + (chrome.exitCode !== null ? ` (код виходу ${chrome.exitCode})` : '') + '\n' + chromeErr);
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

const clickIn = (sel, label) => ev(`(() => { const b=[...document.querySelector(${JSON.stringify(sel)}).querySelectorAll('button')].find(x=>x.innerText.trim()===${JSON.stringify(label)}); b.click(); })()`);
await send('Page.navigate', { url: BASE });
await waitFor((t) => t.includes('Вичитка навчальних матеріалів'), 'вітання застосунку методиста');
const first = await text();
check(!/учень|учениц|вчител|Хто ти|Імпорт/i.test(first), 'на вході немає вибору ролей, дитячих слів і імпорту');
check(first.includes('Тут немає даних дітей'), 'вітання пояснює, що даних дітей тут немає');
check(await ev(`document.title.includes('Kmit Методист')`), 'назва застосунку в заголовку');
await shot('welcome');
await click('Почати');
await waitFor((t) => t.includes('Профіль методиста'), 'створення профілю');
await click('Створити профіль');
await waitFor((t) => t.includes('Напишіть ім'), 'помилка без імені');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='Оксана Петрівна'; i[1].value='5678'; i[2].value='5679'; })()`);
await click('Створити профіль');
await waitFor((t) => t.includes('PIN у двох полях різний'), 'помилка різних PIN');
await ev(`document.querySelectorAll('input')[2].value='5678'`);
await click('Створити профіль');
await waitFor((t) => t.includes('Код відновлення'), 'код відновлення');
const recovery = (await ev(`document.querySelector('.code').innerText`)).trim();
check(recovery.length >= 8, 'код відновлення показано');
check(await ev(`document.querySelector('button.primary').disabled`), '«Далі» вимкнено, доки код не збережено');
await ev('(() => { const c=document.querySelector("input[type=checkbox]"); c.checked=true; c.dispatchEvent(new Event("change")); })()');
await click('Далі');
await waitFor((t) => t.includes('Вичитка') && t.includes('Перевірено'), 'головний екран');
const home = await text();
check(home.includes('0 %') && home.includes('Продовжити:'), 'головний екран: прогрес і «Продовжити»');
check(!/Роботи|Тематичний план|Мої зошити|Кабінет/i.test(home), 'у методиста немає робіт, планів і дитячих екранів');
await shot('home');
check((await ev(`document.querySelectorAll('.chip').length`)) >= 4, 'фільтри за класами');
await click('6 клас');
check((await ev(`[...document.querySelectorAll('.group h2')].map(x=>x.innerText).join()`)) === '6 клас', 'фільтр за класом лишає лише 6 клас');
await click('Усі класи');

// Тема: завдання по одному
await click('Продовжити:');
await waitFor((t) => t.includes('Завдання 1') && t.includes('Усе гаразд'), 'перше завдання теми');
const topicName = await ev(`document.querySelector('h1').innerText`);
await shot('task');
check(await ev(`document.querySelector('.count').innerText.startsWith('1 із')`), 'лічильник завдань');
// «Усе гаразд» переходить до наступного
await click('Усе гаразд');
await waitFor((t) => t.includes('Завдання 2'), 'перехід до другого завдання');
check(await ev(`document.querySelector('.count').innerText.startsWith('2 із')`), 'після «усе гаразд» відкрито друге завдання');
// Виправлення
await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>x.innerText.trim()==='Виправити'); b.click(); })()`);
await ev(`(() => { const ed=document.querySelector('.editor:not([hidden])'); ed.querySelector('textarea').value='Виправлений текст від методиста'; })()`);
await clickIn('.editor:not([hidden])', 'Зберегти виправлення');
await waitFor((t) => t.toLowerCase().includes('ваше виправлення') && t.includes('Виправлений текст від методиста'), 'виправлення збережено');
check((await text()).includes('Решта гаразд'), 'після виправлення пропонується «решта гаразд»');
// Те саме виправлення без змін відхиляється
await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>x.innerText.includes('Змінити виправлення')); b.click(); })()`);
await ev(`(() => { const ed=document.querySelector('.editor:not([hidden])'); ed.querySelector('textarea').value=''; })()`);
await clickIn('.editor:not([hidden])', 'Зберегти виправлення');
await waitFor((t) => t.includes('Змініть текст'), 'порожнє виправлення відхилено');
await clickIn('.editor:not([hidden])', 'Скасувати');
await click('Решта гаразд');
await waitFor((t) => t.includes('Завдання 3'), 'перехід до третього завдання');
// Прибрати завдання з підтвердженням
await click('Прибрати завдання');
check(await ev(`!document.querySelector('.confirm').hidden`), 'прибрати: спершу підтвердження');
await click('Так, прибрати завдання');
await waitFor((t) => t.includes('Завдання 4'), 'після «прибрати» відкрито наступне');
await click('Попереднє');
await waitFor((t) => t.includes('Це завдання буде прибране'), 'позначку «прибрати» видно');
await click('Скасувати позначку');
await waitFor((t) => !t.includes('Це завдання буде прибране'), 'позначку знято');
// Незбережена чернетка не губиться при переході між завданнями
await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>x.innerText.trim()==='Виправити'); b.click(); })()`);
await ev(`(() => { const ta=document.querySelector('.editor:not([hidden]) textarea'); ta.value='Чернетка виправлення'; ta.dispatchEvent(new Event('input',{bubbles:true})); })()`);
await click('Наступне');
await waitFor((t) => t.includes('Завдання 4'), 'наступне завдання');
await click('Попереднє');
await waitFor((t) => t.includes('Завдання 3'), 'повернення до третього');
check(await ev(`document.querySelector('.editor:not([hidden]) textarea')?.value === 'Чернетка виправлення'`), 'чернетка виправлення збереглася після переходу');
await clickIn('.editor:not([hidden])', 'Скасувати');
// «Продовжити» веде до першого завдання без позначки
await click('Назад');
await waitFor((t) => t.includes('Вичитка') && t.includes('Продовжити:'), 'головний екран після теми');
await click('Продовжити:');
await waitFor((t) => t.includes('Завдання 3'), 'продовження з першого непозначеного завдання');
check(await ev(`document.querySelector('.count').innerText.startsWith('3 із')`), '«Продовжити» відкриває перше завдання без позначки');
// Сітка всіх завдань відображає стани
await ev(`document.querySelector('.allgrid summary').click()`);
const dots = await ev(`[...document.querySelectorAll('.dot')].slice(0,4).map(d=>d.getAttribute('aria-label')).join('|')`);
check(dots.includes('Завдання 1: Усе гаразд') && dots.includes('Завдання 2: Є виправлення'), 'сітка завдань показує стани');

// Підсумок: решта разом
await click('Підсумок');
await waitFor((t) => t.includes('Підсумок теми'), 'підсумок теми');
await click('Решту без позначки');
await waitFor((t) => t.includes('Підтвердити:'), 'масова позначка просить підтвердження');
await click('Підтвердити:');
await waitFor((t) => t.includes('Усю тему перевірено'), 'усю тему перевірено');
await shot('summary');

// Наступна тема замінює екран: «Назад» телефону веде до головного, а не лишає порожній екран
await click('Наступна тема');
await waitFor((t) => t.includes('Завдання') && !t.includes('Підсумок теми'), 'наступна тема відкрилась');
await ev(`history.back()`);
await waitFor((t) => t.includes('Вичитка') && t.includes('Перевірено'), 'системне «Назад» повертає на головний');

// Надсилання
await click('Надіслати файл');
await waitFor((t) => t.includes('Що буде у файлі'), 'екран надсилання');
check((await text()).includes('Виправити: 1') && (await text()).includes('Прибрати: 0'), 'у файлі одне виправлення, нічого прибирати');
await click('Зберегти файл');
await waitFor((t) => t.includes('Файл збережено: vychytka-oksana-petrivna-'), 'файл збережено з ім’ям методиста');
await shot('send');

// Блокування, PIN, збереження позначок
await click('Назад');
await waitFor((t) => t.includes('Вичитка'), 'головний екран');
await click('Продовжити:');
await waitFor((t) => t.includes('Завдання'), 'відкрито тему перед блокуванням');
await click('Заблокувати');
await waitFor((t) => t.includes('Введіть PIN'), 'екран PIN');
await ev(`history.back()`);
await sleep(400);
check((await text()).includes('Введіть PIN') && !(await text()).includes('Завдання'), 'після блокування «Назад» не показує дані теми');
await ev(`document.querySelector('input').value='0000'`);
await click('Відкрити');
await waitFor((t) => t.includes('Не той PIN'), 'хибний PIN');
await ev(`document.querySelector('input').value='5678'`);
await click('Відкрити');
await waitFor((t) => t.includes('Вичитка') && t.includes('Перевірено'), 'вхід із правильним PIN');
check((await ev(`document.querySelector('.big-num').innerText`)) !== '0 %', 'позначки збереглися після блокування');

// Відновлення за кодом
await click('Заблокувати');
await waitFor((t) => t.includes('Введіть PIN'), 'екран PIN знову');
await click('Забули PIN?');
await waitFor((t) => t.includes('Відновлення доступу'), 'екран відновлення');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value='ХИБНИЙ-КОД'; i[1].value='1111'; i[2].value='1111'; })()`);
await click('Відновити');
await waitFor((t) => t.includes('Код не підходить'), 'хибний код відновлення');
await ev(`(() => { const i=document.querySelectorAll('input'); i[0].value=${JSON.stringify(recovery)}; i[1].value='1111'; i[2].value='1111'; })()`);
await click('Відновити');
await waitFor((t) => t.includes('Вичитка') && t.includes('Перевірено'), 'вхід після відновлення');

// Налаштування
await click('Налаштування');
await waitFor((t) => t.includes('Розмір тексту'), 'налаштування');
await click('130 %');
check((await ev(`getComputedStyle(document.documentElement).getPropertyValue('--scale')`)).trim() === '1.3', 'розмір тексту змінюється');
await click('Темна');
check(await ev(`document.documentElement.getAttribute('data-theme')`) === 'dark', 'темна тема вмикається');
await click('Авто');
await shot('settings');
await click('Видалити профіль і позначки');
await ev(`document.querySelector('.confirm input').value='щось'`);
await click('Видалити назавжди');
await waitFor((t) => t.includes('Напишіть слово'), 'видалення вимагає слово');
await ev(`document.querySelector('.confirm input').value='видалити'`);
await click('Видалити назавжди');
await waitFor((t) => t.includes('Вичитка навчальних матеріалів'), 'після видалення знову вітання');

console.log(failed ? `ПРОВАЛЕНО: ${failed}` : 'Усе добре');
ws.close(); cleanup();
process.exit(failed ? 1 : 0);
