// Кабінет вчителя: окремий профіль із власною обкладинкою «Класний журнал». У кабінеті немає даних учнів.
import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, field, guarded, downloadBase64, iconOnly, icon, face } from './ui.js';
import { register, go, state } from './router.js';
import { errText } from './start.js';
import { figureSvg } from './art.js';

const tt = T.teacher;

register('role', async () => {
  const card = (cls, title, text, onclick) => h('button', { type: 'button', class: `role-card ${cls}`, onclick },
    h('span', { class: 'rc-title' }, title), h('span', { class: 'rc-text' }, text));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.roleTitle), iconOnly('back', T.back, () => go('profiles'), 'quiet')),
    h('div', { class: 'stack' },
      card('rc-child', T.roleChild, T.roleChildHint, () => go('adultnotice', { next: 'create' })),
      card('rc-teacher', T.roleTeacher, T.roleTeacherHint, () => go('teacher_create'))),
    h('p', { class: 'hint' }, T.roleNote));
});

register('teacher_create', async () => {
  const nick = h('input', { type: 'text', maxlength: 24, autocomplete: 'off' });
  const pin1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const pin2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div');
  const submit = guarded(async () => {
    if (pin1.value !== pin2.value) { msg.replaceChildren(notice(T.pinMismatch, 'error')); return; }
    try {
      const r = await call('create_profile', { kind: 'teacher', nickname: nick.value, pin: pin1.value });
      state.slot = r.slot;
      state.kind = 'teacher';
      await go('recovery', { code: r.recovery_code, next: 'cabinet' });
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('h1', {}, tt.createTitle),
    h('p', {}, tt.createText),
    field(tt.nickname, nick, tt.nicknameHint),
    field(tt.formal.pinNewLabel, pin1, tt.formal.pinNewHint),
    field(tt.formal.pinRepeat, pin2), msg,
    h('div', { class: 'row' }, button(T.create, submit, 'primary'), button(T.back, () => go('role'))));
  nick.focus();
});

const lockBtn = () => iconOnly('exit', tt.lockLabel, async () => { await call('lock'); state.kind = null; await go('profiles'); }, 'quiet');

// Невелике свято в кабінеті (віха робіт чи календарна дата): показується один раз, Сова вітає й тихо зникає.
function celebration(c) {
  if (!c) return null;
  const box = h('section', { class: 'celebration', 'aria-label': c.title },
    h('span', { class: 'owl', 'aria-hidden': 'true' }, face(3)),
    h('div', {}, h('h2', {}, c.title), h('p', {}, c.text),
      h('div', { class: 'row' }, button(tt.thanksBtn, guarded(async () => {
        try { await call('teacher_celebrate_seen', { id: c.id }); } catch { /* не критично */ }
        box.remove();
      }), 'primary'))));
  return box;
}

register('cabinet', async () => {
  const d = await call('teacher_home');
  const tile = (title, text, onclick) => h('button', { type: 'button', class: 'tile-card', onclick },
    h('span', { class: 'tc-title' }, title),
    h('span', { class: 'tc-text' }, text));
  const recent = d.works.length
    ? h('ul', { class: 'plain' }, d.works.map((w) => h('li', {},
        h('button', { type: 'button', class: 'btn quiet link-row', 'aria-label': `${tt.repeatWork}: ${tt.workLine(T.teacher.kind[w.kind] || w.kind, w.grade, w.skills.length)}`, onclick: () => go('tworks', { grade: w.grade, again: w }) },
          tt.workLine(T.teacher.kind[w.kind] || w.kind, w.grade, w.skills.length)))))
    : h('p', { class: 'muted' }, tt.noWorks);
  const marks = d.bookmarks.length
    ? h('ul', { class: 'plain' }, d.bookmarks.map((b) => h('li', {}, h('button', { type: 'button', class: 'btn quiet link-row', onclick: () => go('tlesson', { id: b.id, grade: b.grade }) }, `${b.title} · ${tt.gradeWord(b.grade)}`))))
    : h('p', { class: 'muted' }, tt.noBookmarks);
  mount(
    h('div', { class: 'cabinet-cover' },
      h('div', { class: 'cc-top' }, h('span', { class: 'cc-brand' }, tt.forTeachers), h('div', { class: 'cc-actions' }, lockBtn())),
      h('h1', { class: 'cc-title' }, tt.cabinet),
      h('p', { class: 'cc-name' }, tt.hello(d.nickname))),
    celebration(d.celebration),
    h('p', { class: 'hint' }, tt.privacy),
    h('div', { class: 'stack' },
      tile(tt.tiles.works, tt.tiles.worksText, () => go('tworks')),
      tile(tt.tiles.notes, tt.tiles.notesText, () => go('tnotes')),
      tile(tt.tiles2.plan, tt.tiles2.planText, () => go('tplan')),
      tile(tt.data.tile, tt.data.tileText, () => go('tdata'))),
    h('h2', {}, tt.bookmarks), marks,
    h('h2', {}, tt.lastWorks), recent);
});

register('tworks', async ({ grade, again } = {}) => {
  const data = await call('teacher_topics', { grade: grade || 5 });
  const picked = new Set(again ? again.skills : []);
  const cfg = again ? { kind: again.kind, variants: again.variants, level: again.level || 'any', large: !!again.large } : { kind: 'test', variants: 2, level: 'any', large: false };
  const out = h('div', { 'aria-live': 'polite' });
  const counter = h('p', { class: 'hint' }, tt.pickCount(0));

  const gradeRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.grade }, data.grades.map((g) =>
    h('button', { type: 'button', class: 'btn', 'aria-pressed': String(g === data.grade), onclick: () => go('tworks', { grade: g }) }, `${g} клас`)));

  const boxes = [];
  const subjects = data.subjects.map((s, i) => h('details', { class: 'card', open: (again ? s.skills.some((k) => picked.has(k.id)) : i === 0) ? true : null },
    h('summary', {}, s.title),
    h('div', { class: 'checks' }, s.skills.map((k) => {
      const cb = h('input', { type: 'checkbox', value: k.id });
      cb.checked = picked.has(k.id);
      cb.addEventListener('change', () => {
        if (cb.checked) picked.add(k.id); else picked.delete(k.id);
        counter.textContent = tt.pickCount(picked.size);
        boxes.forEach((b) => { b.disabled = !b.checked && picked.size >= 8; });
      });
      boxes.push(cb);
      return h('label', { class: 'check' }, cb, h('span', {}, k.title));
    }))));

  const earlier = (data.earlier || []).filter((e) => e.subjects.length).map((e) => h('details', { class: 'card' },
    h('summary', {}, `${tt.earlierTitle}: ${tt.gradeWord(e.grade)}`),
    ...e.subjects.map((sub) => h('div', {}, h('h3', {}, sub.title), h('div', { class: 'checks' }, sub.skills.map((k) => {
      const cb = h('input', { type: 'checkbox', value: k.id });
      cb.checked = picked.has(k.id);
      cb.addEventListener('change', () => {
        if (cb.checked) picked.add(k.id); else picked.delete(k.id);
        counter.textContent = tt.pickCount(picked.size);
        boxes.forEach((b) => { b.disabled = !b.checked && picked.size >= 8; });
      });
      boxes.push(cb);
      return h('label', { class: 'check' }, cb, h('span', {}, k.title));
    }))))));
  const chip = (group, key, value, label) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(cfg[key] === value), onclick: (ev) => {
    cfg[key] = value;
    group.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    ev.currentTarget.setAttribute('aria-pressed', 'true');
  } }, label);
  const kindRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.kindTitle });
  kindRow.append(...Object.entries(tt.kind).map(([k, label]) => chip(kindRow, 'kind', k, label)));
  const levelRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.levelTitle });
  levelRow.append(...Object.entries(tt.levels).map(([k, label]) => chip(levelRow, 'level', k, label)));
  const largeBox = h('input', { type: 'checkbox' });
  largeBox.checked = cfg.large;
  largeBox.addEventListener('change', () => { cfg.large = largeBox.checked; });
  const varRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.variants });
  varRow.append(...[1, 2, 3, 4].map((n) => chip(varRow, 'variants', n, String(n))));

  const args = (extra = {}) => ({ grade: data.grade, skills: [...picked], kind: cfg.kind, variants: cfg.variants, level: cfg.level, large: cfg.large, ...extra });
  // PDF створюються з того самого вибору, що й робота (знімок на момент «Створити»), навіть якщо потім змінили налаштування.
  const pdfButton = (label, part, snap, primary) => button(label, guarded(async () => {
    try {
      const r = await call('teacher_work_pdf', { ...snap, part });
      if (await downloadBase64(r.data, r.filename, 'application/pdf')) out.append(notice(tt.saved, 'good'));
    } catch (e) { out.replaceChildren(notice(errText(e), 'error')); }
  }), primary ? 'primary' : '');

  const create = guarded(async () => {
    if (!picked.size) { out.replaceChildren(notice(tt.needTopic, 'warn')); return; }
    try {
      const snapArgs = args(again ? { seed: again.seed } : {});
      const r = await call('teacher_work_build', snapArgs);
      const snap = { ...snapArgs, seed: r.seed };
      const v = r.work.variants[0];
      out.replaceChildren(
        h('div', { class: 'card' },
          h('h2', {}, tt.ready),
          h('p', { class: 'hint' }, tt.seedNote(r.seed)),
          h('div', { class: 'row' }, pdfButton(tt.pdfTasks, 'tasks', snap, true), pdfButton(tt.pdfKey, 'key', snap), pdfButton(tt.pdfAnswers, 'answers', snap), pdfButton(tt.pdfCards, 'cards', snap)),
          h('h3', {}, tt.firstVariant),
          h('ol', { class: 'work-preview' }, v.items.map((it) => h('li', {}, it.statement, it.choices.length ? h('span', { class: 'muted' }, ` (${it.choices.join(' / ')})`) : null)))));
      out.scrollIntoView({ block: 'nearest' });
    } catch (e) { out.replaceChildren(notice(errText(e), 'error')); }
  });

  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, tt.worksTitle), iconOnly('back', T.back, () => go('cabinet'), 'quiet')),
    h('h2', {}, tt.grade), gradeRow,
    h('h2', {}, tt.topics), counter, ...subjects, ...earlier,
    h('h2', {}, tt.kindTitle), kindRow,
    h('h2', {}, tt.levelTitle), levelRow, h('p', { class: 'hint' }, tt.levelNote),
    h('label', { class: 'check' }, largeBox, h('span', {}, tt.largePrint)),
    h('h2', {}, tt.variants), varRow,
    h('div', { class: 'row' }, button(tt.create, create, 'primary')), out);
  counter.textContent = tt.pickCount(picked.size);
  if (again) create({ currentTarget: null });
});

