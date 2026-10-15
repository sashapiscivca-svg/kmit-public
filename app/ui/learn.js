// Навчання, а не лише тести: сторінка теми, урок-пояснення, власний конспект, вільна практика.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, downloadBase64, iconOnly, icon } from './ui.js';
import { register, go } from './router.js';

const back = (to = 'home', params = {}) => iconOnly('back', T.back, () => go(to, params), 'quiet');

register('skill', async ({ id }) => {
  const p = await call('skill_page', { skill: id });
  const act = (name, title, desc, onclick, main = false) => h('button', { type: 'button', class: `action ${main ? 'main' : ''}`.trim(), onclick },
    h('span', { class: 'ico' }, icon(name, 24)), h('span', {}, h('span', { class: 't' }, title), h('br'), h('span', { class: 'd' }, desc)));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, p.title), back()),
    h('div', { class: 'skill-head' }, h('span', { class: `state-chip ${p.state}` }, p.state === 'mastered' ? icon('check', 14) : null, T.state[p.state])),
    p.mastered && !p.note_chars ? notice(T.learn.notesSuggest, 'good') : null,
    h('div', { class: 'stack' },
      p.has_lesson ? act('book', T.learn.lesson, T.learn.actions.lesson, () => go('lesson', { id }), true) : h('p', { class: 'muted' }, T.learn.lessonNone),
      act('pencil', `${T.learn.notes}${p.note_chars ? ` · ${T.learn.chars(p.note_chars)}` : ''}`, T.learn.actions.notes, () => go('note', { id })),
      act('check', T.testout.action, T.testout.actionDesc, () => go('session', { start: 'testout', skill: id })),
      act('play', T.learn.practice, T.learn.actions.practice, () => go('session', { start: 'practice', skill: id }))));
});

register('notes', async () => {
  const r = await call('notes_list');
  const pdf = guarded(async () => {
    const x = await call('notes_pdf');
    await downloadBase64(x.data, x.filename, 'application/pdf');
  });
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.learn.notesAll), back()),
    r.notes.length
      ? h('ul', { class: 'skillmap' }, r.notes.map((x) => h('li', {}, h('button', { type: 'button', class: 'skill', onclick: () => go('note', { id: x.skill }) },
          h('span', {}, x.title), h('span', { class: 'tag' }, T.learn.chars(x.chars))))))
      : h('p', { class: 'muted' }, T.learn.notesEmpty),
    r.notes.length ? h('div', { class: 'row' }, button(T.learn.notesPdf, pdf, 'primary')) : null);
});
