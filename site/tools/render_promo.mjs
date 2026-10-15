// Рендер ролика: кадр за кадром через Chromium (детермінований `__seek(t)`), далі ffmpeg (site/tools/make_promo.sh).
// Запуск: CHROME=... node site/tools/render_promo.mjs <тека кадрів> [fps=60] [тривалість=15] | stills <тека> t1,t2,...
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const CHROME = process.env.CHROME;
if (!CHROME) { console.error('Вкажи CHROME'); process.exit(2); }
const mode = process.argv[2];
const stills = mode === 'stills';
const out = resolve(stills ? process.argv[3] : process.argv[2]);
const fps = Number(stills ? 60 : process.argv[3] || 60);
const dur = Number(stills ? 15 : process.argv[4] || 15);
const times = stills ? process.argv[4].split(',').map(Number) : null;
mkdirSync(out, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), 'kmit-promo-'));
const PORT = 9444;
const chrome = spawn(CHROME, ['--headless=new', '--no-sandbox', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files',
  `--user-data-dir=${profile}`, `--remote-debugging-port=${PORT}`, '--window-size=1920,1080', 'about:blank'], { stdio: 'ignore' });
const cleanup = () => { try { chrome.kill('SIGKILL'); } catch {} try { rmSync(profile, { recursive: true, force: true }); } catch {} };
process.on('exit', cleanup);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let wsUrl;
for (let i = 0; i < 100 && !wsUrl; i++) { try { wsUrl = (await (await fetch(`http://127.0.0.1:${PORT}/json`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl; } catch {} await sleep(100); }
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let id = 0; const pending = new Map();
ws.addEventListener('message', (m) => { const d = JSON.parse(m.data); if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id); } });
const send = (method, params = {}) => new Promise((res) => { const i = ++id; pending.set(i, res); ws.send(JSON.stringify({ id: i, method, params })); });
await send('Page.enable');
await send('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });
await send('Page.navigate', { url: 'file://' + resolve(new URL('../promo/promo.html', import.meta.url).pathname) });
await sleep(800);
await send('Runtime.evaluate', { expression: 'window.__ready', awaitPromise: true });
const frame = async (t, name) => {
  await send('Runtime.evaluate', { expression: `window.__seek(${t})` });
  const r = await send('Page.captureScreenshot', { format: 'jpeg', quality: 95 });
  writeFileSync(join(out, name), Buffer.from(r.result.data, 'base64'));
};
if (stills) {
  for (const t of times) await frame(t, `still-${String(t).replace('.', '_')}.jpg`);
} else {
  const n = Math.round(fps * dur);
  for (let f = 0; f < n; f++) {
    await frame(f / fps, `f${String(f).padStart(5, '0')}.jpg`);
    if (f % 60 === 0) console.log(`кадр ${f}/${n}`);
  }
}
ws.close(); cleanup();