register('tnotes', async ({ grade } = {}) => {
  const data = await call('teacher_topics', { grade: grade || 5 });
  const gradeRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.grade }, data.grades.map((g) =>
    h('button', { type: 'button', class: 'btn', 'aria-pressed': String(g === data.grade), onclick: () => go('tnotes', { grade: g }) }, `${g} клас`)));
  const subjects = data.subjects.map((s, i) => h('details', { class: 'card', open: i === 0 ? true : null },
    h('summary', {}, s.title),
    h('ul', { class: 'plain' }, s.skills.map((k) => h('li', {},
      h('button', { type: 'button', class: 'btn quiet link-row', onclick: () => go('tlesson', { id: k.id, grade: data.grade }) }, k.title))))));
  const results = h('div', { 'aria-live': 'polite' });
  const q = h('input', { type: 'search', maxlength: 60, 'aria-label': tt.searchLabel, placeholder: tt.searchLabel });
  const doSearch = guarded(async () => {
    const r = await call('teacher_search', { q: q.value });
    results.replaceChildren(r.hits.length
      ? h('ul', { class: 'plain' }, r.hits.map((x) => h('li', {},
          h('button', { type: 'button', class: 'btn quiet link-row', onclick: () => go('tlesson', { id: x.id, grade: x.grade }) },
            h('span', {}, `${x.title} · ${tt.gradeWord(x.grade)}`), x.snippet ? h('span', { class: 'muted snippet' }, ` ${x.snippet}`) : null))))
      : (q.value.trim().length >= 2 ? notice(tt.noHits, 'info') : null));
  });
  q.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { ev.preventDefault(); doSearch({ currentTarget: null }); } });
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, tt.notesTitle), iconOnly('back', T.back, () => go('cabinet'), 'quiet')),
    h('p', { class: 'muted' }, tt.notesIntro),
    h('div', { class: 'row' }, q, button(tt.searchGo, doSearch, 'primary')), h('p', { class: 'hint' }, tt.searchHint), results,
    gradeRow, ...subjects);
});

