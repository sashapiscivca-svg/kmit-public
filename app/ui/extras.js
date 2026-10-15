import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, field, guarded, downloadBase64, iconOnly, icon } from './ui.js';
import { register, go, state } from './router.js';
import { applySettings } from './home.js';
import { BADGE_ART } from './session.js';
import { sticker } from './ui.js';
import { creature } from './art.js';
import { heroSvg } from './hero.js';
import { errText } from './start.js';

const back = () => iconOnly('back', T.back, () => go('home'), 'quiet');

register('album', async ({ grade } = {}) => {
  const a = await call('album', grade ? { grade } : {});
  const world = a.world.id;
  const rows = (subjectId) => {
    const bySkill = new Map();
    for (const c of a.cells.filter((x) => x.subject === subjectId)) {
      if (!bySkill.has(c.skill)) bySkill.set(c.skill, { title: c.skill_title, cells: [] });
      bySkill.get(c.skill).cells.push(c);
    }
    return [...bySkill.values()].map((row) => h('div', { class: 'album-row' },
      h('span', { class: 'title' }, row.title),
      h('span', { class: 'slots' }, row.cells.map((c) => h('span', { class: `slot ${c.earned ? 'earned' : ''}`, role: 'img', 'aria-label': `${T.kinds[c.kind]}: ${c.earned ? 'є' : 'ще немає'}` }, creature(c.skill, c.kind, c.subject, c.earned, 40, world))))));
  };
  const legend = h('ul', { class: 'legend' }, ['mastery', 'persisted', 'self_corrected', 'asked_and_solved'].map((k) =>
    h('li', {}, h('span', { class: 'lg-art' }, creature('SAMPLE-LEGEND', k, 'math', true, 44, world)), h('span', {}, T.collectionKinds[k]))));
  const count = (id) => {
    const cells = a.cells.filter((x) => x.subject === id);
    return T.collectionBySubject(cells.filter((c) => c.earned).length, cells.length);
  };
  const worldChips = a.worlds.length > 1
    ? h('div', { class: 'grade-row', role: 'group', 'aria-label': T.world.others }, a.worlds.map((w) =>
      h('button', { type: 'button', class: 'btn', 'aria-pressed': String(w.grade === a.grade), onclick: () => go('album', { grade: w.grade }) }, T.world.chip(w.grade, w.world))))
    : null;
  const certs = a.worlds.filter((w) => w.ceremony_done).map((w) => button(T.world.certificate(w.grade), guarded(async () => {
    const r = await call('ceremony_pdf', { from: w.grade });
    await downloadBase64(r.data, r.filename, 'application/pdf');
  }), 'quiet'));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.albumTitle), iconOnly('back', T.back, () => go('home'), 'quiet')),
    worldChips,
    h('section', { class: `album-world w-${world}` },
      h('h2', {}, a.world.name),
      h('p', {}, T.world.intro(a.world.name)),
      h('div', { class: 'starter-row' }, h('span', { class: 'starter-art', role: 'img', 'aria-label': T.world.starter }, creature(`start:${world}`, 'gift', 'math', true, 64, world)),
        h('span', {}, h('strong', {}, T.world.starter), h('br'), h('span', { class: 'muted' }, T.world.starterNote)))),
    h('p', { class: 'muted' }, T.albumText),
    h('h2', {}, T.stickersTitle),
    h('p', {}, T.collectionHow), legend,
    h('p', { class: 'hint' }, T.collectionNote),
    h('p', {}, h('strong', {}, T.collectionTotal(a.count))),
    ...a.subjects.map((s) => h('div', {}, h('h3', {}, `${s.title}: ${count(s.id)}`), h('div', {}, rows(s.id)))),
    certs.length ? h('div', { class: 'row' }, certs) : null,
    h('h2', {}, `${T.badgesTitle}. ${T.badgesCount(a.badges_count, a.badges_total)}`),
    ...['lesson', 'note', 'family', 'course'].map((g) => h('div', {},
      h('h3', {}, T.badgeGroups[g]),
      h('div', { class: 'stickers' }, a.badges.filter((b) => b.group === g).map((b) =>
        h('div', { class: `sticker ${b.earned ? 'earned' : 'empty'}`, 'aria-label': `${b.title}: ${b.earned ? 'є' : 'ще немає'}. ${b.description}` },
          sticker(BADGE_ART[g], 56), h('strong', {}, b.title)))))));
});


