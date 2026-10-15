// «Згадай і поясни»: пригадування своїми словами, потім звірка з ключовими ідеями й самооцінка. Не блокує навчання.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, iconOnly } from './ui.js';
import { badgeList } from './session.js';
import { register, go } from './router.js';

register('recall', async () => {
  const q = await call('recall_queue');
  const cards = q.cards;
  let i = 0;
  const back = iconOnly('back', T.back, () => go('home'), 'quiet');
  if (!cards.length) {
    mount(h('div', { class: 'toolbar' }, h('h1', {}, T.recall.title), back),
      notice(T.recall.none, 'good'), h('p', { class: 'muted' }, T.recall.noneHint));
    return;
  }
  let earned = [];
  const paint = () => {
    const c = cards[i];
    if (!c) {
      mount(h('div', { class: 'toolbar' }, h('h1', {}, T.recall.title), back),
        notice(T.recall.finished, 'good'), badgeList(earned),
        h('div', { class: 'row' }, button(T.recall.home, () => go('home'), 'primary')));
      return;
    }
    const ta = h('textarea', { class: 'notes short', maxlength: 800, placeholder: T.recall.placeholder, 'aria-label': c.prompt });
    const msg = h('div', { 'aria-live': 'polite' });
    const stage = h('div', {});
    const show = guarded(async () => {
      const r = await call('recall_show', { skill: c.skill, text: ta.value });
      if (!r.accepted) { msg.replaceChildren(notice(r.message, 'info')); return; }
      const rate = (rating) => guarded(async () => {
        const r2 = await call('recall_rate', { skill: c.skill, rating });
        earned = earned.concat(r2.new_badges || []);
        i += 1;
        paint();
      });
      stage.replaceChildren(
        h('p', {}, T.recall.compare),
        h('ul', { class: 'ideas plain' }, r.key_ideas.map((t) => h('li', {}, t))),
        h('p', {}, h('strong', {}, T.recall.how)),
        h('div', { class: 'row wrap' },
          button(T.recall.yes, rate('yes'), 'primary'),
          button(T.recall.part, rate('part')),
          button(T.recall.no, rate('no')),
          button(T.recall.reread, () => go('lesson', { id: c.skill }), 'quiet')));
    });
    mount(h('div', { class: 'toolbar' }, h('h1', {}, T.recall.title), back),
      h('p', { class: 'hint center' }, T.learn.stepOf(i + 1, cards.length)),
      h('div', { class: 'card' }, h('p', { class: 'muted' }, c.title), h('p', {}, h('strong', {}, c.prompt))),
      h('p', { class: 'hint' }, T.recall.noPeek), ta, msg,
      h('div', { class: 'row' }, button(T.recall.show, show, 'primary')), stage);
    ta.focus();
  };
  paint();
});
