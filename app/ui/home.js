import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, iconButton, iconOnly, notice, face, icon, guarded } from './ui.js';
import { skillIconName, SUBJECT_ICON } from './art.js';
import { planCard } from './review.js';
import { heroSvg } from './hero.js';
import { register, go } from './router.js';

export function applySettings(s) {
  document.documentElement.style.setProperty('--scale', String((s.font_scale_percent || 100) / 100));
  document.documentElement.classList.toggle('calm', !!s.reduce_motion);
  document.documentElement.dataset.sound = s.sound ? '1' : '0';
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  if (tauri) tauri.invoke('set_secure', { on: !s.allow_screenshots }).catch(() => {});
  document.documentElement.dataset.confidence = s.ask_confidence ? '1' : '0';
  document.documentElement.classList.toggle('reading', !!s.reading_mode);
  try { localStorage.setItem('scale', String(s.font_scale_percent || 100)); localStorage.setItem('reading', s.reading_mode ? '1' : '0'); } catch { /* не критично */ }
}

function deniedText(c) {
  const f = T.denied[c.reason];
  return typeof f === 'function' ? f(c.secs || 0) : f || T.genericError;
}

function ticks(done, total) {
  const n = Math.min(total, 14);
  const on = total ? Math.round((done / total) * n) : 0;
  return h('span', { class: 'ticks', 'aria-hidden': 'true' }, Array.from({ length: n }, (_, i) => h('i', { class: i < on ? 'on' : '' })));
}

function suggestion(s, c) {
  if (!s || !c.ok || c.resume) return null;
  const why = { review: T.suggest.review, continue: T.suggest.cont, new: T.suggest.new }[s.reason] || '';
  return h('div', { class: 'suggest' },
    h('div', { class: 'txt' }, h('div', { class: 'tag' }, T.suggest.title), h('div', { class: 'name' }, `${s.subject_title}: ${s.skill_title}`), h('div', { class: 'muted' }, why)),
    button(icon('play', 22), () => go('session', { start: true, subject: s.subject }), 'primary icon', { 'aria-label': T.startSubject(s.subject_title) }));
}

// Коротка лінійка тижня на головній: прогрес старанності й заклик забрати нагороду.
function rulerCard(d) {
  const w = d.week;
  const pct = Math.min(100, Math.round((w.units / 15) * 100));
  const fill = h('i', { class: 'fill' });
  fill.style.width = `${Math.max(3, pct)}%`;
  return h('button', { type: 'button', class: `ruler-mini ${d.week_claimable ? 'claim' : ''}`.trim(), onclick: () => go('ruler') },
    h('span', { class: 'rm-t' }, d.week_claimable ? T.ruler.claimNow : T.ruler.title),
    h('span', { class: 'mem', role: 'img', 'aria-label': T.ruler.alt(w.units) }, fill),
    h('span', { class: 'rm-n' }, `${w.units}/15`));
}