const list = (items, render) => items.length
  ? h('ul', {}, items.map(render))
  : h('p', { class: 'muted' }, T.sheetNone);

register('sheet', async ({ subject = 'math' } = {}) => {
  const [s, hm, hero] = await Promise.all([call('sheet', { subject }), call('home'), call('hero_catalog')]);
  const pdf = guarded(async () => {
    const r = await call('sheet_pdf', { subject });
    await downloadBase64(r.data, r.filename, 'application/pdf');
  });
  const tile = (n, label, cls) => h('div', { class: `tile ${cls}` }, h('strong', {}, String(n)), h('span', {}, label));
  const card = (title, cls, body) => h('section', { class: `sheet-card ${cls}` }, h('h2', {}, title), body);
  mount(
    h('div', { class: 'toolbar noprint' }, h('h1', {}, T.sheetTitle), back()),
    h('p', { class: 'muted noprint' }, T.sheetText),
    h('div', { class: 'grade-row noprint', role: 'group', 'aria-label': T.sheetSubject }, hm.subjects.map((x) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(x.id === subject), onclick: () => go('sheet', { subject: x.id }) }, x.title))),
    h('div', { class: 'row noprint' }, button(T.sheetPdf, pdf, 'primary'), button(T.sheetPrint, () => window.print()), button(T.thanks.link, () => go('thanks'))),
    h('div', { class: `sheet-doc s-${subject}` },
      h('header', { class: 'sheet-band' },
        h('div', {}, h('p', { class: 'kicker' }, T.sheetKicker), h('h2', { class: 'hand-big' }, s.subject), h('p', {}, h('strong', {}, s.nickname)), h('p', { class: 'small' }, T.sheetNote)),
        h('div', { class: 'sheet-hero' }, heroSvg(hero.look.px, 72))),
      h('div', { class: 'tiles' }, tile(s.mastered.length, T.sheetMastered, 't-ok'), tile(s.in_progress.length, T.sheetProgress, 't-pen'),
        tile(s.to_repeat.length, T.sheetRepeat, 't-mark'), tile(s.gaps.length, T.sheetGaps, 't-red')),
      card(T.sheetMastered, 'c-ok', list(s.mastered, (x) => h('li', {}, x.title))),
      s.to_repeat.length ? card(T.sheetRepeat, 'c-mark', list(s.to_repeat, (x) => h('li', {}, x.title))) : null,
      card(T.sheetProgress, 'c-pen', list(s.in_progress, (x) => h('li', {}, x.title))),
      card(T.sheetGaps, 'c-red', list(s.gaps, (g) => h('li', {}, h('strong', {}, g.title),
        g.hypotheses.length ? h('ul', {}, g.hypotheses.map((x) => h('li', {}, x))) : null,
        g.ideas.length ? h('ul', {}, g.ideas.map((x) => h('li', {}, x))) : null))),
      card(T.sheetAsk, 'c-purple', list(s.ask_adult, (x) => h('li', {}, `${x.title}: ${x.statement}`))),
      null));
});

// ---- Подяка вчителю: дитина пише своїми словами, застосунок робить листівку чи текст ----
register('thanks', async () => {
  const hero = await call('hero_catalog');
  const prof = await call('home');
  const to = h('input', { type: 'text', maxlength: 60, placeholder: T.thanks.toPlaceholder, 'aria-label': T.thanks.to });
  const ta = h('textarea', { class: 'notes short', maxlength: 600, placeholder: T.thanks.placeholder, 'aria-label': T.thanks.text });
  const from = h('input', { type: 'text', maxlength: 40, 'aria-label': T.thanks.from });
  from.value = prof.nickname || '';
  const msg = h('div', { 'aria-live': 'polite' });
  const run = (name) => guarded(async () => {
    msg.replaceChildren();
    try {
      const r = await call(name, { to: to.value, text: ta.value, from: from.value });
      await downloadBase64(r.data, r.filename, name === 'thanks_pdf' ? 'application/pdf' : 'text/plain');
      msg.replaceChildren(notice(T.thanks.done, 'good'));
    } catch (e) { msg.replaceChildren(notice(T.thanks.needText, 'info')); }
  });
  const starters = T.thanks.starters.map((t) => h('button', { type: 'button', class: 'btn chip', onclick: () => { ta.value = `${ta.value}${ta.value && !ta.value.endsWith(' ') ? ' ' : ''}${t} `; ta.focus(); } }, t));
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.thanks.title), back()),
    h('p', { class: 'muted' }, T.thanks.intro),
    field(T.thanks.to, to),
    h('p', { class: 'hint' }, T.thanks.startersHint), h('div', { class: 'chips' }, starters),
    field(T.thanks.text, ta),
    field(T.thanks.from, from),
    msg,
    h('div', { class: 'thanks-preview' }, h('p', { class: 'hand-big' }, T.thanks.thankYou), h('div', { class: 'tp-hero' }, heroSvg(hero.look.px, 56))),
    h('div', { class: 'row' }, button(T.thanks.pdf, run('thanks_pdf'), 'primary'), button(T.thanks.txt, run('thanks_text'))),
    h('p', { class: 'hint' }, T.thanks.hint));
});

