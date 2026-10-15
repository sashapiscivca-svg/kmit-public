// Повторення, профіль знань і план на сьогодні. Повторення це окремий режим: короткі задачі лише з тем, які пора
// повторити, чергуючись; профіль показує, наскільки міцно тримається кожна тема.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, iconOnly, icon } from './ui.js';
import { SUBJECT_ICON, memoryGraphSvg } from './art.js';
import { register, go, goBack } from './router.js';

const back = (to = 'home', params = {}) => iconOnly('back', T.back, () => goBack(to, params), 'quiet');

// Смужка міцності пам'яті (0..100): зелена, жовта, червона; значення дублюється текстом.
export function memBar(pct, kind) {
  const cls = kind || (pct >= 67 ? 'good' : pct >= 34 ? 'mid' : 'low');
  const fill = h('i', { class: `fill ${cls}` });
  fill.style.width = `${Math.max(4, pct)}%`;
  return h('span', { class: 'mem', role: 'img', 'aria-label': T.rv.memory(pct) }, fill);
}

const startReview = (subject) => go('session', { start: 'review', subject });

// План на сьогодні: короткий перелік кроків, що підлаштовується під результати останніх днів.
export function planCard(p, canStart) {
  if (!p.blocks.length) return null;
  const rowFor = (b) => {
    const subj = b.subject_title ? ` · ${b.subject_title}` : '';
    const map = {
      review: [T.rv.planReview(b.count), b.hard ? T.rv.planReviewHard(b.hard) : T.rv.planReviewWhy, () => startReview(b.subject), 'refresh'],
      fix: [T.rv.planFix(b.skill_title), b.has_lesson ? T.rv.planFixWhy : T.rv.planFixWhyNoLesson, () => go('skill', { id: b.skill }), 'book'],
      continue: [T.rv.planContinue(b.skill_title), T.rv.planContinueWhy, () => go('session', { start: true, subject: b.subject }), 'play'],
      new: [T.rv.planNew(b.skill_title), T.rv.planNewWhy, () => go('session', { start: true, subject: b.subject }), 'play'],
      recall: [T.rv.planRecall(b.count), T.rv.planRecallWhy, () => go('recall'), 'pencil'],
    };
    const [title, why, act, ico] = map[b.kind];
    const needsStart = b.kind === 'review' || b.kind === 'continue' || b.kind === 'new';
    return h('li', { class: `plan-row k-${b.kind}` },
      h('span', { class: 'ico' }, icon(ico, 22)),
      h('span', { class: 'txt' }, h('span', { class: 't' }, title), h('span', { class: 'd' }, `${why}${subj} · ${T.rv.minutes(b.minutes)}`)),
      button(T.rv.go, act, 'primary small', needsStart && !canStart ? { disabled: true } : {}));
  };
  return h('section', { class: 'plan card', 'aria-label': T.rv.planTitle },
    h('h2', {}, T.rv.planTitle),
    p.mode === 'consolidate' ? h('p', { class: 'hint' }, T.rv.modeConsolidate)
      : p.mode === 'advance' ? h('p', { class: 'hint' }, T.rv.modeAdvance) : null,
    h('ul', { class: 'plan-list' }, p.blocks.map(rowFor)));
}

register('review', async () => {
  const o = await call('review_overview');
  const home = await call('home');
  const canStart = home.can_start.ok && !home.can_start.resume;
  const max = Math.max(1, ...o.week);
  const week = h('div', { class: 'weekbars', role: 'img', 'aria-label': T.rv.weekLabel(o.week.reduce((a, b) => a + b, 0)) },
    o.week.map((n, i) => h('div', { class: `wbar ${n ? 'on' : ''}` },
      h('div', { class: 'col' }, (() => { const f = h('i', { class: 'fill' }); f.style.height = `${Math.round((n / max) * 100)}%`; return f; })()),
      h('span', {}, i === 0 ? T.rv.tomorrow : `+${i + 1}`))));
  const subj = (s) => {
    const due = s.scheduled;
    return h('div', { class: `card rv-subj s-${s.id}` },
      h('h2', {}, h('span', { class: 'ico' }, icon(SUBJECT_ICON[s.id] || 'math', 22)), ' ', s.title),
      due.length || s.item_skills.length ? h('ul', { class: 'rv-list' },
        due.map((d) => h('li', {}, h('span', { class: 'nm' }, d.title), memBar(d.memory ?? 0))),
        s.item_skills.map((d) => h('li', {}, h('span', { class: 'nm' }, d.title), h('span', { class: 'pill' }, T.rv.missed))))
        : h('p', { class: 'muted' }, T.rv.nothing),
      s.recall ? h('p', { class: 'hint' }, T.rv.recallNote(s.recall)) : null,
      h('div', { class: 'row' },
        s.tasks_due ? button([icon('refresh', 20), T.rv.start(s.tasks_due)], () => startReview(s.id), 'primary', canStart ? {} : { disabled: true }) : null,
        button(T.rv.myKnowledge, () => go('profile', { subject: s.id }))));
  };
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.rv.title), back()),
    h('p', { class: 'muted' }, T.rv.intro),
    o.tracked === 0 ? notice(T.rv.empty, 'info') : o.total_due ? h('p', { class: 'strong-line' }, T.rv.dueTotal(o.total_due)) : null,
    h('div', { class: 'card' }, h('h2', {}, T.rv.week), week),
    h('div', { class: 'stack' }, o.subjects.filter((s) => s.tasks_due || s.recall || o.tracked).map(subj)),
    o.subjects.some((s) => s.recall) ? h('div', { class: 'row' }, button([icon('pencil', 20), T.recall.title], () => go('recall'))) : null);
});