register('tlesson', async ({ id, grade }) => {
  const l = await call('teacher_lesson', { skill: id });
  const checks = (l.reviewer && l.reviewer.checks) || [];
  const explains = (l.reviewer && l.reviewer.explains) || [];
  const out = h('div', { 'aria-live': 'polite' });
  const home = await call('teacher_home');
  let marked = home.bookmarks.some((b) => b.id === id);
  const bookmarkBtn = () => {
    const b = button(marked ? tt.removeBookmark : tt.addBookmark, guarded(async () => {
      marked = !marked;
      await call('teacher_bookmark', { skill: id, on: marked });
      b.textContent = marked ? tt.removeBookmark : tt.addBookmark;
      b.setAttribute('aria-pressed', String(marked));
    }), 'quiet', { 'aria-pressed': String(marked) });
    return b;
  };
  const block = (b) => {
    switch (b.type) {
      case 'text': return h('p', {}, b.text);
      case 'hook': return h('p', { class: 'hint' }, b.prompt);
      case 'term': return h('p', {}, h('strong', {}, `${b.term}: `), b.definition);
      case 'remember': return h('div', { class: 'notice info' }, b.text);
      case 'figure': return h('figure', {}, figureSvg(b.figure), b.caption ? h('figcaption', { class: 'muted' }, b.caption) : null);
      case 'example': return h('div', { class: 'card' }, h('p', {}, h('strong', {}, b.statement)), h('ol', {}, b.steps.map((x) => h('li', {}, x))), h('p', {}, b.result));
      case 'selfcheck':
      case 'quiz': {
        const c = checks.find((x) => x.index === b.index) || {};
        return h('div', { class: 'card' },
          h('p', {}, h('strong', {}, b.prompt)),
          b.options ? h('ul', {}, b.options.map((o) => h('li', { class: o === c.answer ? 'teacher-correct' : null }, o))) : null,
          h('p', { class: 'teacher-correct' }, `${tt.answerLabel}: ${c.answer || ''}`),
          c.explanation ? h('p', { class: 'muted' }, c.explanation) : null);
      }
      case 'explain': {
        const e = explains.find((x) => x.index === b.index) || { key_ideas: [] };
        return h('div', { class: 'card' }, h('p', {}, h('strong', {}, b.prompt)), h('p', { class: 'hint' }, tt.keyIdeas), h('ul', {}, e.key_ideas.map((k) => h('li', {}, k))));
      }
      default: return null;
    }
  };
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, l.title), iconOnly('back', T.back, () => go('tnotes', { grade: grade || l.grade }), 'quiet')),
    h('p', {}, h('span', { class: 'pill' }, tt.reviewStatus[l.review] || l.review)),
    h('div', { class: 'row' }, bookmarkBtn(), button(tt.handoutPdf, guarded(async () => {
      try {
        const r = await call('teacher_handout_pdf', { skill: id });
        if (await downloadBase64(r.data, r.filename, 'application/pdf')) out.replaceChildren(notice(tt.saved, 'good'));
      } catch (e) { out.replaceChildren(notice(errText(e), 'error')); }
    }), 'quiet'), button(tt.lessonPlanPdf, guarded(async () => {
      try {
        const r = await call('teacher_lesson_plan_pdf', { skill: id });
        if (await downloadBase64(r.data, r.filename, 'application/pdf')) out.replaceChildren(notice(tt.saved, 'good'));
      } catch (e) { out.replaceChildren(notice(errText(e), 'error')); }
    }), 'primary')),
    out,
    ...l.blocks.map(block));
});