register('home', async () => {
  const d = await call('home');
  const plan = d.grade_ready ? await call('plan') : null;
  const gl = !d.grade_ready ? (await call('grades')).grades.find((x) => x.grade === d.grade) : null;
  applySettings(d.settings);
  const c = d.can_start;
  mount(
    h('div', { class: 'nametag' }, h('button', { type: 'button', class: 'hero-mini', 'aria-label': T.hero.title, onclick: () => go('hero') }, heroSvg(d.hero.px, 64)),
      h('div', {},
        h('div', { class: 'small' }, T.nametag),
        h('h1', { class: 'who' }, d.nickname),
        h('button', { type: 'button', class: 'count linklike', onclick: () => go('album'), 'aria-label': T.collection(d.stickers.count) }, icon('sticker', 18), T.collection(d.stickers.count)))),
    h('div', { class: 'topbar' }, button(T.grades.chip(d.grade), () => go('grades'), 'quiet grade-chip', { 'aria-label': `${T.grades.title}: ${T.grades.chip(d.grade)}` }),
      h('span', { class: 'spacer' }),
      iconOnly('exit', T.lock, async () => { await call('lock'); await go('profiles'); }, 'quiet'),
      iconOnly('gear', T.settings, () => go('settings'), 'quiet')),
    d.promote ? h('div', { class: 'suggest promote' }, h('div', { class: 'txt' }, h('div', { class: 'tag' }, T.ceremony.offerTitle), h('div', { class: 'name' }, d.promote.final ? T.ceremony.finalOfferText : T.ceremony.offerText(d.promote.from, d.promote.to))), button(T.ceremony.offerBtn, () => go('ceremony'), 'primary')) : null,
    rulerCard(d),
    d.recovery_ok ? null : notice(T.recoveryHealth, 'warn'),
    c.ok && c.resume ? h('div', { class: 'resume' }, button([icon('play', 20), T.resume], () => go('session', { start: true }), 'primary big')) : null,
    !c.ok ? notice(deniedText(c), 'info') : null,
    d.recall_due ? h('div', { class: 'suggest recall' },
      h('div', { class: 'txt' }, h('div', { class: 'tag' }, T.recall.title), h('div', { class: 'name' }, T.recall.due(d.recall_due)), h('div', { class: 'muted' }, T.recall.why)),
      button(icon('play', 22), () => go('recall'), 'primary icon', { 'aria-label': T.recall.title })) : null,
    plan && plan.blocks.length ? planCard(plan, c.ok && !c.resume) : (d.grade_ready ? suggestion(d.suggestion, c) : null),
    d.backup_hint ? h('div', { class: 'suggest' }, h('div', { class: 'txt' }, h('div', { class: 'name' }, T.backup.hint)),
      button(T.backup.do, () => go('adult'), 'primary'),
      button(T.backup.later, async () => { await call('backup_snooze'); await go('home'); }, 'quiet')) : null,
    h('h2', {}, T.notebooks),
    d.grade_ready && d.grade !== 5 ? notice(T.grades.mathSoon, 'info') : null,
    !d.grade_ready ? h('div', { class: 'card soon' }, h('p', {}, T.grades.soonText), gl ? h('ul', {}, gl.planned.map((p) => h('li', {}, p))) : null, h('div', { class: 'row' }, button(T.grades.back, guarded(async () => { await call('settings_set', { grade: 5 }); await go('home'); }), 'primary'), button(T.grades.title, () => go('grades')))) : null,
    h('div', { class: 'covers' }, (d.grade_ready ? d.subjects : []).map((sub) => {
      const done = sub.total > 0 && sub.mastered === sub.total;
      return h('button', { type: 'button', class: `cover s-${sub.id}`, 'aria-label': `${sub.title}. ${T.subjectProgress(sub.mastered, sub.total)}`, onclick: () => go('subject', { id: sub.id }) },
        h('span', { class: 'label' },
          h('span', { class: 'ico' }, icon(SUBJECT_ICON[sub.id] || 'math', 22)),
          h('span', { class: 'name' }, sub.title)),
        h('span', { class: 'meta2' }, ticks(sub.mastered, sub.total),
          done ? h('span', { class: 'done' }, T.subjectAllDone) : h('span', {}, `${sub.mastered}/${sub.total}`)));
    })),
    h('div', { class: 'shortcuts' },
      iconButton('refresh', d.review_due_total ? T.rv.homeBtn(d.review_due_total) : T.rv.title, () => go('review')),
      iconButton('pencil', T.notesBtn, () => go('notes')),
      iconButton('sticker', T.albumShort, () => go('album')),
      iconButton('sheet', T.sheetShort, () => go('sheet'))),
    h('div', { class: 'row' }, iconButton('hand', T.calm.link, () => go('calm'))));
});

