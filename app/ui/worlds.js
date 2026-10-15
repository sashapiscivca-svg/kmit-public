// Світи істот за класами: у кожного класу своя родина істот і свій колір колекції.
// Перелік світів і їхні назви живуть у `crates/app-core/src/catalog.rs` (WORLDS); тут лише малюнок.
// Додати світ: рядок у WORLDS (Rust) і запис у WORLD_STYLE та `decorate` (докладно: docs/worlds.md).

export const WORLD_STYLE = {
  // 5 клас: звірята з вушками на лісовій галявині (так виглядали всі істоти до появи світів).
  forest: { tint: null, mix: 0, bg: '#e6f4d8', accent: '#3aa66a', mouth: null, decor: { color: '#3aa66a', cells: [[10, 10, 1], [9, 11, 1]] } },
  // 6 клас: морські створіння з плавцями, щупальцями й бульбашками.
  sea: { tint: '#2a9cc8', mix: 0.4, bg: '#d6edf8', accent: '#1b7fb0', mouth: '#0f4f73', decor: { color: '#ffffff', cells: [[1, 1, 1], [10, 2, 1], [0, 3, 0.5]] } },
  // 7 клас: гірські й небесні птахи з крилами, чубчиком і дзьобом.
  sky: { tint: '#9a86e8', mix: 0.35, bg: '#ebe3fb', accent: '#6a55c9', mouth: '#ffb22e', decor: { color: '#ffffff', cells: [[10, 1, 2], [9, 2, 3], [0, 10, 1]] } },
  // 8 клас: печерні істоти з ріжками, шипами й самоцвітами.
  cave: { tint: '#7d6bb0', mix: 0.4, bg: '#e4def2', accent: '#5b4a96', mouth: '#2d2350', decor: { color: '#7fe3f0', cells: [[1, 1, 1], [10, 2, 1], [10, 10, 1]] } },
  // 9 клас: пустельні верблюдики з горбами, довгими ніжками й сонцем.
  desert: { tint: '#e0a040', mix: 0.4, bg: '#faedd0', accent: '#c7842a', mouth: '#7a4a12', decor: { color: '#ffd23f', cells: [[10, 1, 2], [10, 2, 2]] } },
  // 10 клас: космічні мандрівники з антенами-очима, одним «блюдцем» замість ніг і зорями.
  space: { tint: '#4a5ec8', mix: 0.4, bg: '#dfe4f8', accent: '#3a4aa8', mouth: '#ffd23f', decor: { color: '#ffffff', cells: [[0, 1, 0.5], [10, 0, 0.5], [11, 5, 0.5], [1, 10, 0.5]] } },
  // 11 клас: полярні пухнастики з бурульками, снігом і сяйвом.
  ice: { tint: '#6ec6e8', mix: 0.45, bg: '#def3fa', accent: '#3aa0c8', mouth: '#1b5f80', decor: { color: '#ffffff', cells: [[1, 1, 0.5], [10, 3, 0.5], [9, 10, 0.5], [2, 9, 0.5]] } },
  // 12 клас: вогняні птахи-фенікси на вершині перед НМТ.
  summit: { tint: '#e0623a', mix: 0.4, bg: '#fbe2d6', accent: '#c4472a', mouth: '#ffd23f', decor: { color: '#ffd23f', cells: [[1, 1, 1], [10, 1, 1], [5, 11, 2]] } },
};

export const worldStyle = (id) => WORLD_STYLE[id] || WORLD_STYLE.forest;

// Змішує колір предмета з кольором світу: істота лишається «своєї теми», але належить своєму світу.
export function worldBody(hex, id) {
  const w = worldStyle(id);
  if (!w.tint || !w.mix) return hex;
  const a = parseInt(hex.slice(1), 16);
  const b = parseInt(w.tint.slice(1), 16);
  const ch = (s) => Math.round(((a >> s) & 255) * (1 - w.mix) + ((b >> s) & 255) * w.mix);
  return `#${((1 << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)).toString(16).slice(1)}`;
}

// Змінює сітку 8×8 (дзеркально симетричну) під світ: верх, боки й низ тіла. Ядро (рядки 3–5, стовпці 2–5) лишається.
export function decorate(g, id, rnd) {
  const mirror = (r, c) => { g[r][c] = 1; g[r][7 - c] = 1; };
  const clear = (r) => { for (let c = 0; c < 8; c += 1) g[r][c] = 0; };
  if (id === 'sea') {
    clear(0); clear(1);
    mirror(1, 3); mirror(0, 3); // спинний плавець
    if (rnd() < 0.5) mirror(1, 2);
    mirror(4, 0); mirror(5, 0); // бічні плавці
    if (rnd() < 0.5) mirror(3, 0);
    clear(7);
    const pat = rnd() < 0.5 ? [1, 3] : [1, 2];
    for (const c of pat) mirror(7, c); // щупальця
  } else if (id === 'sky') {
    clear(0); clear(1);
    mirror(1, 3); mirror(0, 3); // чубчик
    if (rnd() < 0.5) mirror(0, 2);
    mirror(3, 0); mirror(4, 0); mirror(3, 1); mirror(4, 1); // крила
    if (rnd() < 0.5) mirror(5, 1);
    clear(7);
    mirror(7, 2); // ніжки
  } else if (id === 'cave') {
    clear(0); clear(1);
    mirror(1, 2); mirror(0, 1); // ріжки
    mirror(2, 1); // шипи на плечах
    if (rnd() < 0.5) mirror(0, 3);
    mirror(5, 1); // кремезне тіло
    clear(7);
    mirror(7, 1); mirror(7, 2); // широкі лапи
  } else if (id === 'desert') {
    clear(0); clear(1);
    mirror(1, 3); mirror(0, 3); mirror(1, 2); // горби
    if (rnd() < 0.5) mirror(0, 2);
    mirror(2, 1); mirror(2, 0); // довгі вуха й шия вбік
    clear(7);
    mirror(7, 1); mirror(7, 3); // тонкі ніжки
  } else if (id === 'space') {
    clear(0); clear(1);
    mirror(0, 1); mirror(1, 2); mirror(0, 2); // антени-очі
    mirror(3, 0); mirror(5, 0); // «руки»
    if (rnd() < 0.5) mirror(4, 0);
    clear(7);
    mirror(7, 3); // блюдце
  } else if (id === 'ice') {
    clear(0); clear(1);
    mirror(1, 2); // маленькі круглі вушка
    for (let r = 2; r <= 6; r += 1) mirror(r, 1); // пухнастий бік
    if (rnd() < 0.5) mirror(2, 0);
    clear(7);
    mirror(7, 1); mirror(7, 3); // бурульки
  } else if (id === 'summit') {
    clear(0); clear(1);
    mirror(0, 3); mirror(1, 3); mirror(1, 2); mirror(0, 1); // вогняний чубчик
    mirror(2, 1); mirror(3, 0); mirror(4, 0); // крила
    clear(7);
    mirror(7, 2); mirror(7, 3); // хвіст-полум'я
  }
  return g;
}