// Налаштування: один екран із розділами, зміни зберігаються одразу.
register('settings', async () => {
  const s = await call('settings_get');
  const status = h('p', { class: 'save-note', 'aria-live': 'polite' });
  let timer = null;
  const saved = (r) => {
    applySettings(r);
    status.textContent = T.settingsSaved;
    clearTimeout(timer);
    timer = setTimeout(() => { status.textContent = ''; }, 1800);
  };
  const put = async (patch) => { try { saved(await call('settings_set', patch)); } catch (e) { status.textContent = errText(e); } };
  const toggle = (key, label, hint) => {
    const cb = h('input', { type: 'checkbox' });
    cb.checked = !!s[key];
    cb.addEventListener('change', () => put({ [key]: cb.checked }));
    return h('label', { class: 'check' }, cb, h('span', {}, label, hint ? h('small', { class: 'hint-inline' }, hint) : null));
  };
  const scale = h('input', { type: 'range', min: 80, max: 200, step: 10, value: s.font_scale_percent, 'aria-label': T.fontScale });
  const scaleVal = h('span', { class: 'pill' }, `${s.font_scale_percent}%`);
  scale.addEventListener('input', () => { scaleVal.textContent = `${scale.value}%`; applySettings({ font_scale_percent: Number(scale.value), reduce_motion: s.reduce_motion }); });
  scale.addEventListener('change', () => put({ font_scale_percent: Number(scale.value) }));
  const link = (ico, title, desc, to, params = {}) => h('button', { type: 'button', class: 'action', onclick: () => go(to, params) },
    h('span', { class: 'ico' }, icon(ico, 24)), h('span', {}, h('span', { class: 't' }, title), h('br'), h('span', { class: 'd' }, desc)));
  const group = (title, ...kids) => h('section', { class: 'set-group' }, h('h2', {}, title), ...kids);
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.settingsTitle), back()),
    status,
    group(T.settingsLook,
      h('div', { class: 'field' }, h('label', {}, T.fontScale, ' ', scaleVal), scale),
      toggle('reading_mode', T.settingsReading, T.settingsReadingHint),
      toggle('reduce_motion', T.reduceMotion),
      toggle('sound', T.sound, T.soundHint)),
    group(T.settingsLearning,
      toggle('lesson_first', T.settingsLessonFirst),
      toggle('ask_confidence', T.settingsConfidence),
      link('refresh', T.week.link, T.settingsWeekDesc, 'week'),
      link('book', T.grades.title, T.settingsGradeDesc, 'grades')),
    group(T.settingsMe,
      link('sticker', T.hero.title, T.settingsHeroDesc, 'hero'),
      link('sheet', T.data.title, T.data.desc, 'data')),
    group(T.settingsPrivacy,
      toggle('allow_screenshots', T.settingsScreenshots, T.settingsScreenshotsHint),
      link('lock', T.privacy.title, T.settingsPrivacyDesc, 'privacy', { to: 'settings' })),
    group(T.settingsGrownups,
      link('lock', T.adult, T.settingsAdultDesc, 'adult')),
    group(T.settingsHelp,
      link('hand', T.calm.link, T.settingsCalmDesc, 'calm')));
});