// Стартова перевірка: підряд короткі перевірки тем, поки не знайдеться незнайоме.
function placementBox(id, skills) {
  if (!skills.some((s) => s.state === 'available')) return null;
  return h('div', { class: 'suggest' },
    h('div', { class: 'txt' }, h('div', { class: 'name' }, T.testout.placement), h('div', { class: 'muted' }, T.testout.placementDesc)),
    button(icon('check', 22), async () => {
      const p = await call('placement_next', { subject: id });
      if (p.skill) await go('session', { start: 'testout', skill: p.skill });
    }, 'icon', { 'aria-label': T.testout.placement }));
}

register('subject', async ({ id }) => {
  const d = await call('home');
  const sub = d.subjects.find((s) => s.id === id);
  const skills = d.skills.filter((s) => s.subject === id);
  const c = d.can_start;
  const canStart = c.ok && !c.resume && !sub.nothing_left;
  mount(
    h('div', { class: 'toolbar' }, iconOnly('back', T.back, () => go('home'), 'quiet')),
    h('div', { class: `subject-head s-${id}` },
      h('span', { class: 'ico' }, icon(SUBJECT_ICON[id] || 'math', 30)),
      h('div', {}, h('h1', {}, sub.title), h('p', { class: 'muted-on' }, T.subjectProgress(sub.mastered, sub.total)))),
    canStart ? h('div', { class: 'row' }, button([icon('play', 20), T.startSubject(sub.title)], () => go('session', { start: true, subject: id }), 'primary big')) : null,
    c.ok && c.resume ? h('div', { class: 'row' }, button([icon('play', 20), T.resume], () => go('session', { start: true }), 'primary big')) : null,
    !c.ok ? notice(deniedText(c), 'info') : null,
    c.ok && sub.nothing_left ? notice(T.subjectDone, 'good') : null,
    h('div', { class: 'row' }, iconButton('refresh', T.rv.myKnowledge, () => go('profile', { subject: id }))),
    placementBox(id, skills),
    h('h2', {}, T.topics),
    h('ul', { class: 'path' }, skills.map((s) =>
      h('li', {}, h('button', { type: 'button', class: `node ${s.state}`, 'aria-label': `${s.title}. ${T.state[s.state]}${s.review_due ? '. ' + T.reviewDue : ''}`, onclick: () => go('skill', { id: s.id }) },
        h('span', { class: 'dot', 'aria-hidden': 'true' }, s.state === 'mastered' ? icon('check', 18) : s.state === 'locked' ? icon('lock', 16) : icon(skillIconName(s.id), 18)),
        h('span', {}, h('span', { class: 'name' }, s.title), h('br'), h('span', { class: 'state' }, T.state[s.state]), s.review_due ? [' ', h('span', { class: 'pill' }, T.reviewDue)] : null))))));
});

register('grades', async () => {
  const g = await call('grades');
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.grades.title), iconOnly('back', T.back, () => go('home'), 'quiet')),
    h('div', { class: 'stack' }, g.grades.map((x) => h('div', { class: 'card soon' },
      h('h2', {}, T.grades.chip(x.grade), ' ', h('span', { class: `pill ${x.ready ? 'ok' : ''}` }, x.ready ? T.grades.ready : T.grades.soon)),
      x.ready ? null : [h('ul', {}, x.planned.map((p) => h('li', {}, p))), h('p', { class: 'hint' }, T.grades.plannedNote)],
      x.grade === g.current && x.grade === 12 ? h('div', { class: 'row' }, button(T.ceremony.finalGrades, () => go('ceremony'))) : null,
      h('div', { class: 'row' }, button(x.grade === g.current ? T.grades.current : T.grades.choose, guarded(async () => { if (x.ready && x.grade === g.current + 1) { await go('ceremony'); return; } await call('settings_set', { grade: x.grade }); await go('home'); }), x.grade === g.current ? '' : 'primary', x.grade === g.current ? { disabled: true } : {}))))));
});