register('tdata', async () => {
  const d = tt.data;
  const msg = h('div', { 'aria-live': 'polite' });
  const zone = h('div');
  const SCALES = [90, 100, 115, 130, 150];
  const cur = Number((() => { try { return localStorage.getItem('scale'); } catch { return null; } })()) || 100;
  const sizeRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': d.textSize }, SCALES.map((sc) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(sc === cur), onclick: (ev) => {
    try { localStorage.setItem('scale', String(sc)); } catch { /* не критично */ }
    document.documentElement.style.setProperty('--scale', String(sc / 100));
    sizeRow.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    ev.currentTarget.setAttribute('aria-pressed', 'true');
  } }, `${sc} %`)));
  const askErase = () => {
    const word = h('input', { type: 'text', autocomplete: 'off', autocapitalize: 'off', spellcheck: 'false' });
    zone.replaceChildren(notice(d.eraseWarn, 'warn'), field(d.erase, word),
      h('div', { class: 'row' }, button(d.eraseConfirm, guarded(async () => {
        if (word.value.trim().toLowerCase() !== d.eraseWord) { msg.replaceChildren(notice(d.eraseWrong, 'error')); return; }
        try { await call('erase_profile', { slot: state.slot }); state.slot = null; state.kind = null; await go('profiles'); }
        catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
      }), 'danger'), button(T.cancel, () => zone.replaceChildren())));
  };
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, d.title), iconOnly('back', T.back, () => go('cabinet'), 'quiet')),
    h('h2', {}, d.textSize), sizeRow,
    h('h2', {}, d.about), h('ul', { class: 'prose' }, d.items.map((x) => h('li', {}, x))),
    h('h2', {}, T.recoveryTitle), h('p', { class: 'muted' }, d.newCodeNote),
    h('div', { class: 'row' }, button(d.newCode, guarded(async () => {
      try { const r = await call('recovery_new'); msg.replaceChildren(h('p', { class: 'code' }, r.recovery_code)); }
      catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
    }))),
    msg,
    h('h2', {}, d.erase), zone,
    h('div', { class: 'row' }, button(d.erase, askErase, 'danger')));
});

