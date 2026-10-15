// Плеєр уроку (один крок на екрані) і зошит конспекту. Прогрес веде застосунок (ядро), а не цей файл:
// «Далі» відкривається лише після розкритого прикладу, розв'язаного питання й зданого пояснення.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, iconOnly, icon } from './ui.js';
import { badgeList } from './session.js';
import { speakButton, speechOn } from './ui.js';
import { figureSvg } from './art.js';
import { register, go, state } from './router.js';

const bit = (mask, i) => ((mask >> i) & 1) === 1;
const lowerFirst = (s) => (s ? s[0].toLowerCase() + s.slice(1) : s);
// Різні слова довжиною від двох літер: так само рахує ядро, щоб дитина бачила, скільки ще бракує.
const distinctWords = (t) => new Set((t.toLowerCase().match(/[\p{L}’']{2,}/gu) || [])).size;

// Підпис блока без дубля: якщо текст уже починається з підпису («Подумай: …»), другий раз не додаємо.
const labelled = (label, text) => {
  const t = String(text);
  return t.toLowerCase().startsWith(label.toLowerCase()) ? h('span', {}, t) : h('span', {}, h('strong', {}, `${label}: `), t);
};

register('lesson', async ({ id, returnTo }) => {
  const l = await call('lesson', { skill: id });
  const open = { chk: new Set(), ex: new Set(), xp: new Set() };
  const sync = (p) => {
    for (let k = 0; k < 32; k++) {
      if (bit(p.checks, k)) open.chk.add(k);
      if (bit(p.examples, k)) open.ex.add(k);
      if (bit(p.explains, k)) open.xp.add(k);
    }
  };
  sync(l.progress);
  // Кроки: пояснення й малюнок до нього разом; поняття групами не більше трьох; решта блоків окремо.
  const steps = [];
  for (let k = 0; k < l.blocks.length; k++) {
    const b = l.blocks[k];
    const nx = l.blocks[k + 1];
    if (b.type === 'text' && nx && nx.type === 'figure') { steps.push([b, nx]); k += 1; } else if (b.type === 'figure' && nx && nx.type === 'text') { steps.push([b, nx]); k += 1; } else if (b.type === 'term') {
      const g = [b];
      while (g.length < 3 && l.blocks[k + 1] && l.blocks[k + 1].type === 'term') { g.push(l.blocks[k + 1]); k += 1; }
      steps.push(g);
    } else steps.push([b]);
  }
  const memo = new Map(); // стан блоків, щоб «Назад» і «Далі» нічого не стирали
  const mem = (key, init) => { if (!memo.has(key)) memo.set(key, init()); return memo.get(key); };
  const burnt = new Set(); // кроки, розібрані без самостійної відповіді (завтра пройти самому)
  const guesses = [];
  let newBadges = [];
  let complete = !!l.progress.complete;
  let jumpFrom = null;

  const blockDone = (b) => {
    if (b.type === 'example') return open.ex.has(b.index);
    if (b.type === 'quiz' || b.type === 'selfcheck') return open.chk.has(b.index);
    if (b.type === 'explain') return open.xp.has(b.index);
    return true;
  };
  const stepDone = (s) => s.every(blockDone);

  // Позиція: збережена в цьому пристрої, інакше перший непройдений крок, якщо вже щось пройдено.
  const posKey = `kmit.lp.${state.slot ?? 0}.${id}`;
  const loadPos = () => { try { const v = Number(localStorage.getItem(posKey)); return Number.isInteger(v) && v > 0 ? v : null; } catch { return null; } };
  const savePos = (n) => { try { localStorage.setItem(posKey, String(n)); } catch { /* не критично */ } };
  let i = 0;
  let resumed = false;
  if (!complete) {
    const saved = loadPos();
    const anyDone = open.chk.size + open.ex.size + open.xp.size > 0;
    const firstOpen = steps.findIndex((s) => !stepDone(s));
    const start = saved !== null ? Math.min(saved, steps.length - 1) : (anyDone && firstOpen > 0 ? firstOpen : 0);
    if (start > 0) { i = start; resumed = true; }
  }

  const stepKind = (s) => T.lp.kinds[s[0].type] || T.lp.kinds.text;
  const lockText = (s) => {
    const b = s.find((x) => !blockDone(x));
    if (!b) return '';
    if (b.type === 'example') {
      const st = memo.get(`ex${b.index}`);
      return st && st.shown >= b.steps.length ? T.lp.locked.exampleAck : T.lp.locked.example;
    }
    if (b.type === 'quiz' || b.type === 'selfcheck') {
      const st = memo.get(`q${b.index}`);
      return st && st.wrong.size > 0 ? T.lp.locked.again : T.lp.locked[b.type];
    }
    return T.lp.locked[b.type] || '';
  };

  const toSession = guarded(async () => {
    const r = await call('intro_continue');
    await go('session', { start: false, model: r.model });
  });

  let nextSlot;
  let finishSlot;
  let lockNote;
  const goStep = (n) => { i = Math.max(0, Math.min(steps.length - 1, n)); savePos(i); paint(); };
  const hintTarget = () => {
    for (let k = i - 1; k >= 0; k--) if (steps[k].some((b) => b.type === 'example' || b.type === 'text')) return k;
    return null;
  };

  const repaintNext = (announce) => {
    if (!nextSlot) return;
    const s = steps[i];
    const last = i === steps.length - 1;
    const ok = stepDone(s);
    const next = last ? null : h('button', { type: 'button', class: `btn primary ${ok ? '' : 'is-locked'}`.trim(), 'aria-disabled': ok ? null : 'true',
      onclick: () => {
        if (stepDone(steps[i])) { goStep(i + 1); return; }
        const r = lockText(steps[i]);
        if (lockNote) lockNote.textContent = r;
        const first = document.querySelector('.lesson-block button, .lesson-block input, .lesson-block textarea');
        if (first) first.focus();
      } }, T.welcome.next);
    nextSlot.replaceChildren(...[i > 0 ? button(T.learn.prev, () => goStep(i - 1)) : null, next].filter(Boolean));
    if (lockNote) lockNote.textContent = ok ? (announce ? T.lp.canGo : '') : lockText(s);
    if (lockNote && lockNote.parentElement) lockNote.parentElement.hidden = !lockNote.textContent;
    if (last && ok) finishSlot.replaceChildren(finish());
  };

  const finish = () => {
    const terms = l.blocks.filter((b) => b.type === 'term');
    if (returnTo === 'session') return h('div', { class: 'row' }, button(T.intro.toTasks, toSession, 'primary big'));
    const left = [...burnt].sort((a, b) => a - b);
    return h('div', { class: 'stack lp-finish' },
      h('div', { class: 'stamp-line' }, h('span', { class: 'stamp', role: 'status' }, complete ? [icon('check', 28), T.lp.finishStamp] : T.lp.seenStamp)),
      h('div', { class: 'row' },
        button(T.learn.notes, () => go('note', { id, prompts: l.note_prompts }), 'primary'),
        button(T.learn.practice, () => go('session', { start: 'practice', skill: id }))),
      complete ? notice(T.lp.lessonDone, 'good') : notice(left.length ? T.lp.lessonSeenSteps(left.join(', ')) : T.lp.lessonSeen, 'info'),
      badgeList(newBadges),
      guesses.length ? h('div', { class: 'card' }, h('h2', {}, T.lp.guesses), h('ul', { class: 'prose' }, guesses.map((g) => h('li', {}, g)))) : null,
      terms.length ? h('div', { class: 'card' }, h('h2', {}, T.lp.words),
        h('dl', { class: 'terms' }, terms.flatMap((t) => [h('dt', {}, t.term), h('dd', {}, lowerFirst(t.definition))]))) : null,
      h('p', { class: 'hint' }, T.learn.writeNotes));
  };

  // ---- приклад: кроки відкриваються по одному, без «Зрозуміло» далі не пускає ----
  const exampleBlock = (b) => {
    const st = mem(`ex${b.index}`, () => ({ shown: 0 }));
    const box = h('div', { class: 'lesson-block example' });
    const draw = (refocus) => {
      const done = open.ex.has(b.index);
      const visible = done ? b.steps.length : st.shown;
      const all = visible >= b.steps.length;
      const ack = guarded(async () => {
        const r = await call('lesson_example', { skill: id, index: b.index });
        sync(r);
        complete = r.complete;
        draw(true);
        repaintNext(true);
      });
      const more = button(T.lp.nextStep, () => { st.shown += 1; draw(true); repaintNext(); }, 'primary wide');
      const actions = done ? notice(T.lp.exampleDone, 'good')
        : all ? h('div', { class: 'row' }, button(T.lp.understood, ack, 'primary'), button(T.lp.again, () => { st.shown = 0; draw(true); repaintNext(); }, 'quiet'))
          : more;
      box.replaceChildren(...[
        h('p', {}, h('strong', {}, `${T.learn.example}: `), b.statement),
        !done && visible === 0 ? h('p', { class: 'hint' }, T.lp.tryFirst) : null,
        visible > 0 ? h('ol', { class: 'steps', 'aria-label': T.learn.steps, 'aria-live': 'polite' }, b.steps.slice(0, visible).map((x) => h('li', { class: 'pop' }, x))) : null,
        (all || done) ? h('p', { class: 'pop' }, h('strong', {}, `${T.learn.result}: `), b.result) : null,
        done ? null : h('p', { class: 'hint' }, T.lp.stepsShown(visible, b.steps.length)),
        actions].filter(Boolean));
      if (refocus) { const f = box.querySelector('button'); if (f) f.focus({ preventScroll: true }); }
    };
    draw(false);
    box.dataset.shown = '1';
    b.visibleText = () => `${b.statement}. ${b.steps.slice(0, open.ex.has(b.index) ? b.steps.length : st.shown).join('. ')}`;
    return box;
  };

  // ---- питання: вибір чи число; підказка, розбір без відзнаки, стан вибору ----
  const questionBlock = (b) => {
    const st = mem(`q${b.index}`, () => ({ wrong: new Set(), right: null, hint: 0, removed: null, gaveUp: false, answer: '', note: null, text: '' }));
    const box = h('div', { class: 'lesson-block selfcheck' });
    let busy = false;
    const finished = () => open.chk.has(b.index);
    const apply = (r, picked) => {
      if (r.progress) { sync(r.progress); complete = r.progress.complete; }
      if (r.new_badges && r.new_badges.length) newBadges = newBadges.concat(r.new_badges);
      if (r.unreadable) { st.note = { kind: 'info', text: r.explanation }; return; }
      if (r.correct) {
        st.right = picked; open.chk.add(b.index);
        st.note = { kind: 'good', text: `${r.credited ? T.learn.right : T.lp.rightAfter}${r.explanation}` };
      } else if (r.unlocked) {
        st.gaveUp = true; burnt.add(steps.findIndex((s) => s.includes(b)) + 1); open.chk.add(b.index); st.answer = r.answer;
        if (picked !== null) st.wrong.add(picked);
        st.note = { kind: 'info', text: `${r.gave_up ? T.lp.solutionHead : T.lp.showSolution} ${r.explanation}`, answer: r.answer };
      } else {
        if (picked !== null) st.wrong.add(picked);
        st.note = { kind: 'info', text: T.lp.tryAgainLeft(r.attempts_left) };
      }
    };
    const send = async (payload, picked) => {
      if (busy) return;
      busy = true; draw();
      try { apply(await call('lesson_check', { skill: id, index: b.index, ...payload }), picked); } finally { busy = false; }
      draw(); repaintNext(true);
    };
    const askHint = guarded(async () => {
      const r = await call('lesson_hint', { skill: id, index: b.index });
      st.hint = r.level;
      if (r.kind === 'eliminate' && r.remove !== null && r.remove !== undefined) st.removed = r.remove;
      st.hintKind = r.kind;
      draw();
    });
    const draw = () => {
      const done = finished();
      const out = [];
      out.push(h('p', {}, h('strong', {}, `${T.lp.question}: `), b.prompt));
      if (b.type === 'quiz') {
        out.push(h('div', { class: 'choices', role: 'group', 'aria-label': T.task.choose }, b.options.map((o, k) => {
          const cls = ['btn', st.right === k ? 'right' : '', st.wrong.has(k) ? 'wrong' : '', st.removed === k ? 'gone' : ''].filter(Boolean).join(' ');
          const dead = busy || done || st.wrong.has(k) || st.removed === k;
          const mark = st.right === k ? '✓' : st.wrong.has(k) ? '×' : '';
          return h('button', { type: 'button', class: cls, 'aria-disabled': dead ? 'true' : null, 'aria-pressed': st.right === k || st.wrong.has(k) ? 'true' : null,
            onclick: () => { if (!dead) send({ input: String(k + 1) }, k); } },
          h('span', { class: 'letter', 'aria-hidden': 'true' }, mark || 'АБВГДЕ'[k] || String(k + 1)),
          h('span', {}, o),
          st.removed === k ? h('span', { class: 'sr-only' }, T.lp.removed) : null);
        })));
      } else {
        const input = h('input', { type: 'text', class: 'answer', id: `lq${b.index}`, autocomplete: 'off', autocapitalize: 'off', maxlength: 40, inputmode: 'text',
          placeholder: T.lp.answerPlaceholder, readonly: done ? 'true' : null, 'aria-describedby': `lqh${b.index}` });
        input.value = done && st.answer ? st.answer : st.text;
        input.addEventListener('input', () => { st.text = input.value; });
        const submit = () => { if (!st.text.trim()) { st.note = { kind: 'info', text: T.lp.emptyAnswer }; draw(); input.focus(); return; } send({ input: st.text }, null); };
        input.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { ev.preventDefault(); submit(); } });
        out.push(h('div', { class: 'field' }, h('label', { for: `lq${b.index}` }, T.learn.yourAnswer), input, h('p', { class: 'hint', id: `lqh${b.index}` }, T.lp.answerHint)));
        if (!done) out.push(h('div', { class: 'row' }, button(T.learn.check, submit, 'primary', busy ? { disabled: true } : {})));
      }
      if (!done) {
        const tried = st.wrong.size > 0 || st.hint > 0;
        const row = [];
        if (st.hint < 2) row.push(button([icon('hint', 18), st.hint === 0 ? T.lp.hint : T.lp.hintMore], askHint, 'quiet'));
        if (tried) row.push(button(T.lp.showSolutionBtn, () => send({ give_up: true }, null), 'quiet'));
        if (row.length) out.push(h('div', { class: 'row' }, row));
      }
      if (st.hint >= 1 && !done) {
        const tgt = hintTarget();
        out.push(h('div', { class: 'hintbox', 'aria-live': 'polite' },
          h('p', {}, h('strong', {}, `${T.lp.hint}: `), st.hint === 1 ? T.lp.hint1 : (b.type === 'quiz' ? T.lp.hint2Quiz : T.lp.hint2Method)),
          tgt !== null ? button(T.lp.hintShowText, () => { jumpFrom = i; goStep(tgt); }, 'small') : null));
      }
      if (st.note) {
        out.push(h('div', { 'aria-live': 'polite' }, notice(st.note.text, st.note.kind), st.note.answer ? h('p', { class: 'correct-was' }, h('strong', {}, `${T.learn.correctWas}`), st.note.answer) : null));
      }
      box.replaceChildren(...out);
    };
    draw();
    return box;
  };

  // ---- пояснення своїми словами ----
  const explainBlock = (b) => {
    const st = mem(`x${b.index}`, () => ({ text: '', ideas: null, skipped: false }));
    const box = h('div', { class: 'lesson-block explain' });
    const ta = h('textarea', { class: 'notes short', maxlength: 600, placeholder: T.lp.explainPlaceholder, 'aria-label': b.prompt });
    ta.value = st.text;
    const counter = h('p', { class: 'hint', 'aria-live': 'off' });
    const upd = () => { st.text = ta.value; counter.textContent = T.lp.wordsCount(distinctWords(ta.value)); };
    ta.addEventListener('input', upd);
    const head = h('p', {}, labelled(T.lp.explain, b.prompt));
    const done = () => open.xp.has(b.index);
    const draw = () => {
      if (done() && !st.ideas) { box.replaceChildren(head, notice(T.lp.explainDone, 'good')); return; }
      if (!st.ideas) {
        const msg = h('div', { 'aria-live': 'polite' });
        upd();
        box.replaceChildren(head, ta, counter, msg,
          h('div', { class: 'row' },
            button(T.lp.showIdeas, guarded(async () => {
              const r = await call('lesson_explain', { skill: id, index: b.index, text: ta.value });
              if (!r.accepted) { msg.replaceChildren(notice(r.message, 'info')); ta.focus(); return; }
              st.ideas = r.key_ideas; draw();
            }), 'primary'),
            button(T.lp.dontKnow, guarded(async () => {
              const r = await call('lesson_explain', { skill: id, index: b.index, skip: true });
              st.ideas = r.key_ideas; st.skipped = true; draw();
            }), 'quiet')));
        return;
      }
      const checks = st.ideas.map(() => h('input', { type: 'checkbox' }));
      box.replaceChildren(head,
        st.text.trim() ? h('blockquote', { class: 'own-text' }, h('strong', {}, `${T.lp.yourText}: `), st.text) : null,
        h('p', {}, st.skipped ? T.lp.ideasSkipped : T.lp.compare),
        h('ul', { class: 'ideas' }, st.ideas.map((t, k) => h('li', {}, h('label', {}, checks[k], h('span', {}, t))))),
        h('div', { class: 'row' }, button(T.lp.done, guarded(async () => {
          if (st.skipped) {
            open.xp.add(b.index); burnt.add(steps.findIndex((s) => s.includes(b)) + 1);
          } else {
            const r = await call('lesson_explain', { skill: id, index: b.index, text: st.text, recalled: checks.map((c) => c.checked) });
            sync(r.progress); complete = r.progress.complete;
          }
          st.ideas = null; draw(); repaintNext(true);
        }), 'primary')));
    };
    draw();
    return box;
  };

  const hookBlock = (b) => {
    const st = mem(`h${b.prompt}`, () => ({ text: '' }));
    const ta = h('textarea', { class: 'notes short', maxlength: 300, placeholder: T.lp.hookPlaceholder, 'aria-label': b.prompt });
    ta.value = st.text;
    ta.addEventListener('input', () => {
      st.text = ta.value;
      const entry = `${b.prompt} ${ta.value.trim()}`;
      const at = guesses.findIndex((g) => g.startsWith(b.prompt));
      if (!ta.value.trim()) { if (at >= 0) guesses.splice(at, 1); } else if (at >= 0) guesses[at] = entry; else guesses.push(entry);
    });
    return h('div', { class: 'lesson-block hook' }, h('p', {}, labelled(T.lp.hook, b.prompt)), ta, h('p', { class: 'hint' }, T.lp.hookHint));
  };

  const blockNode = (b) => {
    switch (b.type) {
      case 'hook': return hookBlock(b);
      case 'text': return h('div', { class: 'lesson-block card prose' }, h('p', {}, b.text));
      case 'term': return h('div', { class: 'lesson-block term' }, h('strong', {}, b.term), h('span', {}, ` — ${lowerFirst(b.definition)}`));
      case 'figure': { const f = b.figure && figureSvg(b.figure); return h('div', { class: 'lesson-block figure-card' }, f, b.caption ? h('p', {}, b.caption) : null); }
      case 'example': return exampleBlock(b);
      case 'selfcheck':
      case 'quiz': return questionBlock(b);
      case 'explain': return explainBlock(b);
      case 'remember': return h('p', { class: 'lesson-block remember' }, h('strong', {}, `${T.learn.remember}. `), b.text);
      default: return null;
    }
  };

  const speakText = () => steps[i].map((b) => {
    if (b.type === 'example') return b.visibleText ? b.visibleText() : b.statement;
    if (b.type === 'term') return `${b.term}. ${b.definition}`;
    if (b.type === 'quiz') return `${b.prompt}. ${b.options.join('. ')}`;
    return b.text || b.prompt || '';
  }).filter(Boolean).join(' ');

  const progressBar = () => {
    const fill = h('i', { class: 'fill' });
    fill.style.width = `${Math.round(((i + 1) / steps.length) * 100)}%`;
    return h('div', { class: 'lp-progress' },
      h('div', { class: 'lp-bar', role: 'progressbar', 'aria-valuemin': 1, 'aria-valuemax': steps.length, 'aria-valuenow': i + 1, 'aria-valuetext': T.lp.progressText(i + 1, steps.length) }, fill),
      h('span', { class: 'lp-count' }, T.learn.stepOf(i + 1, steps.length)));
  };

  function paint() {
    if (jumpFrom === i) jumpFrom = null;
    nextSlot = h('div', { class: 'pager-nav' });
    finishSlot = h('div', {});
    lockNote = h('p', { class: 'lp-lock', role: 'status' });
    const lockWrap = h('div', { class: 'lp-lockwrap' }, icon('lock', 16), lockNote);
    mount(
      h('div', { class: 'toolbar' }, h('h1', {}, l.title),
        iconOnly('back', returnTo === 'session' ? T.lp.backToTasks : T.lp.toTopic, () => go(returnTo === 'session' ? 'session' : 'skill', returnTo === 'session' ? { start: false } : { id }), 'quiet')),
      progressBar(),
      resumed && i > 0 && !complete ? h('p', { class: 'hint center' }, T.lp.resumed(i + 1), ' ', button(T.lp.fromStart, () => { resumed = false; goStep(0); }, 'quiet small')) : null,
      jumpFrom !== null ? h('div', { class: 'row' }, button(T.lp.backToQuestion, () => goStep(jumpFrom), 'small')) : null,
      h('h2', { class: 'sr-only', id: 'lp-step', tabindex: '-1' }, T.lp.stepHeading(i + 1, stepKind(steps[i]))),
      speechOn() ? h('div', { class: 'row' }, speakButton(speakText, T.task.speak)) : null,
      ...steps[i].map(blockNode),
      lockWrap, nextSlot, finishSlot);
    resumed = false;
    repaintNext();
    const hd = document.getElementById('lp-step');
    if (hd) hd.focus({ preventScroll: true });
  }
  paint();
});

// Зошит конспекту: окремі поля за запитаннями-каркасами уроку (не вставлений текст), словник теми для підгляду.
register('note', async ({ id, prompts }) => {
  const [n, info] = await Promise.all([call('note_get', { skill: id }), call('skill_page', { skill: id })]);
  let terms = [];
  try { const l = await call('lesson', { skill: id }); terms = l.blocks.filter((b) => b.type === 'term'); } catch { /* уроку може не бути */ }
  const labels = (prompts || info.note_prompts || []).slice(0, 5);
  const saved = new Map((n.fields || []).map((f) => [f.label, f.text]));
  const all = [...labels, ...[...saved.keys()].filter((k) => k && !labels.includes(k))];
  if (!all.length) all.push('');
  const free = !labels.length ? [''] : [];
  const areas = (free.length ? free : all).map((label) => {
    const ta = h('textarea', { class: 'notes', maxlength: 2000, placeholder: T.learn.notesPlaceholder, 'aria-label': label || T.learn.notesTitle });
    ta.value = saved.get(label) || '';
    return { label, ta };
  });
  const msg = h('div', { 'aria-live': 'polite' });
  const save = guarded(async () => {
    const r = await call('note_set', { skill: id, fields: areas.map((a) => ({ label: a.label, text: a.ta.value })) });
    msg.replaceChildren(...[notice(T.learn.notesSaved, 'good'), badgeList(r.new_badges)].filter(Boolean));
  });
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.learn.notesTitle), iconOnly('back', T.back, guarded(async () => { await save({ currentTarget: null }); await go('skill', { id }); }), 'quiet')),
    h('p', { class: 'muted' }, info.title),
    terms.length ? h('details', { class: 'card words' }, h('summary', {}, T.lp.words),
      h('dl', { class: 'terms' }, terms.map((t) => [h('dt', {}, t.term), h('dd', {}, t.definition)]))) : null,
    h('p', { class: 'hint' }, T.lp.notesHint),
    h('div', { class: 'notebook' }, areas.map((a) => h('div', { class: 'nb-field' },
      a.label ? h('label', { class: 'nb-label' }, a.label) : null, a.ta))),
    msg, h('div', { class: 'row' }, button(T.learn.notesSave, save, 'primary')));
});
