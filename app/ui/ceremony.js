// Свято переходу до наступного класу. Перехід нічим не блокується; свято лише пропонується, коли в класі опановано ≥ 80 % тем.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, downloadBase64 } from './ui.js';
import { register, go } from './router.js';
import { creature } from './art.js';

const C = T.ceremony;

register('ceremony', async () => {
  const s = await call('ceremony_status');
  const album = await call('album', { grade: s.from });
  // Для випускного показуємо істот з усіх світів, для переходу лише пройденого класу.
  let parts = [{ world: s.world.id, cells: album.cells.filter((c) => c.earned) }];
  if (s.final) {
    const others = await Promise.all(album.worlds.filter((w) => w.grade !== s.from).map(async (w) => ({ world: w.id, cells: (await call('album', { grade: w.grade })).cells.filter((c) => c.earned) })));
    parts = [...others, ...parts];
  }
  const earned = parts.flatMap((p) => p.cells.map((c) => ({ ...c, world: p.world })));
  let stage = 'summary';

  const parade = () => {
    const shown = earned.slice(0, 14);
    if (!shown.length) return h('p', { class: 'muted' }, C.paradeNone(s.world.name));
    return h('div', { class: `parade w-${s.world.id}`, role: 'img', 'aria-label': C.parade(s.world.name, earned.length) },
      shown.map((c, i) => { const w = h('span', { class: 'hop' }, creature(c.skill, c.kind, c.subject, true, 52, c.world)); w.style.animationDelay = `${i * 0.12}s`; return w; }));
  };
  const shownParade = () => parade();
  const effort = () => {
    const k = s.stickers.by_kind;
    return (k.persisted || k.selfcorrected) ? h('p', {}, C.effort(k.persisted || 0, k.selfcorrected || 0)) : null;
  };

  const draw = () => {
    if (s.final) {
      const msg = h('div', { 'aria-live': 'polite' });
      if (stage === 'summary') {
        mount(h('section', { class: `ceremony w-${s.world.id}` },
          h('h1', { class: 'hand-big' }, C.finalTitle),
          shownParade(),
          h('p', { class: 'big' }, C.finalSum(s.overall.mastered, s.overall.total, s.overall.stickers)),
          h('p', {}, C.finalText),
          h('div', { class: 'row' }, button(C.finalDo, guarded(async () => { await call('ceremony_do'); stage = 'done'; draw(); }), 'primary big'), button(C.back, () => go('home'), 'quiet'))));
      } else {
        mount(h('section', { class: `ceremony w-${s.world.id}` },
          h('h1', { class: 'hand-big' }, C.finalDone),
          h('p', { class: 'big' }, C.finalDoneText),
          msg,
          h('div', { class: 'row' },
            button(C.certificate, guarded(async () => {
              try { const r = await call('ceremony_pdf', { from: s.from }); if (await downloadBase64(r.data, r.filename, 'application/pdf')) msg.replaceChildren(notice(T.saved, 'good')); }
              catch (e) { msg.replaceChildren(notice(e.message, 'error')); }
            })),
            button(C.back, () => go('home'), 'primary big'))));
      }
      return;
    }
    if (!s.can) {
      mount(h('h1', {}, C.title(s.from)), notice(C.cannot, 'info'), h('div', { class: 'row' }, button(C.back, () => go('home'), 'primary')));
      return;
    }
    if (stage === 'summary') {
      mount(
        h('section', { class: 'ceremony' },
          h('h1', { class: 'hand-big' }, C.title(s.from)),
          parade(),
          h('p', { class: 'big' }, C.sum(s.mastered, s.total)),
          h('p', {}, C.stickers(s.stickers.total)),
          effort(),
          s.left > 0 && s.percent < s.offer_percent ? notice(C.carry(s.left), 'info') : null,
          h('div', { class: 'row' }, button(C.next, () => { stage = 'world'; draw(); }, 'primary big'), button(C.stay(s.from), () => go('home'), 'quiet'))));
    } else if (stage === 'world') {
      mount(
        h('section', { class: `ceremony w-${s.next_world.id}` },
          h('h1', { class: 'hand-big' }, C.newWorld(s.next_world.name)),
          h('div', { class: 'starter', role: 'img', 'aria-label': T.world.starter }, creature(`start:${s.next_world.id}`, 'gift', 'math', true, 120, s.next_world.id)),
          h('p', { class: 'big' }, C.starter),
          h('div', { class: 'row' },
            button(C.go(s.to), guarded(async () => { await call('ceremony_do'); stage = 'done'; draw(); }), 'primary big'),
            button(C.stay(s.from), () => go('home'), 'quiet'))));
    } else {
      const msg = h('div', { 'aria-live': 'polite' });
      mount(
        h('section', { class: `ceremony w-${s.next_world.id}` },
          h('h1', { class: 'hand-big' }, C.welcome(s.to)),
          h('div', { class: 'starter', 'aria-hidden': 'true' }, creature(`start:${s.next_world.id}`, 'gift', 'math', true, 96, s.next_world.id)),
          h('p', { class: 'big' }, C.welcomeText),
          msg,
          h('div', { class: 'row' },
            button(C.certificate, guarded(async () => {
              try { const r = await call('ceremony_pdf', { from: s.from }); if (await downloadBase64(r.data, r.filename, 'application/pdf')) msg.replaceChildren(notice(T.saved, 'good')); }
              catch (e) { msg.replaceChildren(notice(e.message, 'error')); }
            })),
            button(C.start, () => go('home'), 'primary big'))));
    }
  };
  draw();
});