register('tplan', async ({ grade } = {}) => {
  const data = await call('teacher_topics', { grade: grade || 5 });
  const cfg = { subject: data.subjects[0] ? data.subjects[0].id : '', lessons: 2 };
  const out = h('div', { 'aria-live': 'polite' });
  const gradeRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.grade }, data.grades.map((g) =>
    h('button', { type: 'button', class: 'btn', 'aria-pressed': String(g === data.grade), onclick: () => go('tplan', { grade: g }) }, `${g} клас`)));
  const pick = (row, key, value, label) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(cfg[key] === value), onclick: (ev) => {
    cfg[key] = value;
    row.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    ev.currentTarget.setAttribute('aria-pressed', 'true');
  } }, label);
  const subjRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.planSubject });
  subjRow.append(...data.subjects.map((s) => pick(subjRow, 'subject', s.id, s.title)));
  const lessonsRow = h('div', { class: 'tabs wrap', role: 'group', 'aria-label': tt.planLessons });
  lessonsRow.append(...[1, 2, 3, 4].map((n) => pick(lessonsRow, 'lessons', n, String(n))));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, tt.planTitle), iconOnly('back', T.back, () => go('cabinet'), 'quiet')),
    h('h2', {}, tt.grade), gradeRow,
    h('h2', {}, tt.planSubject), subjRow,
    h('h2', {}, tt.planLessons), lessonsRow,
    h('div', { class: 'row' }, button(tt.planPdf, guarded(async () => {
      try {
        const r = await call('teacher_plan_pdf', { grade: data.grade, subject: cfg.subject, lessons: cfg.lessons });
        if (await downloadBase64(r.data, r.filename, 'application/pdf')) out.replaceChildren(notice(tt.saved, 'good'));
      } catch (e) { out.replaceChildren(notice(errText(e), 'error')); }
    }), 'primary')), out);
});
