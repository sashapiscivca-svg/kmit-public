// Малі помічники застосунку методиста: побудова елементів без innerHTML (увесь текст іде через textContent).

if (!Element.prototype.replaceChildren) {
  Element.prototype.replaceChildren = function (...nodes) { this.textContent = ''; this.append(...nodes); };
}

// Друкарський апостроф: у даних прямий ', на екрані ’.
export const typo = (s) => String(s).replace(/(\p{L})'(\p{L})/gu, '$1’$2');

export function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === false || v === null || v === undefined) continue;
    if (k === 'class') el.className = v;
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (v === true) el.setAttribute(k, '');
    else el.setAttribute(k, k === 'aria-label' || k === 'title' || k === 'placeholder' ? typo(v) : String(v));
  }
  for (const c of children.flat(Infinity)) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c.nodeType ? c : document.createTextNode(typo(c)));
  }
  return el;
}

const SVG = 'http://www.w3.org/2000/svg';
const PATHS = {
  check: 'M5 12.5l4.5 4.5L19 7',
  pen: 'M4 20l4-1 11-11-3-3L5 16zM14 6l3 3',
  cross: 'M6 6l12 12M18 6L6 18',
  circle: 'M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16z',
  back: 'M15 5l-7 7 7 7',
  next: 'M9 5l7 7-7 7',
  lock: 'M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3',
  gear: 'M4 7h10M18 7h2M4 17h2M10 17h10M14 4v6M6 14v6',
  send: 'M4 12l16-8-6 16-3-7z',
  book: 'M12 6c-2-2-5-3-9-3v14c4 0 7 1 9 3 2-2 5-3 9-3V3c-4 0-7 1-9 3zM12 6v14',
  list: 'M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01',
};
export function icon(name, size = 20) {
  const s = document.createElementNS(SVG, 'svg');
  s.setAttribute('viewBox', '0 0 24 24');
  s.setAttribute('width', String(size));
  s.setAttribute('height', String(size));
  s.setAttribute('aria-hidden', 'true');
  s.setAttribute('focusable', 'false');
  const p = document.createElementNS(SVG, 'path');
  p.setAttribute('d', PATHS[name] || PATHS.circle);
  p.setAttribute('fill', 'none');
  p.setAttribute('stroke', 'currentColor');
  p.setAttribute('stroke-width', '2');
  p.setAttribute('stroke-linecap', 'round');
  p.setAttribute('stroke-linejoin', 'round');
  s.append(p);
  return s;
}

export function mount(...nodes) {
  const main = document.getElementById('main');
  main.replaceChildren(...nodes.filter((n) => n));
  // Фокус на заголовок екрана: читалка оголошує, де ви, а клавіатура починає зі змісту.
  const heading = main.querySelector('h1');
  if (heading) { heading.setAttribute('tabindex', '-1'); heading.focus({ preventScroll: true }); } else main.focus({ preventScroll: true });
  document.title = heading && heading.textContent.trim() ? `${heading.textContent.trim()} · Kmit Методист` : 'Kmit Методист';
  window.scrollTo(0, 0);
}

export function button(label, onclick, cls = '', extra = {}) {
  return h('button', { type: 'button', class: `btn ${cls}`.trim(), onclick, ...extra }, label);
}

export function iconButton(name, label, onclick, cls = '', extra = {}) {
  return button([icon(name, 18), h('span', {}, label)], onclick, cls, extra);
}

export function iconOnly(name, aria, onclick, cls = '') {
  return h('button', { type: 'button', class: `btn icon ${cls}`.trim(), onclick, 'aria-label': aria, title: aria }, icon(name, 22));
}

export function notice(text, kind = 'info') {
  return h('p', { class: `notice ${kind}`, role: kind === 'error' ? 'alert' : 'status' }, text);
}

let fieldN = 0;
export function field(labelText, input, hint) {
  const id = `f${++fieldN}`;
  input.id = id;
  const hintId = `${id}h`;
  if (hint) input.setAttribute('aria-describedby', hintId);
  return h('div', { class: 'field' },
    h('label', { for: id }, labelText),
    input,
    hint ? h('p', { class: 'hint', id: hintId }, hint) : null);
}

// Кнопка, яка не дає натиснути двічі, поки виконується дія.
export function guarded(fn) {
  let busy = false;
  return async (ev) => {
    if (busy) return;
    busy = true;
    const b = ev && ev.currentTarget;
    if (b) b.disabled = true;
    try { await fn(ev); } finally { busy = false; if (b && b.isConnected) b.disabled = false; }
  };
}

// Зберігає файл: на телефоні через системне вікно «Зберегти як» (Tauri), у браузері через завантаження.
// Повертає true, лише якщо файл справді збережено.
export async function saveFile(base64, filename, mime) {
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  if (tauri) return (await tauri.invoke('save_file', { name: filename, mime, data: base64 })) === true;
  const bin = atob(base64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
  const a = h('a', { href: url, download: filename });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
  return true;
}

// Невелике сховище налаштувань вигляду й останньої теми: лише зручність, без нього все працює.
export const pref = {
  get(k, d = null) { try { const v = localStorage.getItem(`rv.${k}`); return v === null ? d : v; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(`rv.${k}`, String(v)); } catch { /* не критично */ } },
  clear() { try { Object.keys(localStorage).filter((k) => k.startsWith('rv.')).forEach((k) => localStorage.removeItem(k)); } catch { /* не критично */ } },
};

// Оголошення для читалки екрана без зміни фокуса.
export function announce(text) {
  const el = document.getElementById('live');
  if (!el) return;
  el.textContent = '';
  setTimeout(() => { el.textContent = text; }, 30);
}
