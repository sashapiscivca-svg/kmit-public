// Власні векторні ілюстрації Kmit: однієї лінії (обводка 2, закруглення), без емодзі (старі Android їх не показують).
// Іконки малюються кольором `currentColor`; наліпки й аватари мають власну палітру з чорнильною обводкою.
import { decorate, worldBody, worldStyle } from './worlds.js';


const NS = 'http://www.w3.org/2000/svg';

function el(tag, attrs = {}, kids = []) {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  for (const c of kids) e.append(c);
  return e;
}

function svg(size, viewBox, kids, label) {
  const s = el('svg', { width: size, height: size, viewBox, fill: 'none', 'aria-hidden': label ? 'false' : 'true', focusable: 'false' }, kids);
  if (label) { s.setAttribute('role', 'img'); s.setAttribute('aria-label', label); }
  return s;
}

const stroke = (d, extra = {}) => el('path', { d, stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round', 'stroke-linejoin': 'round', ...extra });
const txt = (t, x, y, size, weight = 800) => {
  const e = el('text', { x, y, 'text-anchor': 'middle', 'font-size': size, 'font-weight': weight, fill: 'currentColor', 'font-family': 'Manrope, system-ui, sans-serif' });
  e.textContent = t;
  return e;
};

// ---- Іконки предметів і тем (24×24) ----
const ICONS = {
  math: () => [stroke('M6 3v6M3 6h6'), stroke('M16 3l6 6M22 3l-6 6'), stroke('M3 18h6'), el('circle', { cx: 6, cy: 14.5, r: 1.4, fill: 'currentColor' }), el('circle', { cx: 6, cy: 21.5, r: 1.4, fill: 'currentColor' }), stroke('M15 18h6')],
  add: () => [stroke('M12 4v16M4 12h16')],
  sub: () => [stroke('M4 12h16')],
  mul: () => [stroke('M6 6l12 12M18 6L6 18')],
  div: () => [stroke('M4 12h16'), el('circle', { cx: 12, cy: 5.5, r: 1.6, fill: 'currentColor' }), el('circle', { cx: 12, cy: 18.5, r: 1.6, fill: 'currentColor' })],
  order: () => [stroke('M8 4C4 8 4 16 8 20'), stroke('M16 4c4 4 4 12 0 16')],
  decimal: () => [txt('0,5', 12, 16, 11)],
  fraction: () => [stroke('M5 12h14'), el('circle', { cx: 12, cy: 6, r: 2, fill: 'currentColor' }), el('circle', { cx: 12, cy: 18, r: 2, fill: 'currentColor' })],
  ukrainian: () => [txt('Аб', 12, 16, 12)],
  english: () => [txt('Aa', 12, 16, 12)],
  history: () => [stroke('M3 21h18M5 21V11M9.5 21V11M14.5 21V11M19 21V11M2.5 10.5L12 4l9.5 6.5')],
  nature: () => [stroke('M5 19C5 9 11 4 20 4c0 9-5 15-15 15z'), stroke('M5 19l8-8')],
  informatics: () => [stroke('M4 5h16a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z'), stroke('M8 20h8M12 16v4')],
  biology: () => [stroke('M12 21V9'), stroke('M12 13C7 13 5 9 5 5c5 0 7 3 7 8z'), stroke('M12 11c0-4 2-6 7-6 0 4-2 7-7 6z')],
  geography: () => [stroke('M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z'), stroke('M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18')],
  physics: () => [stroke('M12 12m-1.5 0a1.5 1.5 0 1 0 3 0 1.5 1.5 0 1 0-3 0'), stroke('M12 4c5 0 8 8 0 8s-5-8 0-8zM4.5 16c2.5-4.5 10-1 7.500-4M19.5 16c-2.500-4.500-10-1-7.500-4')],
  chemistry: () => [stroke('M9 3h6M10 3v6l-5 9a2 2 0 0 0 1.800 3h10.400a2 2 0 0 0 1.800-3l-5-9V3'), stroke('M7.500 15h9')],
  literature: () => [stroke('M12 6c-2-2-5-3-9-3v14c4 0 7 1 9 3 2-2 5-3 9-3V3c-4 0-7 1-9 3z'), stroke('M12 6v14')],
  lock: () => [stroke('M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3')],
  check: () => [stroke('M5 12.5l4.5 4.5L19 7', { 'stroke-width': 3 })],
  speaker: () => [stroke('M4 9.5h3.5L12 5.5v13l-4.5-4H4z'), stroke('M15.5 9a4 4 0 0 1 0 6M18 6.5a8 8 0 0 1 0 11')],
  book: () => ICONS.literature(),
  pencil: () => [stroke('M4 20l1-4 11-11 3 3L8 19z'), stroke('M14 7l3 3')],
  sheet: () => [stroke('M6 3h9l4 4v14H6z'), stroke('M14 3v5h5M9 12h7M9 16h7')],
  hint: () => [stroke('M9 18h6M10 21h4'), stroke('M12 3a6 6 0 0 0-3.5 10.9c.9.7 1.5 1.6 1.5 2.6h4c0-1 .6-1.9 1.5-2.6A6 6 0 0 0 12 3z')],
  refresh: () => [stroke('M20 11a8 8 0 0 0-14-4M4 4v4h4'), stroke('M4 13a8 8 0 0 0 14 4M20 20v-4h-4')],
  sticker: () => [stroke('M5 4h14a1 1 0 0 1 1 1v9l-6 6H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z'), stroke('M20 14h-5a1 1 0 0 0-1 1v5')],
  gear: () => [el('circle', { cx: 12, cy: 12, r: 3, stroke: 'currentColor', 'stroke-width': 2 }), stroke('M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1')],
  back: () => [stroke('M15 5l-7 7 7 7')],
  exit: () => [stroke('M10 4H5v16h5M15 8l4 4-4 4M19 12H9')],
  hint: () => [stroke('M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.1 1 1.9V16h5v-.2c0-.8.4-1.4 1-1.9A6 6 0 0 0 12 3z')],
  hand: () => [stroke('M12 20l-7-7a4.5 4.5 0 0 1 7-5.5A4.5 4.5 0 0 1 19 13z')],
  play: () => [el('path', { d: 'M8 5v14l11-7z', fill: 'currentColor' })],
  star: () => [el('path', { d: 'M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z', stroke: 'currentColor', 'stroke-width': 2, 'stroke-linejoin': 'round' })],
};

export function icon(name, size = 24) {
  const f = ICONS[name] || ICONS.star;
  return svg(size, '0 0 24 24', f());
}

const SKILL_ICON = { 'N-ADD': 'add', 'N-SUB': 'sub', 'N-MUL': 'mul', 'N-DIV': 'div', 'N-ORD': 'order' };
export function skillIconName(id) {
  const k = Object.keys(SKILL_ICON).find((p) => id.startsWith(p));
  if (k) return SKILL_ICON[k];
  if (id.startsWith('D-')) return 'decimal';
  if (id.startsWith('F-')) return 'fraction';
  if (id.startsWith('UA-')) return 'ukrainian';
  if (id.startsWith('EN-')) return 'english';
  if (id.startsWith('HI-')) return 'history';
  if (id.startsWith('NS-')) return 'nature';
  if (id.startsWith('IT-')) return 'informatics';
  if (id.startsWith('LT-')) return 'literature';
  if (/^BI\d/.test(id)) return 'biology';
  if (/^GE\d/.test(id)) return 'geography';
  if (/^PH\d/.test(id)) return 'physics';
  if (/^CH\d/.test(id)) return 'chemistry';
  if (/^IT\d/.test(id)) return 'informatics';
  return 'math';
}

export const SUBJECT_ICON = { math: 'math', ukrainian: 'ukrainian', english: 'english', history: 'history', nature: 'nature', informatics: 'informatics', literature: 'literature', biology: 'biology', geography: 'geography', physics: 'physics', chemistry: 'chemistry' };

// ---- Наліпки (48×48, кольорові) ----
const INK = '#1b2a6b';
const o = (extra) => ({ stroke: INK, 'stroke-width': 2.2, 'stroke-linejoin': 'round', 'stroke-linecap': 'round', ...extra });
const STICKERS = {
  mastery: () => [
    el('path', { d: 'M17 30l-5 14 8-3 4 5 3-12', fill: '#d93a2f', ...o() }),
    el('path', { d: 'M31 30l5 14-8-3-4 5-3-12', fill: '#b82a20', ...o() }),
    el('circle', { cx: 24, cy: 20, r: 15, fill: '#ffd23f', ...o() }),
    el('circle', { cx: 24, cy: 20, r: 10, fill: '#ffe9a0', ...o({ 'stroke-width': 1.6 }) }),
    el('path', { d: 'M24 12.5l2.4 5 5.4.7-4 3.8 1 5.4L24 24.8l-4.8 2.6 1-5.4-4-3.8 5.4-.7z', fill: '#2b4bd6', ...o({ 'stroke-width': 1.6 }) }),
  ],
  persisted: () => [
    el('path', { d: 'M4 42L18 16l8 12 6-8 12 22z', fill: '#7fb685', ...o() }),
    el('path', { d: 'M18 16l-4 7 4 2 3-2 3 4 2-3z', fill: '#fff', ...o({ 'stroke-width': 1.6 }) }),
    el('path', { d: 'M18 16V5', ...o() }),
    el('path', { d: 'M18 5l10 3-10 4z', fill: '#d93a2f', ...o() }),
  ],
  self_corrected: () => [
    el('path', { d: 'M10 38l2-8L32 10l6 6L18 36z', fill: '#ffd23f', ...o() }),
    el('path', { d: 'M32 10l3-3a2.8 2.8 0 0 1 4 0l2 2a2.8 2.8 0 0 1 0 4l-3 3z', fill: '#f0a0a0', ...o() }),
    el('path', { d: 'M10 38l2-8 6 6z', fill: '#f4d9b0', ...o() }),
    el('path', { d: 'M10 38l-1.2 4.2L13 41', fill: INK, ...o({ 'stroke-width': 1.6 }) }),
    el('path', { d: 'M24 42h16', ...o() }),
  ],
  asked_and_solved: () => [
    el('path', { d: 'M24 5a13 13 0 0 0-7.5 23.6c1.2.9 1.9 2.2 1.9 3.6V34h11.2v-1.8c0-1.4.7-2.7 1.9-3.6A13 13 0 0 0 24 5z', fill: '#ffe173', ...o() }),
    el('path', { d: 'M18.5 38.5h11M20.5 43h7', ...o() }),
    el('path', { d: 'M20 22l4 4 4-4M24 26v8', ...o({ 'stroke-width': 1.8 }) }),
  ],
  lesson: () => [
    el('path', { d: 'M24 12c-4-4-10-5-18-5v28c8 0 14 1 18 5 4-4 10-5 18-5V7c-8 0-14 1-18 5z', fill: '#cfe0ff', ...o() }),
    el('path', { d: 'M24 12v28', ...o() }),
    el('path', { d: 'M11 15c3 .2 6 1 9 3M11 22c3 .2 6 1 9 3M28 18c3-2 6-2.8 9-3M28 25c3-2 6-2.8 9-3', ...o({ 'stroke-width': 1.6 }) }),
  ],
  note: () => [
    el('path', { d: 'M9 8h24v34H9z', fill: '#fff', ...o() }),
    el('path', { d: 'M9 8v34', ...o() }),
    el('path', { d: 'M15 17h14M15 23h14M15 29h9', ...o({ 'stroke-width': 1.6, stroke: '#2b4bd6' }) }),
    el('path', { d: 'M28 38l2-7 11-11 5 5L35 36z', fill: '#ffd23f', ...o({ 'stroke-width': 1.8 }) }),
    el('path', { d: 'M28 38l2-7 5 5z', fill: '#f4d9b0', ...o({ 'stroke-width': 1.6 }) }),
  ],
  family: () => [
    el('path', { d: 'M23 4h2v40h-2z', fill: '#c8742b', ...o({ 'stroke-width': 1.8 }) }),
    el('path', { d: 'M25 9h14l5 5-5 5H25z', fill: '#4f8cff', ...o() }),
    el('path', { d: 'M23 22H9l-5 5 5 5h14z', fill: '#ffd23f', ...o() }),
    el('path', { d: 'M17 44h14', ...o() }),
  ],
  course: () => [
    el('path', { d: 'M24 8l20 9-20 9L4 17z', fill: '#1b2a6b', ...o() }),
    el('path', { d: 'M12 22v9c0 3 5 6 12 6s12-3 12-6v-9l-12 5.5z', fill: '#2b4bd6', ...o() }),
    el('path', { d: 'M41 19v12', ...o() }),
    el('circle', { cx: 41, cy: 33, r: 2.5, fill: '#ffd23f', ...o({ 'stroke-width': 1.6 }) }),
  ],
};


// ---- Наліпки-створіння: кожна тема має власне піксельне створіння, а вид наліпки додає йому річ (корона, пов'язка,
// лупа, лампочка). Створіння виводиться з назви теми, тож однакове завжди, а різні теми різні. ----
const SUBJECT_COLOR = { math: '#2f5bea', ukrainian: '#f2b705', english: '#e0453a', history: '#c8742b', nature: '#2e9b5f', informatics: '#1b9aaa', literature: '#7b4fc9', biology: '#4c9a2a', geography: '#2a7fba', physics: '#5a4fcf', chemistry: '#c2410c' };
const KIND_BG = { mastery: '#fff0b3', persisted: '#d7efc9', self_corrected: '#ead7f4', asked_and_solved: '#ffd9e2', gift: '#ffe0ee' };

function seedOf(str) {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0 || 1;
}

function shade(hex, k) {
  const n = parseInt(hex.slice(1), 16);
  const ch = (v) => Math.max(0, Math.min(255, Math.round(v * k)));
  const r = ch((n >> 16) & 255), g = ch((n >> 8) & 255), b = ch(n & 255);
  return `#${((1 << 24) | (r << 16) | (g << 8) | b).toString(16).slice(1)}`;
}

// Піксельне створіння 8×8 (дзеркально симетричне): тіло, очі, рот, вушка чи антени.
function creatureGrid(skill, world = 'forest') {
  let s = seedOf(skill);
  const rnd = () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return s / 4294967296; };
  const g = Array.from({ length: 8 }, () => Array(8).fill(0));
  for (let r = 1; r < 8; r++) {
    for (let c = 0; c < 4; c++) {
      const centre = c >= 2 ? 0.92 : 0.62;
      const rowBias = r >= 2 && r <= 6 ? 1 : 0.55;
      if (rnd() < centre * rowBias) { g[r][c] = 1; g[r][7 - c] = 1; }
    }
  }
  // Основа: щільне ядро, щоб створіння завжди було впізнаване.
  for (let r = 3; r <= 5; r++) for (const c of [2, 3, 4, 5]) g[r][c] = 1;
  // Вушка або антени.
  const ear = Math.floor(rnd() * 3);
  g[0][ear === 0 ? 1 : ear === 1 ? 2 : 3] = 1; g[0][ear === 0 ? 6 : ear === 1 ? 5 : 4] = 1;
  g[1][ear === 0 ? 1 : 2] = 1; g[1][ear === 0 ? 6 : 5] = 1;
  return decorate(g, world, rnd);
}

function pathOf(cells, size = 1) {
  // Горизонтальні серії одного кольору в один шлях.
  let d = '';
  for (const [x, y, w] of cells) d += `M${x} ${y}h${w}v${size}h-${w}z`;
  return d;
}

export function creature(skill, kind, subject, earned = true, size = 44, world = 'forest') {
  const ws = worldStyle(world);
  const base = worldBody(SUBJECT_COLOR[subject] || '#2f5bea', world);
  const dark = shade(base, 0.62);
  const g = creatureGrid(skill || 'x', world);
  const body = [], rim = [];
  const ox = 2, oy = 3;
  for (let r = 0; r < 8; r++) {
    let c = 0;
    while (c < 8) {
      if (!g[r][c]) { c++; continue; }
      let e = c;
      while (e < 8 && g[r][e]) e++;
      (r === 7 ? rim : body).push([ox + c, oy + r, e - c]);
      c = e;
    }
  }
  const kids = [el('rect', { x: 0, y: 0, width: 12, height: 12, rx: 1.4, fill: earned ? (KIND_BG[kind] || '#eee') : 'var(--sheet2)' })];
  if (!earned) {
    kids.push(el('path', { d: pathOf([...body, ...rim]), fill: 'var(--ink-soft)', opacity: '.6' }));
  } else {
    kids.push(el('path', { d: pathOf(body), fill: base }), el('path', { d: pathOf(rim), fill: dark }));
    // Очі й рот.
    kids.push(el('path', { d: pathOf([[ox + 2, oy + 3, 1], [ox + 5, oy + 3, 1]]), fill: '#ffffff' }));
    kids.push(el('path', { d: pathOf([[ox + 2.5, oy + 3, 0.5], [ox + 5, oy + 3, 0.5]]), fill: '#2b2b2b' }));
    kids.push(el('path', { d: pathOf([[ox + 3, oy + 5, 2]], 1), fill: ws.mouth || dark }));
    if (ws.decor) kids.push(el('path', { d: pathOf(ws.decor.cells), fill: ws.decor.color, opacity: '.85' }));
    // Річ за видом наліпки.
    const gold = '#ffd23f', ink = '#2b2b2b';
    if (kind === 'mastery') {
      kids.push(el('path', { d: pathOf([[4, 1, 1], [6, 1, 1], [8, 1, 1], [4, 2, 5]]), fill: gold }), el('path', { d: pathOf([[6, 2, 1]]), fill: '#d93a2f' }));
    } else if (kind === 'gift') {
      // Стартова істота нового світу: червона стрічка-бантик.
      kids.push(el('path', { d: pathOf([[4, 1, 1], [7, 1, 1], [5, 2, 2], [4, 3, 1], [7, 3, 1]]), fill: '#d93a2f' }), el('path', { d: pathOf([[5, 1, 2]]), fill: '#ffd23f' }));
    } else if (kind === 'persisted') {
      kids.push(el('path', { d: pathOf([[3, 3, 6]]), fill: '#d93a2f' }), el('path', { d: pathOf([[9, 4, 2], [10, 3, 1]]), fill: '#d93a2f' }));
    } else if (kind === 'self_corrected') {
      kids.push(el('path', { d: pathOf([[8, 8, 3], [8, 9, 1], [10, 9, 1], [8, 10, 3]]), fill: ink }), el('path', { d: pathOf([[9, 9, 1]]), fill: '#cfe9ff' }), el('path', { d: pathOf([[10, 11, 1]]), fill: ink }));
    } else {
      kids.push(el('path', { d: pathOf([[5, 0, 2], [5, 1, 2]]), fill: '#fff3a8' }), el('path', { d: pathOf([[4, 0, 1], [7, 0, 1]]), fill: gold }));
    }
  }
  const s = el('svg', { viewBox: '0 0 12 12', width: size, height: size, 'shape-rendering': 'crispEdges', 'aria-hidden': 'true', focusable: 'false' }, kids);
  s.setAttribute('class', `creature ${earned ? 'earned' : 'locked'}`);
  return s;
}

export function sticker(kind, size = 56) {
  const f = STICKERS[kind] || STICKERS.mastery;
  return svg(size, '0 0 48 48', f());
}

// ---- Аватари (64×64) ----
const eye = (x, y) => [el('circle', { cx: x, cy: y, r: 3.2, fill: INK }), el('circle', { cx: x + 1, cy: y - 1, r: 1, fill: '#fff' })];
const AVATARS = [
  // 0 лис
  { bg: '#ffd9a8', art: () => [el('path', { d: 'M10 14l14 8-2 14zM54 14L40 22l2 14z', fill: '#e8742a', ...o() }), el('path', { d: 'M12 34c0-12 9-18 20-18s20 6 20 18c0 10-9 18-20 18S12 44 12 34z', fill: '#f58b3a', ...o() }), el('path', { d: 'M12 36c6 2 12 8 20 8s14-6 20-8c-1 8-9 16-20 16S13 44 12 36z', fill: '#fff', ...o({ 'stroke-width': 0 }) }), ...eye(24, 33), ...eye(40, 33), el('path', { d: 'M29 42h6l-3 4z', fill: INK, ...o({ 'stroke-width': 1.4 }) })] },
  // 1 ведмідь
  { bg: '#e8d5bd', art: () => [el('circle', { cx: 16, cy: 17, r: 7, fill: '#9a6a43', ...o() }), el('circle', { cx: 48, cy: 17, r: 7, fill: '#9a6a43', ...o() }), el('circle', { cx: 32, cy: 35, r: 20, fill: '#b98558', ...o() }), el('ellipse', { cx: 32, cy: 41, rx: 9, ry: 7, fill: '#ecd2b0', ...o({ 'stroke-width': 1.6 }) }), ...eye(24, 31), ...eye(40, 31), el('circle', { cx: 32, cy: 39, r: 2.6, fill: INK })] },
  // 2 панда
  { bg: '#dfe7ee', art: () => [el('circle', { cx: 16, cy: 17, r: 7, fill: INK }), el('circle', { cx: 48, cy: 17, r: 7, fill: INK }), el('circle', { cx: 32, cy: 35, r: 20, fill: '#fff', ...o() }), el('ellipse', { cx: 23, cy: 31, rx: 5.5, ry: 7, fill: INK, transform: 'rotate(18 23 31)' }), el('ellipse', { cx: 41, cy: 31, rx: 5.5, ry: 7, fill: INK, transform: 'rotate(-18 41 31)' }), el('circle', { cx: 23.5, cy: 30, r: 1.6, fill: '#fff' }), el('circle', { cx: 40.5, cy: 30, r: 1.6, fill: '#fff' }), el('ellipse', { cx: 32, cy: 41, rx: 3.4, ry: 2.4, fill: INK })] },
  // 3 сова
  { bg: '#e6dcc6', art: () => [el('path', { d: 'M12 14l8 6M52 14l-8 6', ...o({ 'stroke-width': 3 }) }), el('path', { d: 'M12 36c0-14 8-22 20-22s20 8 20 22c0 10-8 18-20 18S12 46 12 36z', fill: '#a8793f', ...o() }), el('circle', { cx: 24, cy: 31, r: 8.5, fill: '#fff', ...o() }), el('circle', { cx: 40, cy: 31, r: 8.5, fill: '#fff', ...o() }), el('circle', { cx: 24, cy: 31, r: 3.6, fill: INK }), el('circle', { cx: 40, cy: 31, r: 3.6, fill: INK }), el('path', { d: 'M29 40h6l-3 6z', fill: '#ffb22e', ...o({ 'stroke-width': 1.6 }) })] },
  // 4 жаба
  { bg: '#d7efc9', art: () => [el('circle', { cx: 20, cy: 20, r: 9, fill: '#6cc04a', ...o() }), el('circle', { cx: 44, cy: 20, r: 9, fill: '#6cc04a', ...o() }), el('ellipse', { cx: 32, cy: 38, rx: 22, ry: 17, fill: '#6cc04a', ...o() }), ...eye(20, 20), ...eye(44, 20), el('path', { d: 'M20 42c4 5 20 5 24 0', ...o({ 'stroke-width': 2.4 }) }), el('circle', { cx: 28, cy: 35, r: 1.2, fill: INK }), el('circle', { cx: 36, cy: 35, r: 1.2, fill: INK })] },
  // 5 кіт
  { bg: '#ead7f4', art: () => [el('path', { d: 'M12 12l4 18 12-8zM52 12l-4 18-12-8z', fill: '#8c8c99', ...o() }), el('ellipse', { cx: 32, cy: 38, rx: 21, ry: 18, fill: '#a3a3b1', ...o() }), ...eye(23, 35), ...eye(41, 35), el('path', { d: 'M29 42h6l-3 3z', fill: '#f08fa0', ...o({ 'stroke-width': 1.4 }) }), el('path', { d: 'M10 40h9M10 45l9-2M54 40h-9M54 45l-9-2', ...o({ 'stroke-width': 1.4 }) })] },
  // 6 зайченя
  { bg: '#ffd9e2', art: () => [el('path', { d: 'M20 30C14 14 17 3 22 3s7 12 6 27z', fill: '#f0e6e6', ...o() }), el('path', { d: 'M44 30C50 14 47 3 42 3s-7 12-6 27z', fill: '#f0e6e6', ...o() }), el('ellipse', { cx: 32, cy: 41, rx: 19, ry: 17, fill: '#f0e6e6', ...o() }), ...eye(25, 38), ...eye(39, 38), el('path', { d: 'M29 44h6l-3 3z', fill: '#f08fa0', ...o({ 'stroke-width': 1.4 }) })] },
  // 7 черепаха
  { bg: '#d3f0e1', art: () => [el('ellipse', { cx: 32, cy: 38, rx: 22, ry: 15, fill: '#4a9c6a', ...o() }), el('path', { d: 'M16 38h32M24 25l-3 13M40 25l3 13M32 24v14', ...o({ 'stroke-width': 1.6 }) }), el('circle', { cx: 51, cy: 31, r: 7, fill: '#9bd29c', ...o() }), el('circle', { cx: 53, cy: 29.5, r: 1.6, fill: INK }), el('path', { d: 'M49 34c1.5 1.2 3.4 1.2 5 0', ...o({ 'stroke-width': 1.4 }) }), el('path', { d: 'M16 50l-2 6M26 52l-1 6M40 52l1 6M50 50l2 6', ...o({ 'stroke-width': 3 }) })] },
];

export function avatarArt(i, size = 64) {
  const a = AVATARS[i % AVATARS.length];
  return svg(size, '0 0 64 64', [el('circle', { cx: 32, cy: 32, r: 31, fill: a.bg }), ...a.art()]);
}
export const AVATAR_COUNT = AVATARS.length;

// ---- Малюнки до задач і уроків (дані приходять з ядра; див. tutor-core/src/figure.rs) ----
const cls = (e, c) => { e.setAttribute('class', c); return e; };
const svgText = (t, x, y, anchor = 'middle', size = 13) => {
  const e = cls(el('text', { x, y, 'text-anchor': anchor, 'font-size': size, 'font-weight': 800, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text');
  e.textContent = t;
  return e;
};

function fractionBar(f) {
  const bars = Math.max(1, Math.ceil(f.num / f.den));
  const cellW = Math.min(36, Math.floor(240 / f.den));
  const w = cellW * f.den;
  const kids = [];
  let left = f.num;
  for (let b = 0; b < bars; b++) {
    const filled = Math.min(left, f.den);
    left -= filled;
    for (let i = 0; i < f.den; i++) {
      kids.push(cls(el('rect', { x: 2 + i * cellW, y: 2 + b * 40, width: cellW, height: 34, rx: 3 }), i < filled ? 'fig-fill' : 'fig-empty'));
    }
  }
  const s = svg(w + 4, `0 0 ${w + 4} ${bars * 40 + 2}`, kids, `${f.num} із ${f.den} частин зафарбовано`);
  s.setAttribute('height', String(bars * 40 + 2));
  s.setAttribute('class', 'fig');
  return s;
}

function decimalGrid(f) {
  const grids = Math.max(1, Math.ceil(f.hundredths / 100));
  const kids = [];
  let left = f.hundredths;
  for (let g = 0; g < grids; g++) {
    const filled = Math.min(left, 100);
    left -= filled;
    for (let i = 0; i < 100; i++) {
      const r = Math.floor(i / 10), c = i % 10;
      kids.push(cls(el('rect', { x: 2 + g * 128 + c * 11, y: 2 + r * 11, width: 10, height: 10 }), i < filled ? 'fig-fill' : 'fig-empty'));
    }
  }
  const s = svg(grids * 128, `0 0 ${grids * 128} 114`, kids, `Зафарбовано клітинок: ${f.hundredths} зі 100`);
  s.setAttribute('width', String(grids * 128));
  s.setAttribute('height', '114');
  s.setAttribute('class', 'fig');
  return s;
}

function rectFig(f) {
  const kids = [
    cls(el('rect', { x: 30, y: 24, width: 160, height: 90, rx: 2 }), 'fig-empty fig-line'),
    svgText(f.w, 110, 16),
    svgText(f.h, 214, 72, 'start'),
  ];
  const s = svg(260, '0 0 260 130', kids, `Прямокутник: ${f.w} і ${f.h}`);
  s.setAttribute('width', '260');
  s.setAttribute('height', '130');
  s.setAttribute('class', 'fig');
  return s;
}

function angleFig(f) {
  const cx = 40, cy = 110, len = 130;
  const rad = (f.deg * Math.PI) / 180;
  const x2 = cx + len * Math.cos(rad), y2 = cy - len * Math.sin(rad);
  const r = 34;
  const ax = cx + r * Math.cos(rad), ay = cy - r * Math.sin(rad);
  const kids = [
    cls(el('path', { d: `M${cx} ${cy} L${cx + len} ${cy}`, fill: 'none' }), 'fig-stroke'),
    cls(el('path', { d: `M${cx} ${cy} L${x2.toFixed(1)} ${y2.toFixed(1)}`, fill: 'none' }), 'fig-stroke'),
    cls(el('path', { d: `M${cx + r} ${cy} A${r} ${r} 0 ${f.deg > 180 ? 1 : 0} 0 ${ax.toFixed(1)} ${ay.toFixed(1)}`, fill: 'none' }), 'fig-stroke accent'),
    svgText(f.label, cx + 56 * Math.cos(rad / 2), cy - 50 * Math.sin(rad / 2) + 4, 'middle', 14),
  ];
  const s = svg(190, '0 0 190 125', kids, `Кут ${f.label}`);
  s.setAttribute('width', '190');
  s.setAttribute('height', '125');
  s.setAttribute('class', 'fig');
  return s;
}

function numberLine(f) {
  const w = 280, pad = 16;
  const step = (w - 2 * pad) / ((f.max - f.min) * f.parts);
  const x = (pos) => pad + (pos - f.min * f.parts) * step;
  const kids = [cls(el('path', { d: `M${pad - 6} 44 H${w - pad + 6}`, fill: 'none' }), 'fig-stroke')];
  for (let p = f.min * f.parts; p <= f.max * f.parts; p++) {
    const major = p % f.parts === 0;
    kids.push(cls(el('path', { d: `M${x(p)} ${major ? 36 : 40} V${major ? 52 : 48}`, fill: 'none' }), 'fig-stroke'));
    if (major) kids.push(svgText(String(p / f.parts), x(p), 70));
  }
  for (const m of f.marks) {
    kids.push(cls(el('circle', { cx: x(m.pos), cy: 44, r: 5 }), 'fig-fill'));
    kids.push(svgText(m.label, x(m.pos), 26));
  }
  const s = svg(w, `0 0 ${w} 80`, kids, 'Числова пряма');
  s.setAttribute('width', String(w));
  s.setAttribute('height', '80');
  s.setAttribute('class', 'fig');
  return s;
}


// ---- Схеми для уроків усіх предметів: хронологічна лінія, таблиця, цикл, ланцюжок, карта, стовпчики, круги Ейлера ----
const W = 300;
function wrap(t, n) {
  const words = String(t).split(/\s+/).filter(Boolean);
  const lines = [];
  let cur = '';
  for (const w of words) {
    if (cur && (cur + ' ' + w).length > n) { lines.push(cur); cur = w; } else cur = cur ? `${cur} ${w}` : w;
  }
  if (cur) lines.push(cur);
  return lines.length ? lines : [''];
}
const lineH = 15;
// Багаторядковий підпис; повертає вузли. `weight` 800 для заголовків, 600 для тексту.
function lines(t, x, y, n, anchor = 'middle', size = 12.5, weight = 700) {
  return wrap(t, n).map((ln, k) => {
    const e = cls(el('text', { x, y: y + k * lineH, 'text-anchor': anchor, 'font-size': size, 'font-weight': weight, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text');
    e.textContent = ln;
    return e;
  });
}
const rowsH = (ts, n) => Math.max(...ts.map((t) => wrap(t, n).length)) * lineH;
const box = (x, y, w, h, c = 'fig-box') => cls(el('rect', { x, y, width: w, height: h, rx: 8 }), c);
const arrow = (x1, y1, x2, y2) => {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const hx = (d) => (x2 - 8 * Math.cos(a + d)).toFixed(1);
  const hy = (d) => (y2 - 8 * Math.sin(a + d)).toFixed(1);
  return cls(el('path', { d: `M${x1} ${y1} L${x2} ${y2} M${hx(0.45)} ${hy(0.45)} L${x2} ${y2} L${hx(-0.45)} ${hy(-0.45)}`, fill: 'none' }), 'fig-stroke thin');
};
const done = (kids, h, label) => {
  const s = svg(W, `0 0 ${W} ${h}`, kids, label);
  s.setAttribute('width', String(W));
  s.setAttribute('height', String(h));
  s.setAttribute('class', 'fig diagram');
  return s;
};

function timeline(f) {
  const kids = [];
  let y = 8;
  const mid = 58;
  const spans = [];
  f.rows.forEach(([label, text]) => {
    const h = Math.max(30, rowsH([text], 30) + 12);
    kids.push(...lines(label, mid - 12, y + 18, 9, 'end', 13, 800));
    kids.push(cls(el('circle', { cx: mid, cy: y + 14, r: 6 }), 'fig-fill'));
    kids.push(...lines(text, mid + 16, y + 18, 30, 'start'));
    spans.push(y + 14);
    y += h + 8;
  });
  kids.unshift(cls(el('path', { d: `M${mid} ${spans[0]} V${spans[spans.length - 1]}`, fill: 'none' }), 'fig-stroke'));
  return done(kids, y, `Хронологічна лінія: ${f.rows.map((r) => `${r[0]} ${r[1]}`).join('; ')}`);
}

function compare(f) {
  const cols = f.rows[0].length;
  const cw = (W - 8) / cols;
  const n = Math.max(6, Math.floor(cw / 7.4));
  const kids = [];
  let y = 4;
  f.rows.forEach((r, ri) => {
    const h = rowsH(r, n) + 12;
    r.forEach((c, ci) => {
      kids.push(box(4 + ci * cw, y, cw, h, ri === 0 ? 'fig-head' : (ci === 0 ? 'fig-box strong' : 'fig-box')));
      const ls = lines(c, 4 + ci * cw + cw / 2, y + 18, n, 'middle', 12, ri === 0 || ci === 0 ? 800 : 600);
      if (ri === 0) ls.forEach((e) => e.setAttribute('class', 'fig-text on-fill'));
      kids.push(...ls);
    });
    y += h;
  });
  return done(kids, y + 6, `Таблиця порівняння: ${f.rows.map((r) => r.join(', ')).join('; ')}`);
}

function cycle(f) {
  const n = f.rows.length;
  const R = 92, cx = W / 2, cy = 118;
  const kids = [];
  const pts = f.rows.map((_, i) => {
    const a = -Math.PI / 2 + (2 * Math.PI * i) / n;
    return [cx + R * Math.cos(a), cy + R * Math.sin(a)];
  });
  pts.forEach((p, i) => {
    const q = pts[(i + 1) % n];
    const dx = q[0] - p[0], dy = q[1] - p[1], d = Math.hypot(dx, dy) || 1;
    kids.push(arrow(p[0] + (dx / d) * 36, p[1] + (dy / d) * 22, q[0] - (dx / d) * 38, q[1] - (dy / d) * 24));
  });
  f.rows.forEach(([t], i) => {
    const [x, y] = pts[i];
    kids.push(cls(el('ellipse', { cx: x, cy: y, rx: 47, ry: 25 }), 'fig-box'));
    const ls = wrap(t, 14);
    ls.forEach((ln, k) => {
      const e = cls(el('text', { x, y: y + 4 - ((ls.length - 1) * 7) + k * 13, 'text-anchor': 'middle', 'font-size': ls.some((w) => w.length > 11) ? 10 : 11, 'font-weight': 700, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text');
      e.textContent = ln;
      kids.push(e);
    });
  });
  return done(kids, 242, `Цикл: ${f.rows.map((r) => r[0]).join(' → ')} → знову спочатку`);
}

function flow(f) {
  const kids = [];
  let y = 6;
  f.rows.forEach(([t], i) => {
    const h = rowsH([t], 34) + 14;
    kids.push(box(30, y, W - 60, h));
    kids.push(...lines(t, W / 2, y + 20, 34));
    if (i < f.rows.length - 1) kids.push(arrow(W / 2, y + h, W / 2, y + h + 14));
    y += h + 16;
  });
  return done(kids, y, `Послідовність: ${f.rows.map((r) => r[0]).join(' → ')}`);
}

function mindmap(f) {
  const kids = [];
  const [[center], ...br] = [f.rows[0], ...f.rows.slice(1)];
  const half = Math.ceil(br.length / 2);
  const left = br.slice(0, half), right = br.slice(half);
  const colW = 118;
  const col = (list, x, anchor) => {
    let y = 8;
    const out = [];
    const mids = [];
    list.forEach((r) => {
      const [name, ...items] = r;
      const h = 22 + items.length * 14 + 10 + (wrap(name, 14).length - 1) * 14;
      out.push(box(x, y, colW, h, 'fig-box strong'));
      out.push(...lines(name, x + colW / 2, y + 16, 14, 'middle', 12, 800));
      items.forEach((it, k) => out.push(...lines(`• ${it}`, x + 6, y + 34 + k * 14 + (wrap(name, 14).length - 1) * 14, 17, 'start', 10.5, 600)));
      mids.push([anchor === 'l' ? x + colW : x, y + h / 2]);
      y += h + 10;
    });
    return { out, mids, h: y };
  };
  const L = col(left, 2, 'l'), R = col(right, W - colW - 2, 'r');
  const H = Math.max(L.h, R.h, 70);
  const cx = W / 2, cy = H / 2;
  [...L.mids, ...R.mids].forEach(([x, y]) => kids.push(cls(el('path', { d: `M${x} ${y} L${cx + (x < cx ? -22 : 22)} ${cy}`, fill: 'none' }), 'fig-stroke thin')));
  kids.push(cls(el('ellipse', { cx, cy, rx: 28, ry: 24 }), 'fig-fill'));
  const cl = wrap(center, 8);
  cl.forEach((ln, k) => {
    const e = cls(el('text', { x: cx, y: cy + 4 - ((cl.length - 1) * 6) + k * 12, 'text-anchor': 'middle', 'font-size': 10, 'font-weight': 800, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text on-fill');
    e.textContent = ln;
    kids.push(e);
  });
  kids.push(...L.out, ...R.out);
  return done(kids, H + 4, `Карта понять: ${center}; ${br.map((r) => `${r[0]}: ${r.slice(1).join(', ')}`).join('; ')}`);
}

function bars(f) {
  const max = Math.max(1, ...f.rows.map((r) => Number(r[1])));
  const kids = [];
  let y = 8;
  f.rows.forEach(([label, v]) => {
    const w = Math.round(((W - 210) * Number(v)) / max);
    kids.push(...lines(label, 4, y + 16, 14, 'start', 12, 700));
    kids.push(cls(el('rect', { x: 110, y, width: Math.max(3, w), height: 22, rx: 4 }), 'fig-fill'));
    const t = cls(el('text', { x: 116 + Math.max(3, w), y: y + 16, 'font-size': 12, 'font-weight': 800, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text');
    t.textContent = f.title ? `${v} ${f.title}` : String(v);
    kids.push(t);
    y += 30;
  });
  return done(kids, y + 2, `Стовпчики: ${f.rows.map((r) => `${r[0]} ${r[1]} ${f.title}`).join('; ')}`);
}

function sets(f) {
  const [[a, b], oa, both, ob] = f.rows;
  const kids = [
    cls(el('circle', { cx: 106, cy: 105, r: 84 }), 'fig-set a'),
    cls(el('circle', { cx: 194, cy: 105, r: 84 }), 'fig-set b'),
    ...lines(a, 66, 20, 12, 'middle', 12, 800), ...lines(b, 234, 20, 12, 'middle', 12, 800),
  ];
  const list = (items, x, y0) => items.slice(0, 4).forEach((t, k) => kids.push(...lines(t, x, y0 + k * 16, 10, 'middle', 11, 600)));
  list(oa, 66, 70); list(both, 150, 70); list(ob, 234, 70);
  return done(kids, 200, `Круги: ${a} (${oa.join(', ')}); спільне (${both.join(', ')}); ${b} (${ob.join(', ')})`);
}

// ---- Граф пам'яті: теми як вузли (колір за міцністю пам'яті), зв'язки «передумова → тема», позначка складних прикладів ----
export function memoryGraphSvg(nodes, edges) {
  const byId = new Map(nodes.map((n) => [n.skill, n]));
  const depth = new Map();
  const dep = (id, seen = new Set()) => {
    if (depth.has(id)) return depth.get(id);
    if (seen.has(id)) return 0;
    seen.add(id);
    const ps = edges.filter((e) => e.to === id && byId.has(e.from)).map((e) => dep(e.from, seen) + 1);
    const d = ps.length ? Math.max(...ps) : 0;
    depth.set(id, d);
    return d;
  };
  nodes.forEach((n) => dep(n.skill));
  const cols = Math.max(0, ...depth.values()) + 1;
  const colRows = Array.from({ length: cols }, () => []);
  nodes.forEach((n) => colRows[depth.get(n.skill)].push(n));
  const NW = 150, NH = 72, GX = 26, GY = 14;
  const rows = Math.max(1, ...colRows.map((c) => c.length));
  const W = cols * NW + (cols - 1) * GX + 8;
  const H = rows * NH + (rows - 1) * GY + 8;
  const pos = new Map();
  colRows.forEach((list, c) => list.forEach((n, r) => pos.set(n.skill, { x: 4 + c * (NW + GX), y: 4 + r * (NH + GY) })));
  const kids = [];
  edges.forEach((e) => {
    const a = pos.get(e.from), b = pos.get(e.to);
    if (!a || !b) return;
    kids.push(cls(el('path', { d: `M${a.x + NW} ${a.y + NH / 2} C${a.x + NW + GX / 2} ${a.y + NH / 2} ${b.x - GX / 2} ${b.y + NH / 2} ${b.x} ${b.y + NH / 2}`, fill: 'none' }), 'fig-stroke thin'));
  });
  nodes.forEach((n) => {
    const p = pos.get(n.skill);
    const m = n.memory ?? (n.state === 'mastered' ? 100 : null);
    kids.push(cls(el('rect', { x: p.x, y: p.y, width: NW, height: NH, rx: 8 }), `fig-box ${n.context ? '' : 'strong'} ${n.gaps ? 'has-gap' : ''}`.trim()));
    kids.push(...lines(n.title, p.x + NW / 2, p.y + 15, 18, 'middle', 12, 800).slice(0, 3));
    if (m !== null) {
      const BW = NW - 16 - 34;
      kids.push(cls(el('rect', { x: p.x + 8, y: p.y + NH - 14, width: BW, height: 6, rx: 3 }), 'fig-empty'));
      const cl = m >= 67 ? 'mem-good' : m >= 34 ? 'mem-mid' : 'mem-low';
      kids.push(cls(el('rect', { x: p.x + 8, y: p.y + NH - 14, width: Math.max(4, (BW * m) / 100), height: 6, rx: 3 }), cl));
      const pc = cls(el('text', { x: p.x + NW - 8, y: p.y + NH - 8, 'text-anchor': 'end', 'font-size': 11, 'font-weight': 800, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text');
      pc.textContent = `${Math.round(m)}%`;
      kids.push(pc);
    }
    if (n.gaps) {
      kids.push(cls(el('circle', { cx: p.x + NW - 6, cy: p.y + 6, r: 9 }), 'gap-dot'));
      const t = cls(el('text', { x: p.x + NW - 6, y: p.y + 10, 'text-anchor': 'middle', 'font-size': 11, 'font-weight': 800, 'font-family': 'Manrope, system-ui, sans-serif' }), 'fig-text on-fill');
      t.textContent = String(n.gaps);
      kids.push(t);
    }
  });
  const s = svg(W, `0 0 ${W} ${H}`, kids, `Граф пам’яті: ${nodes.map((n) => n.title).join(', ')}`);
  s.setAttribute('width', String(W));
  s.setAttribute('height', String(H));
  s.setAttribute('class', 'fig diagram memory-graph');
  return s;
}

export function figureSvg(f) {
  switch (f.kind) {
    case 'fraction_bar': return fractionBar(f);
    case 'decimal_grid': return decimalGrid(f);
    case 'rect': return rectFig(f);
    case 'angle': return angleFig(f);
    case 'number_line': return numberLine(f);
    case 'timeline': return timeline(f);
    case 'compare': return compare(f);
    case 'cycle': return cycle(f);
    case 'flow': return flow(f);
    case 'mindmap': return mindmap(f);
    case 'bars': return bars(f);
    case 'sets': return sets(f);
    default: return null;
  }
}

// ---- Читання дробів і чисел для екранних читалок (TalkBack): «3/4» → «три четвертих» ----
const ORD_GEN_PL = { 2: 'других', 3: 'третіх', 4: 'четвертих', 5: 'п’ятих', 6: 'шостих', 7: 'сьомих', 8: 'восьмих', 9: 'дев’ятих', 10: 'десятих', 11: 'одинадцятих', 12: 'дванадцятих' };
const ORD_NOM_PL = { 2: 'другі', 3: 'треті', 4: 'четверті' };
const ORD_FEM = { 2: 'друга', 3: 'третя', 4: 'четверта', 5: 'п’ята', 6: 'шоста', 7: 'сьома', 8: 'восьма', 9: 'дев’ята', 10: 'десята', 11: 'одинадцята', 12: 'дванадцята' };
const NUM_NOM = { 1: 'одна', 2: 'дві', 3: 'три', 4: 'чотири', 5: 'п’ять', 6: 'шість', 7: 'сім', 8: 'вісім', 9: 'дев’ять', 10: 'десять', 11: 'одинадцять', 12: 'дванадцять' };

export function speakable(text) {
  return String(text).replace(/(\d{1,2})\s*\/\s*(\d{1,2})/g, (m, a, b) => {
    const n = Number(a), d = Number(b);
    if (!ORD_FEM[d] || !NUM_NOM[n]) return `${a} на ${b}`;
    if (n === 1) return `одна ${ORD_FEM[d]}`;
    if (n <= 4 && ORD_NOM_PL[d]) return `${NUM_NOM[n]} ${ORD_NOM_PL[d]}`;
    return `${NUM_NOM[n]} ${ORD_GEN_PL[d]}`;
  }).replace(/(\d),(\d)/g, '$1 кома $2').replace(/×/g, ' помножити на ').replace(/÷/g, ' поділити на ').replace(/−/g, ' мінус ');
}