const LABEL_ICON = { strong: 'check', fading: 'refresh', repeat: 'refresh', weak: 'hand', learning: 'play', new: 'play', locked: 'lock' };

register('profile', async ({ subject }) => {
  const p = await call('profile', { subject });
  const sums = p.summary;
  const row = (t) => h('li', { class: `pf-row l-${t.label}` },
    h('button', { type: 'button', class: 'pf-btn', onclick: () => go('skill', { id: t.skill }), 'aria-label': `${t.title}. ${T.rv.label[t.label]}` },
      h('span', { class: 'dot', 'aria-hidden': 'true' }, icon(LABEL_ICON[t.label] || 'play', 16)),
      h('span', { class: 'nm' }, h('span', { class: 'name' }, t.title), h('span', { class: 'lbl' }, T.rv.label[t.label])),
      t.memory !== null && t.memory !== undefined ? memBar(t.memory) : (t.known ? memBar(t.known, 'learn') : null)));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.rv.myKnowledge), back('review')),
    h('div', { class: 'row' }, button([icon('refresh', 18), T.rv.memoryMap], () => go('memory', { subject }))),
    h('div', { class: 'stickers-sum' }, ['strong', 'fading', 'repeat', 'weak', 'learning'].filter((k) => sums[k]).map((k) =>
      h('span', { class: `pill l-${k}` }, `${T.rv.label[k]}: ${sums[k]}`))),
    h('p', { class: 'hint' }, T.rv.profileHint),
    h('ul', { class: 'pf-list' }, p.topics.map(row)));
});

const KIND_TEXT = { missed: 'Не вдалося', struggled: 'Вийшло після спроб', asked_adult: 'Показано дорослому' };

function dueText(d) {
  if (d <= 0) return T.rv.dueToday;
  if (d === 1) return T.rv.dueTomorrow;
  return T.rv.dueIn(d);
}

register('memory', async ({ subject }) => {
  const g = await call('memory_graph', { subject });
  const subjTitle = (await call('home')).subjects.find((s) => s.id === subject)?.title || '';
  const dots = (stage, total) => h('span', { class: 'stage-dots', role: 'img', 'aria-label': T.rv.stageOf(stage, total) },
    Array.from({ length: total }, (_, i) => h('i', { class: i < stage ? 'on' : '' })));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.rv.memoryMap), back('profile', { subject })),
    h('p', { class: 'muted' }, subjTitle),
    h('p', { class: 'hint' }, T.rv.memoryHint),
    h('div', { class: 'stickers-sum' },
      h('span', { class: 'pill l-weak' }, T.rv.openGaps(g.gaps.length)),
      h('span', { class: 'pill l-strong' }, T.rv.closedGaps(g.closed_total))),
    g.topics.length ? h('div', { class: 'graph-wrap', tabindex: '0', role: 'region', 'aria-label': T.rv.memoryMap }, memoryGraphSvg(g.topics, g.edges)) : notice(T.rv.noGraph, 'good'),
    g.gaps.length ? h('div', {}, h('h2', {}, T.rv.hardExamples),
      h('ul', { class: 'gap-list' }, g.gaps.map((x) => h('li', { class: 'card gap-card' },
        h('p', { class: 'muted' }, x.title),
        h('p', { class: 'statement small' }, x.statement),
        h('div', { class: 'gap-meta' }, dots(x.stage, x.stages), h('span', { class: 'pill' }, KIND_TEXT[x.kind]), h('span', { class: 'hint' }, dueText(x.due_in))))))) : null,
    g.misconceptions.length ? h('div', {}, h('h2', {}, T.rv.misconceptions),
      h('ul', { class: 'prose' }, g.misconceptions.map((m) => h('li', {}, `${m.title}: ${m.text}`)))) : null,
    g.closed.length ? h('div', {}, h('h2', {}, T.rv.conquered),
      h('ul', { class: 'prose' }, g.closed.slice(0, 8).map((c) => h('li', {}, `${c.title} · ${T.rv.afterDays(c.days)}`)))) : null);
});
