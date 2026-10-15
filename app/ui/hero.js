// Піксельний герой і «Лінійка тижня»: старанність за тиждень і нагорода (один із трьох предметів для героя).
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, iconOnly, icon, downloadBase64 } from './ui.js';
import { register, go } from './router.js';

const NS = 'http://www.w3.org/2000/svg';
const sv = (tag, attrs = {}, kids = []) => {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  kids.forEach((c) => e.append(c));
  return e;
};
const back = (to = 'home', params = {}) => iconOnly('back', T.back, () => go(to, params), 'quiet');

// Герой: список прямокутників [x, y, w, h, колір] сітки 16×20.
export function heroSvg(px, size = 96, label = T.hero.alt) {
  const s = sv('svg', { viewBox: '0 0 16 20', width: size, height: Math.round(size * 1.25), 'shape-rendering': 'crispEdges', role: 'img', 'aria-label': label, class: 'hero-svg' },
    px.map(([x, y, w, hh, c]) => sv('rect', { x, y, width: w, height: hh, fill: c })));
  return s;
}

// Лінійка: 15 поділок, п'ять відміток із нагородами.
function rulerSvg(week) {
  const W = 320, H = 70, pad = 8;
  const step = (W - 2 * pad) / 15;
  const kids = [sv('rect', { x: pad - 2, y: 22, width: W - 2 * pad + 4, height: 26, rx: 5, class: 'ruler-body' })];
  const filled = Math.min(15, week.units);
  if (filled > 0) kids.push(sv('rect', { x: pad - 2, y: 22, width: filled * step + 2, height: 26, rx: 5, class: 'ruler-fill' }));
  for (let i = 0; i <= 15; i++) {
    const major = i % 3 === 0;
    kids.push(sv('line', { x1: pad + i * step, y1: 48, x2: pad + i * step, y2: major ? 36 : 42, class: 'ruler-tick' }));
    if (major && i > 0) {
      const t = sv('text', { x: pad + i * step, y: 62, 'text-anchor': 'middle', class: 'ruler-num' });
      t.textContent = String(i);
      kids.push(t);
    }
  }
  const REW = { 9: 'common', 12: 'rare', 15: 'epic' };
  for (const [at, rar] of Object.entries(REW)) {
    const x = pad + Number(at) * step;
    const got = week.units >= Number(at);
    kids.push(sv('circle', { cx: x, cy: 12, r: 8, class: `ruler-gem ${rar} ${got ? 'got' : ''}` }));
  }
  const s = sv('svg', { viewBox: `0 0 ${W} ${H}`, class: 'ruler', role: 'img', 'aria-label': T.ruler.alt(week.units) }, kids);
  return s;
}

function itemCard(it, previewPx, actions) {
  return h('div', { class: `item-card r-${it.rarity}` },
    h('div', { class: 'pv' }, heroSvg(previewPx, 72, it.name)),
    h('strong', {}, it.name),
    h('span', { class: `rar ${it.rarity}` }, T.hero.rarity[it.rarity]),
    actions);
}

register('ruler', async () => {
  const [st, cat] = await Promise.all([call('week_status'), call('hero_catalog')]);
  const pxOf = (id) => {
    for (const s of cat.slots) {
      const f = s.items.find((x) => x.id === id);
      if (f) return f.px;
    }
    return [];
  };
  const cur = st.current;
  const b = cur.breakdown;
  const picker = (wk) => h('div', { class: 'pick-row' }, wk.offer.map((it) =>
    itemCard(it, pxOf(it.id), button(T.ruler.choose, guarded(async () => {
      const r = await call('week_claim', { week: wk.week, item: it.id });
      await go('reward', { item: r.claimed, px: r.look.px });
    }), 'primary small'))));
  const claimBlocks = [];
  if (cur.can_claim) claimBlocks.push(h('div', { class: 'card reward-pick' }, h('h2', {}, T.ruler.thisWeek), h('p', {}, T.ruler.pickText), picker(cur),
    cur.tier < 3 ? h('p', { class: 'hint' }, T.ruler.moreHint(cur.tier)) : null));
  st.pending.forEach((wk) => claimBlocks.push(h('div', { class: 'card reward-pick' }, h('h2', {}, T.ruler.pastWeek), h('p', {}, T.ruler.pickText), picker(wk))));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.ruler.title), back()),
    h('p', { class: 'muted' }, T.ruler.intro),
    h('div', { class: 'card ruler-card' }, rulerSvg(cur),
      h('p', { class: 'center' }, T.ruler.units(cur.units)),
      h('div', { class: 'stickers-sum' },
        h('span', { class: 'pill' }, T.ruler.days(b.active_days)),
        h('span', { class: 'pill' }, T.ruler.lessons(b.lessons)),
        h('span', { class: 'pill' }, T.ruler.reviews(b.reviews)),
        b.gaps ? h('span', { class: 'pill l-strong' }, T.ruler.gaps(b.gaps)) : null)),
    cur.claimed ? notice(T.ruler.claimed(cur.claimed.name), 'good') : null,
    !cur.can_claim && !cur.claimed && cur.units < 9 ? notice(T.ruler.need(9 - cur.units), 'info') : null,
    ...claimBlocks,
    h('details', { class: 'card' }, h('summary', {}, T.ruler.howTitle), h('ul', { class: 'prose' }, T.ruler.how.map((x) => h('li', {}, x)))),
    h('div', { class: 'row' }, button([icon('sticker', 20), T.hero.title], () => go('hero'), 'primary'), button(T.week.link, () => go('week')),
      h('span', { class: 'hint' }, T.hero.collected(st.collected, st.collection_total))));
});