// «Мої дані»: що таке копія профілю, як її зберегти й як відновити.
register('data', async () => {
  const pass = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div', { 'aria-live': 'polite' });
  const exp = guarded(async () => {
    try {
      const r = await call('export_profile', { passphrase: pass.value });
      if (await downloadBase64(r.data, r.filename, 'application/octet-stream')) msg.replaceChildren(notice(T.data.saved, 'good'));
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.data.title), iconOnly('back', T.back, () => go('settings'), 'quiet')),
    h('p', {}, T.data.intro),
    h('div', { class: 'card' }, h('h2', {}, T.data.saveTitle), h('ol', { class: 'prose' }, T.data.saveSteps.map((x) => h('li', {}, x))),
      field(T.passphrase, pass, T.data.passHint), msg, h('div', { class: 'row' }, button(T.exportDo, exp, 'primary'))),
    h('div', { class: 'card' }, h('h2', {}, T.data.restoreTitle), h('ol', { class: 'prose' }, T.data.restoreSteps.map((x) => h('li', {}, x))),
      h('div', { class: 'row' }, button(T.data.restoreGo, async () => { await call('lock'); await go('import'); }))),
    notice(T.data.warn, 'warn'));
});

// ---- Режим дорослого ----
register('adult', async () => {
  const st = await call('adult_status');
  if (!st.unlocked) return adultUnlock();
  return adultPanel();
});

register('adultpin', async () => adultSetPin());

