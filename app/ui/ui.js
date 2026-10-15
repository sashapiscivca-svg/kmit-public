// Малі помічники для побудови інтерфейсу без innerHTML (весь текст іде через textContent).

// Старі WebView (до Chromium 86) не мають replaceChildren.
if (!Element.prototype.replaceChildren) {
  Element.prototype.replaceChildren = function (...nodes) { this.textContent = ''; this.append(...nodes); };
}

// Друкарський апостроф: у даних зберігається прямий ', на екрані завжди ’ (м'який → м’який).
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

import { avatarArt, AVATAR_COUNT, icon, sticker } from './art.js';
export { icon, sticker };
export const AVATARS = Array.from({ length: AVATAR_COUNT }, (_, i) => i);
export const face = (i) => h('span', { class: 'avatar', 'aria-hidden': 'true' }, avatarArt(i));
export const avatar = face;

export function iconLabel(name, label, size = 20) {
  return [icon(name, size), h('span', {}, label)];
}

export function mount(...nodes) {
  const main = document.getElementById('main');
  main.replaceChildren(...nodes.filter((n) => n));
  // Фокус на заголовок екрана: читалка оголошує, де ти, а клавіатура починає зі змісту.
  const heading = main.querySelector('h1');
  if (heading) { heading.setAttribute('tabindex', '-1'); heading.focus({ preventScroll: true }); } else main.focus({ preventScroll: true });
  document.title = heading && heading.textContent.trim() ? `${heading.textContent.trim()} · Kmit` : 'Kmit';
  window.scrollTo(0, 0);
}

export function button(label, onclick, cls = '', extra = {}) {
  return h('button', { type: 'button', class: `btn ${cls}`.trim(), onclick, ...extra }, label);
}

// Кнопка з іконкою й підписом. `iconName`: див. art.js.
export function iconButton(iconName, label, onclick, cls = '', extra = {}) {
  return button(iconLabel(iconName, label), onclick, cls, extra);
}

// Кнопка-іконка без видимого підпису (підпис для скрінрідера).
export function iconOnly(iconName, aria, onclick, cls = '') {
  return h('button', { type: 'button', class: `btn icon ${cls}`.trim(), onclick, 'aria-label': aria }, icon(iconName, 22));
}

export function notice(text, kind = 'info') {
  return h('p', { class: `notice ${kind}`, role: kind === 'error' ? 'alert' : 'status' }, text);
}

export function field(labelText, input, hint) {
  const id = `f${Math.random().toString(36).slice(2, 8)}`;
  input.id = id;
  return h('div', { class: 'field' },
    h('label', { for: id }, labelText),
    input,
    hint ? h('p', { class: 'hint' }, hint) : null);
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
export async function downloadBase64(base64, filename, mime) {
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  if (tauri) {
    return (await tauri.invoke('save_file', { name: filename, mime, data: base64 })) === true;
  }
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

// Озвучення: на телефоні вбудований синтезатор (Tauri), у браузері Web Speech API, якщо він є. Без мережі.
export function canSpeak() {
  return !!((window.__TAURI__ && window.__TAURI__.core) || window.speechSynthesis);
}

export async function speak(text) {
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  try {
    if (tauri) {
      await tauri.invoke('speak', { text });
    } else if (window.speechSynthesis) {
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = 'uk-UA';
      window.speechSynthesis.speak(u);
    }
  } catch { /* озвучення необов'язкове */ }
}

export const speechOn = () => canSpeak() && document.documentElement.dataset.sound === '1';

export function speakButton(getText, label) {
  return h('button', { type: 'button', class: 'btn icon speak', 'aria-label': label, onclick: () => speak(getText()) }, icon('speaker', 22));
}