register('reward', async ({ item, px }) => {
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.ruler.newItem), back()),
    h('div', { class: 'reward-stage pop' }, heroSvg(px, 168), h('p', { class: 'hand-big' }, item.name),
      h('span', { class: `rar ${item.rarity}` }, T.hero.rarity[item.rarity])),
    h('p', { class: 'center' }, T.ruler.newItemText),
    h('div', { class: 'row' }, button(T.hero.title, () => go('hero'), 'primary'), button(T.ruler.title, () => go('ruler'))));
});

register('hero', async ({ slot: startSlot } = {}) => {
  let cat = await call('hero_catalog');
  let slot = startSlot || 'hat';
  const paint = async () => {
    const view = cat.slots.find((s) => s.slot === slot) || cat.slots[0];
    const set = (args) => guarded(async () => { await call('hero_set', args); cat = await call('hero_catalog'); await paint(); });
    const pdf = guarded(async () => { const r = await call('hero_pdf'); await downloadBase64(r.data, r.filename, 'application/pdf'); });
    mount(
      h('div', { class: 'toolbar' }, h('h1', {}, T.hero.title), back()),
      h('div', { class: 'hero-stage' }, heroSvg(cat.look.px, 176)),
      h('p', { class: 'hint center' }, T.hero.collected(cat.collected, cat.collection_total)),
      h('details', { class: 'card' }, h('summary', {}, T.hero.look),
        h('p', {}, T.hero.skin),
        h('div', { class: 'swatches', role: 'group', 'aria-label': T.hero.skin }, cat.skins.map((s) => {
          const b = h('button', { type: 'button', class: 'swatch', 'aria-label': s.name, 'aria-pressed': String(s.i === cat.look.skin), onclick: set({ skin: s.i }) });
          b.style.background = s.color;
          return b;
        })),
        h('p', {}, T.hero.hairColor),
        h('div', { class: 'swatches', role: 'group', 'aria-label': T.hero.hairColor }, cat.hair_colors.map((s) => {
          const b = h('button', { type: 'button', class: 'swatch', 'aria-label': s.name, 'aria-pressed': String(s.i === cat.look.hair_color), onclick: set({ hair_color: s.i }) });
          b.style.background = s.color;
          return b;
        }))),
      h('div', { class: 'tabs', role: 'tablist' }, cat.slots.map((s) => h('button', { type: 'button', role: 'tab', class: 'btn', 'aria-selected': String(s.slot === slot),
        onclick: () => { slot = s.slot; paint(); } }, s.title))),
      h('div', { class: 'item-grid' }, view.items.map((it) => {
        const act = it.equipped
          ? (view.removable ? button(T.hero.takeOff, set({ slot, item: '' }), 'small') : h('span', { class: 'pill l-strong' }, T.hero.wearing))
          : it.owned ? button(T.hero.wear, set({ slot, item: it.id }), 'primary small')
            : h('span', { class: 'hint' }, T.hero.locked);
        const c = itemCard(it, it.px, act);
        if (!it.owned) c.classList.add('locked');
        return c;
      })),
      h('div', { class: 'row' }, button([icon('sheet', 20), T.hero.poster], pdf, 'primary'), button(T.ruler.title, () => go('ruler'))),
      h('p', { class: 'hint' }, T.hero.posterHint));
  };
  await paint();
});