function adultSetPin() {
  const p1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const p2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div');
  const ok = guarded(async () => {
    if (p1.value !== p2.value) { msg.replaceChildren(notice(T.pinMismatch, 'error')); return; }
    try { await call("adult_set_pin", { pin: p1.value }); await go("adult"); }
    catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(h('div', { class: 'toolbar' }, h('h1', {}, T.adultSetPin), back()), h('p', {}, T.adultSetText),
    field(T.adultPin, p1), field(T.pinRepeat, p2), msg, h('div', { class: 'row' }, button(T.save, ok, 'primary')));
}

function adultUnlock() {
  const p = h('input', { type: 'password', autocomplete: 'off' });
  const msg = h('div');
  const ok = guarded(async () => {
    try { await call('adult_unlock', { pin: p.value }); await go('adult'); }
    catch (e) { msg.replaceChildren(notice(e.code === 'wrong_adult_pin' ? T.adultWrong : errText(e), 'error')); p.select(); }
  });
  p.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') ok({ currentTarget: null }); });
  mount(h('div', { class: 'toolbar' }, h('h1', {}, T.adultTitle), back()),
    field(T.adultPin, p), msg, h('div', { class: 'row' }, button(T.adultUnlock, ok, 'primary')));
  p.focus();
}

const mmToTime = (m) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const timeToMm = (t) => { const [a, b] = t.split(':').map(Number); return a * 60 + b; };

async function adultPanel() {
  const s = await call('settings_get');
  const items = (await call('adult_items')).items;
  const msg = h('div');
  const say = (text, kind = 'good') => msg.replaceChildren(notice(text, kind));
  const fail = (e) => say(errText(e), 'error');

  const qOn = h('input', { type: 'checkbox' }); qOn.checked = !!s.quiet_hours;
  const qFrom = h('input', { type: 'time', value: mmToTime(s.quiet_hours ? s.quiet_hours.from : 21 * 60) });
  const qTo = h('input', { type: 'time', value: mmToTime(s.quiet_hours ? s.quiet_hours.to : 7 * 60) });
  const extra = h('input', { type: 'checkbox' }); extra.checked = s.adult_extra_sessions;
  const lim = h('input', { type: 'checkbox' }); lim.checked = s.enforce_limits;
  const saveLimits = guarded(async () => {
    try {
      await call('settings_set', {
        quiet_hours: qOn.checked ? { from: timeToMm(qFrom.value), to: timeToMm(qTo.value) } : null,
        adult_extra_sessions: extra.checked,
        enforce_limits: lim.checked,
      });
      say(T.saved);
    } catch (e) { fail(e); }
  });

  const itemsList = items.length
    ? h('ul', {}, items.map((it) => h('li', {}, `${it.title}: ${it.statement} `,
        button(T.resolve, guarded(async () => {
          try { await call('adult_resolve', { skill: it.skill, statement: it.statement }); await go('adult'); } catch (e) { fail(e); }
        }), 'quiet'))))
    : h('p', { class: 'muted' }, T.itemsNone);

  const codeZone = h('div');
  const doNewCode = guarded(async () => {
    try { const r = await call('recovery_new'); await go('recovery', { code: r.recovery_code, next: 'adult' }); } catch (e) { fail(e); }
  });
  const newCode = () => codeZone.replaceChildren(
    notice(T.newRecoveryWarn, 'warn'),
    h('div', { class: 'row' }, button(T.newRecoveryYes, doNewCode, 'danger'), button(T.cancel, () => codeZone.replaceChildren())));

  const pass = h('input', { type: 'password', autocomplete: 'new-password' });
  const exp = guarded(async () => {
    try {
      const r = await call('export_profile', { passphrase: pass.value });
      if (await downloadBase64(r.data, r.filename, 'application/octet-stream')) say(T.saved);
    } catch (e) { fail(e); }
  });

  const st = await call('adult_status');
  const pinSection = st.pin_set
    ? h('p', { class: 'muted' }, T.adultPinOn)
    : h('div', {}, h('p', { class: 'muted' }, T.adultSetText), button(T.adultSetPin, () => go('adultpin')));

  const eraseZone = h('div');
  const askErase = async () => {
    const name = ((await call('home')).nickname || '').trim();
    const typed = h('input', { type: 'text', autocomplete: 'off', autocapitalize: 'off', spellcheck: 'false', id: 'erase-name' });
    const go_ = button(T.eraseConfirm, guarded(async () => {
      try { await call('erase_profile', { slot: state.slot }); state.slot = null; await go('profiles'); } catch (e) { fail(e); }
    }), 'danger', { disabled: !!name });
    typed.addEventListener('input', () => {
      const ok = typed.value.trim().toLowerCase() === name.toLowerCase();
      go_.disabled = !ok;
      go_.setAttribute('aria-disabled', String(!ok));
    });
    eraseZone.replaceChildren(
      notice(T.eraseWarn, 'warn'),
      name ? field(T.eraseTypeName(name), typed) : null,
      h('div', { class: 'row' }, go_, button(T.cancel, () => eraseZone.replaceChildren())));
    if (name) typed.focus();
  };

  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.adultTitle),
      iconOnly('back', T.back, guarded(async () => { await call('adult_lock'); await go('home'); }), 'quiet')),
    msg,
    h('h2', {}, T.limitsTitle), h('p', { class: 'muted' }, T.limitsText),
    h('label', { class: 'check' }, lim, h('span', {}, T.limitsOn)),
    h('label', { class: 'check' }, qOn, h('span', {}, T.quiet)),
    field(T.quietFrom, qFrom), field(T.quietTo, qTo),
    h('label', { class: 'check' }, extra, h('span', {}, T.extra)),
    h('div', { class: 'row' }, button(T.save, saveLimits, 'primary')),
    h('h2', {}, T.items), itemsList,
    h('h2', {}, T.adultSetPin), pinSection,
    h('h2', {}, T.recoveryTitle), h('p', { class: 'muted' }, T.recoveryNote),
    h('div', { class: 'row' }, button(T.newRecovery, newCode)), codeZone,
    h('h2', {}, T.data.title), h('p', { class: 'muted' }, T.data.desc), h('div', { class: 'row' }, button(T.data.title, () => go('data'))),
    h('h2', {}, T.packs.title), h('p', { class: 'muted' }, T.packs.text), packsBox(say, fail),
    h('h2', {}, T.flag.listTitle), h('p', { class: 'muted' }, T.flag.listText),
    h('div', { class: 'row' }, button(T.flag.save, guarded(async () => {
      const r = await call('flags_export');
      if (await downloadBase64(r.data, r.filename, 'application/json')) say(T.saved);
    }))),
    h('h2', {}, T.statsTitle), h('p', { class: 'muted' }, T.statsText),
    h('div', { class: 'row' }, button(T.statsSave, guarded(async () => {
      const r = await call('stats_export');
      if (await downloadBase64(r.data, r.filename, 'application/json')) say(T.saved);
    }))),
    h('h2', {}, T.erase), eraseZone,
    h('div', { class: 'row' }, button(T.erase, askErase, 'danger')));
}

// ---- Спокій, про дані ----
register('calm', async () => {
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.calm.title), back()),
    h('p', {}, T.calm.text),
    h('div', { class: 'breath', role: 'img', 'aria-label': T.calm.inhale }, h('div', { class: 'circle' }, T.calm.inhale)),
    h('ul', { class: 'prose' }, [T.calm.tip1, T.calm.tip2, T.calm.tip3].map((t) => h('li', {}, t))),
    h('div', { class: 'row' }, button(T.calm.back, () => go('home'), 'primary')));
});

register('licenses', async ({ to = 'profiles' }) => {
  let text = '';
  try { text = await (await fetch('licenses.txt')).text(); } catch { text = T.licensesFail; }
  // Спершу перелік компонентів, повні тексти ліцензій — нижче, у розгортному блоці (розділювач ставить scripts/licenses.py).
  // Текст вставляється як є (createTextNode, без типографської заміни апострофів): юридичні тексти не змінюємо.
  const mark = '======== ПОВНІ ТЕКСТИ ЛІЦЕНЗІЙ ========';
  const at = text.indexOf(mark);
  const list = at < 0 ? text : text.slice(0, at);
  const full = at < 0 ? '' : text.slice(at + mark.length).replace(/^\n+/, '');
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.licensesTitle), iconOnly('back', T.back, () => go('privacy', { to }), 'quiet')),
    h('pre', { class: 'licenses' }, document.createTextNode(list)),
    full ? h('details', { class: 'card' }, h('summary', {}, 'Повні тексти ліцензій'),
      h('pre', { class: 'licenses' }, document.createTextNode(full))) : null);
});

register('privacy', async ({ to = 'profiles' }) => {
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.privacy.title), iconOnly('back', T.back, () => go(to), 'quiet')),
    h('ul', { class: 'prose' }, T.privacy.items.map((t) => h('li', {}, t))),
    h('p', { class: 'hint' }, T.privacy.parents),
    h('div', { class: 'row' }, button(T.licensesTitle, () => go('licenses', { to }))));
});

// ---- Мій тиждень ----
register('week', async () => {
  const w = await call('week');
  const max = Math.max(1, ...w.days.map((d) => d.tasks));
  const bars = h('div', { class: 'weekbars', role: 'img', 'aria-label': T.week.tasks(w.tasks_total, w.solved_total) },
    w.days.map((d) => h('div', { class: `wbar ${d.active ? 'on' : ''}` },
      h('div', { class: 'col' }, (() => { const f = h('i', { class: 'fill' }); f.style.height = `${Math.round((d.tasks / max) * 100)}%`; return f; })()),
      h('span', {}, T.week.days[new Date((d.day + 0) * 86400000).getUTCDay()]))));
  const dots = w.goal ? h('div', { class: 'pager-dots', role: 'img', 'aria-label': T.week.goal(w.active_days, w.goal) },
    Array.from({ length: w.goal }, (_, i) => h('i', { class: i < w.active_days ? 'on' : '' }))) : null;
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, T.week.title), back()),
    h('div', { class: 'card' }, bars, h('p', { class: 'center' }, T.week.tasks(w.tasks_total, w.solved_total)), h('p', { class: 'center muted' }, T.week.active(w.active_days))),
    w.goal ? h('div', { class: 'card' }, h('p', { class: 'center' }, T.week.goal(w.active_days, w.goal)), dots) : null,
    h('h2', {}, T.week.setGoal), h('p', { class: 'hint' }, T.week.goalHint),
    h('div', { class: 'grade-row' }, [0, 1, 2, 3, 4, 5, 6, 7].map((g) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(g === w.goal),
      onclick: guarded(async () => { await call('settings_set', { weekly_goal_days: g }); await go('week'); }) }, g === 0 ? T.week.noGoal : String(g)))));
});

// Оновлення вмісту з підписаного пакета (два файли: пакет і підпис). Застосунок сам перевіряє підпис.
function packsBox(say, fail) {
  const toB64 = async (file) => {
    const buf = new Uint8Array(await file.arrayBuffer());
    let bin = '';
    for (let i = 0; i < buf.length; i++) bin += String.fromCharCode(buf[i]);
    return btoa(bin);
  };
  const data = h('input', { type: 'file', accept: '.json' });
  const sig = h('input', { type: 'file', accept: '.sig' });
  const list = h('div', { 'aria-live': 'polite' });
  call('packs_list').then((r) => list.replaceChildren(h('ul', { class: 'prose' }, r.packs.map((p) =>
    h('li', {}, `${p.id}: версія ${p.version}${p.updated ? ' (оновлено)' : ''}`))))).catch(() => {});
  return h('div', {}, list, field(T.packs.file, data), field(T.packs.sigFile, sig),
    h('div', { class: 'row' }, button(T.packs.install, guarded(async () => {
      const f = data.files && data.files[0];
      const g = sig.files && sig.files[0];
      if (!f || !g) return;
      try {
        await call('pack_install', { data: await toB64(f), sig: await toB64(g) });
        say(T.packs.done);
      } catch (e) { fail(e); }
    }))));
}

// Режим методиста: у відкритому уроці видно правильні відповіді й ключові ідеї (для вичитки й тестування).
