// Kmit Методист: окремий застосунок для вичитки вмісту. Жодних дитячих екранів, ролей чи робіт для друку.
// Ядро (`app-core`, feature `reviewer`) дозволяє лише команди `rv_*`; даних дітей у застосунку немає.
import { call } from './api.js';
import { h, mount, button, iconButton, iconOnly, icon, notice, field, guarded, saveFile, pref, announce } from './kit.js';

// ---------------------------------------------------------------------------------------------------------------
// Маршрутизатор: екрани за назвою, кнопка «Назад» телефону повертає на попередній.
// ---------------------------------------------------------------------------------------------------------------
const routes = new Map();
const stack = [];
const register = (name, fn) => routes.set(name, fn);
// Екрани, доступні без відкритого профілю; решту без входу не показуємо (також після «Назад» із заблокованого стану).
const AUTH_FREE = new Set(['welcome', 'setup', 'recovery', 'unlock', 'recover', 'licenses']);
let unlocked = false;
let navigating = false;

async function show(name, params) {
  if (!unlocked && !AUTH_FREE.has(name)) { await start(); return; }
  const fn = routes.get(name);
  if (!fn) throw new Error(`немає екрана ${name}`);
  await fn(params || {});
}
// Перехід уперед: новий запис в історії. Подвійний дотик не створює дубль.
async function go(name, params = {}) {
  if (navigating) return;
  navigating = true;
  try {
    stack.push({ name, params });
    history.pushState({ n: stack.length }, '');
    await show(name, params);
  } finally { navigating = false; }
}
// Заміна поточного екрана без нового запису: екрани входу, «наступна тема». «Назад» телефону веде туди, де були до цього.
async function replace(name, params = {}) {
  stack.pop();
  stack.push({ name, params });
  history.replaceState({ n: stack.length }, '');
  await show(name, params);
}
window.addEventListener('popstate', async () => {
  stack.pop();
  const t = stack[stack.length - 1];
  if (t) await show(t.name, t.params);
  else await start();
});
const back = (fallback, params) => (stack.length > 1 ? history.back() : replace(fallback, params));

// ---------------------------------------------------------------------------------------------------------------
// Тексти й підписи.
// ---------------------------------------------------------------------------------------------------------------
const STATUS = {
  none: { label: 'Без позначки', icon: 'circle', cls: 'none' },
  ok: { label: 'Усе гаразд', icon: 'check', cls: 'ok' },
  fix: { label: 'Є виправлення', icon: 'pen', cls: 'fix' },
  remove: { label: 'Прибрати', icon: 'cross', cls: 'remove' },
};
const PACK_STATUS = { draft: 'чернетка', reviewed: 'вичитано', approved: 'схвалено' };

function errText(e) {
  switch (e.code) {
    case 'wrong_pin': return 'Не той PIN. Спробуйте ще раз.';
    case 'invalid_pin': return 'PIN має мати від 4 до 32 символів.';
    case 'wrong_recovery_code': return 'Код не підходить.';
    case 'profile_data_lost': return 'Дані профілю пошкоджено. Потрібен код відновлення.';
    case 'invalid_nickname': return 'Напишіть ім’я літерами.';
    case 'pin_locked': {
      const sec = (String(e.message).match(/\d+/) || [])[0];
      return sec ? `Забагато спроб. Зачекайте ${sec} с і спробуйте ще раз.` : 'Забагато спроб. Зачекайте трохи й спробуйте ще раз.';
    }
    case 'no_profile_open': return 'Спершу відкрийте профіль.';
    case 'wrong_profile_kind': return 'Ця дія недоступна в застосунку методиста.';
    default: return e.message || 'Щось пішло не так. Спробуйте ще раз.';
  }
}

function fieldLabel(kind, f) {
  const [name, k] = f.split(':');
  const n = k === undefined ? '' : ` ${Number(k) + 1}`;
  const common = { explanation: 'Пояснення', title: 'Назва' };
  if (kind === 'lesson') {
    const l = { title: 'Назва уроку', text: 'Текст', statement: 'Приклад: умова', result: 'Результат', prompt: 'Питання',
      answer: 'Відповідь', explanation: 'Пояснення відповіді', caption: 'Підпис до схеми', step: 'Крок', option: 'Варіант', note_prompt: 'Запитання для конспекту' };
    return `${l[name] || name}${n}`;
  }
  const t = { prompt: 'Умова', correct: 'Правильна відповідь', wrong: 'Хибна відповідь', accepted: 'Прийнятна відповідь', hint: 'Підказка', ...common };
  return `${t[name] || name}${n}`;
}

function unitTitle(u, n) {
  if (u.kind === 'bank' || u.kind === 'typed') return `Завдання ${n || u.index + 1}${u.kind === 'typed' ? ' (введення слова)' : ''}`;
  if (u.unit === 'lesson:title') return 'Назва уроку';
  if (u.unit === 'lesson:notes') return 'Запитання для конспекту';
  const names = u.fields.map((f) => f.field.split(':')[0]);
  if (names.includes('statement')) return `Приклад (блок ${u.index + 1})`;
  if (names.includes('option')) return `Контрольне питання (блок ${u.index + 1})`;
  if (names.includes('answer')) return `Самоперевірка (блок ${u.index + 1})`;
  if (names.includes('caption')) return `Схема (блок ${u.index + 1})`;
  return `Текст уроку (блок ${u.index + 1})`;
}

const pct = (a, b) => (b ? Math.round((a / b) * 100) : 0);
// Підпис відсотка: «0 %» лише коли справді нічого немає, малий прогрес показуємо як «<1 %».
const pctLabel = (a, b) => (a > 0 && pct(a, b) < 1 ? '<1 %' : `${pct(a, b)} %`);
const word = (n, one, few, many) => {
  const m = n % 100;
  if (m >= 11 && m <= 14) return many;
  const d = n % 10;
  return d === 1 ? one : d >= 2 && d <= 4 ? few : many;
};
const num = (n) => Number(n).toLocaleString('uk-UA');
const texts = (n) => `${num(n)} ${word(n, 'текст', 'тексти', 'текстів')}`;

function statusPill(status) {
  const s = STATUS[status] || STATUS.none;
  return h('span', { class: `pill st-${s.cls}` }, icon(s.icon, 14), h('span', {}, s.label));
}

function progressBar(done, total, label) {
  const p = done > 0 ? Math.max(1, pct(done, total)) : 0;
  // Ширина через CSSOM, а не атрибут style: CSP застосунку забороняє вбудовані стилі.
  const fill = h('span');
  fill.style.width = `${p}%`;
  return h('div', { class: 'bar', role: 'progressbar', 'aria-valuemin': 0, 'aria-valuemax': 100, 'aria-valuenow': p, 'aria-label': label }, fill);
}

const loading = (text = 'Завантаження…') => mount(h('p', { class: 'muted', role: 'status' }, text));

// ---------------------------------------------------------------------------------------------------------------
// Вхід: вітання, створення профілю, PIN, відновлення.
// ---------------------------------------------------------------------------------------------------------------
const brand = () => h('div', { class: 'brand' }, h('span', { class: 'mark' }, icon('book', 26)), h('span', {}, 'Kmit Методист'));

register('welcome', async () => {
  mount(
    h('div', { class: 'welcome' },
      brand(),
      h('h1', {}, 'Вичитка навчальних матеріалів'),
      h('p', { class: 'lead' }, 'Прочитайте теми, відзначте, що гаразд, а що виправити, і надішліть один файл розробникові.'),
      h('ol', { class: 'steps' },
        h('li', {}, h('strong', {}, 'Читайте. '), 'Уроки й завдання зібрані за класами й предметами.'),
        h('li', {}, h('strong', {}, 'Позначайте. '), '«Усе гаразд», виправлений текст або «прибрати завдання».'),
        h('li', {}, h('strong', {}, 'Надсилайте. '), 'Застосунок збереже файл, а ви перешлете його поштою чи месенджером.')),
      h('div', { class: 'facts' },
        h('p', {}, h('strong', {}, 'Приватність. '), 'Тут немає даних дітей. Застосунок працює без інтернету й нічого не надсилає сам. Ваші позначки лежать лише на цьому пристрої в зашифрованому вигляді.')),
      button('Почати', () => replace('setup'), 'primary big'),
      button('Ліцензії та подяки', () => go('licenses'), 'quiet')));
});

register('setup', async () => {
  const name = h('input', { type: 'text', maxlength: 24, autocomplete: 'name' });
  const pin1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const pin2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div', { 'aria-live': 'polite' });
  const submit = guarded(async () => {
    if (!name.value.trim()) { msg.replaceChildren(notice('Напишіть ім’я: воно потрібне для назви файлу.', 'error')); name.focus(); return; }
    if (pin1.value !== pin2.value) { msg.replaceChildren(notice('PIN у двох полях різний.', 'error')); return; }
    try {
      const r = await call('create_profile', { kind: 'teacher', nickname: name.value, pin: pin1.value });
      unlocked = true;
      await replace('recovery', { code: r.recovery_code });
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  const form = h('form', { onsubmit: (ev) => { ev.preventDefault(); submit({ currentTarget: null }); } },
    field('Ім’я та прізвище', name, 'Лише для назви файлу з вичиткою. Нікуди не надсилається.'),
    field('PIN (від 4 символів)', pin1, 'Захищає ваші позначки на цьому пристрої.'),
    field('PIN ще раз', pin2), msg,
    h('div', { class: 'row' }, button('Створити профіль', submit, 'primary'), button('Назад', () => replace('welcome'))));
  mount(h('h1', {}, 'Профіль методиста'), h('p', { class: 'muted' }, 'Профіль потрібен лише для ваших позначок. Дані дітей у застосунок не потрапляють.'), form);
  name.focus();
});

register('recovery', async ({ code }) => {
  const done = h('input', { type: 'checkbox' });
  const next = button('Далі', () => replace('home'), 'primary', { disabled: true });
  done.addEventListener('change', () => { next.disabled = !done.checked; });
  const msg = h('div', { 'aria-live': 'polite' });
  const copy = button('Скопіювати код', async () => {
    try { await navigator.clipboard.writeText(code); msg.replaceChildren(notice('Код скопійовано.', 'good')); }
    catch { msg.replaceChildren(notice('Не вдалося скопіювати. Перепишіть код вручну.', 'error')); }
  });
  mount(
    h('h1', {}, 'Код відновлення'),
    h('p', {}, 'Якщо забудете PIN, цей код поверне доступ до ваших позначок. Показується один раз: збережіть його в надійному місці.'),
    h('p', { class: 'code', 'aria-label': `Код відновлення: ${code}` }, code), copy, msg,
    h('label', { class: 'check' }, done, h('span', {}, 'Код збережено')),
    next);
});

register('unlock', async ({ slot }) => {
  const pin = h('input', { type: 'password', autocomplete: 'current-password' });
  const msg = h('div', { 'aria-live': 'polite' });
  const submit = guarded(async () => {
    try {
      await call('open_profile', { slot, pin: pin.value });
      unlocked = true;
      await replace('home');
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); pin.select(); }
  });
  pin.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') submit({ currentTarget: null }); });
  mount(
    brand(), h('h1', {}, 'Введіть PIN'), field('PIN', pin), msg,
    h('div', { class: 'row' }, button('Відкрити', submit, 'primary')),
    button('Забули PIN?', () => replace('recover', { slot }), 'quiet'));
  pin.focus();
});

register('recover', async ({ slot }) => {
  const code = h('input', { type: 'text', autocomplete: 'off', autocapitalize: 'characters', spellcheck: 'false' });
  const pin1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const pin2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div', { 'aria-live': 'polite' });
  const submit = guarded(async () => {
    if (pin1.value !== pin2.value) { msg.replaceChildren(notice('PIN у двох полях різний.', 'error')); return; }
    try {
      await call('recover_profile', { slot, code: code.value, new_pin: pin1.value });
      unlocked = true;
      await replace('home');
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('h1', {}, 'Відновлення доступу'),
    h('p', { class: 'muted' }, 'Введіть код відновлення, який ви зберегли під час створення профілю, і придумайте новий PIN.'),
    field('Код відновлення', code), field('Новий PIN (від 4 символів)', pin1), field('Новий PIN ще раз', pin2), msg,
    h('div', { class: 'row' }, button('Відновити', submit, 'primary'), button('Назад', () => replace('unlock', { slot }))));
  code.focus();
});

// ---------------------------------------------------------------------------------------------------------------
// Головний екран: прогрес, продовження, фільтри, предмети за класами.
// ---------------------------------------------------------------------------------------------------------------
const topTools = () => h('div', { class: 'tools' },
  iconOnly('gear', 'Налаштування', () => go('settings'), 'quiet'),
  iconOnly('lock', 'Заблокувати', lock, 'quiet'));

async function lock() {
  await call('lock');
  unlocked = false;
  await start();
}

const allTopics = (ov) => ov.packs.flatMap((p) => p.topics.map((t) => ({ ...t, pack: p })));

function resumeTarget(ov) {
  const topics = allTopics(ov);
  const last = pref.get('last');
  const t = topics.find((x) => x.id === last && x.done < x.total) || topics.find((x) => x.done < x.total);
  return t || null;
}

register('home', async () => {
  loading();
  const ov = await call('rv_overview');
  const tot = ov.totals;
  const ok = tot.done - tot.fix - tot.remove;
  const grades = [...new Set(ov.packs.map((p) => p.grade))].sort((a, b) => a - b);
  let grade = Number(pref.get('grade', 0));
  if (grade && !grades.includes(grade)) grade = 0;
  let onlyOpen = pref.get('open', '0') === '1';
  const list = h('div', { class: 'groups' });
  const renderList = () => {
    const packs = ov.packs.filter((p) => (!grade || p.grade === grade)
      && (!onlyOpen || p.topics.some((t) => t.done < t.total)));
    if (!packs.length) { list.replaceChildren(h('p', { class: 'muted' }, 'Нічого не знайдено: усе перевірено.')); return; }
    const byGrade = new Map();
    packs.forEach((p) => { if (!byGrade.has(p.grade)) byGrade.set(p.grade, []); byGrade.get(p.grade).push(p); });
    list.replaceChildren(...[...byGrade.entries()].map(([g, ps]) => h('section', { class: 'group', 'aria-label': `${g} клас` },
      h('h2', {}, `${g} клас`),
      h('ul', { class: 'cards' }, ps.map((p) => {
        const total = p.topics.reduce((a, t) => a + t.total, 0);
        const done = p.topics.reduce((a, t) => a + t.done, 0);
        const marked = p.topics.reduce((a, t) => a + t.fix + t.remove, 0);
        return h('li', {}, h('button', { type: 'button', class: `card subj s-${p.subject}`, onclick: () => go('subject', { pack: p.pack }) },
          h('span', { class: 'cardtop' }, h('span', { class: 'ctitle' }, p.title), marked ? h('span', { class: 'pill st-fix' }, `Зауважень: ${marked}`) : null),
          h('span', { class: 'meta' }, `${p.topics.length} тем · перевірено ${pctLabel(done, total)}`),
          progressBar(done, total, `${p.title}, ${g} клас: перевірено ${pctLabel(done, total)}`)));
      })))));
  };
  const gradeRow = h('div', { class: 'chips', role: 'group', 'aria-label': 'Клас' },
    [0, ...grades].map((g) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(grade === g), onclick: (ev) => {
      grade = g; pref.set('grade', g);
      gradeRow.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
      ev.currentTarget.setAttribute('aria-pressed', 'true');
      renderList();
    } }, g ? `${g} клас` : 'Усі класи')));
  const openCb = h('input', { type: 'checkbox', checked: onlyOpen });
  openCb.addEventListener('change', () => { onlyOpen = openCb.checked; pref.set('open', onlyOpen ? '1' : '0'); renderList(); });
  const next = resumeTarget(ov);
  mount(
    h('div', { class: 'topbar' }, h('h1', {}, 'Вичитка'), topTools()),
    h('section', { class: 'summary', 'aria-label': 'Підсумок' },
      h('p', { class: 'big-num' }, pctLabel(tot.done, tot.total)),
      h('p', {}, `Перевірено ${num(tot.done)} із ${texts(tot.total)}`),
      progressBar(tot.done, tot.total, `Перевірено ${pctLabel(tot.done, tot.total)}`),
      h('p', { class: 'tally' },
        h('span', { class: 'pill st-ok' }, icon('check', 14), `Усе гаразд: ${ok}`),
        h('span', { class: 'pill st-fix' }, icon('pen', 14), `Виправити: ${tot.fix}`),
        h('span', { class: 'pill st-remove' }, icon('cross', 14), `Прибрати: ${tot.remove}`)),
      h('div', { class: 'row' },
        next ? button(`Продовжити: ${next.title}`, () => go('topic', { skill: next.id }), 'primary') : null,
        tot.notes ? iconButton('send', 'Надіслати файл', () => go('send'), next ? '' : 'primary') : null)),
    tot.stale ? notice(`Позначок, що застаріли після оновлення вмісту: ${tot.stale}. Вони не потраплять у файл.`, 'info') : null,
    gradeRow,
    h('label', { class: 'check' }, openCb, h('span', {}, 'Лише ті, де є що перевіряти')),
    list);
  renderList();
});

register('subject', async ({ pack }) => {
  loading();
  const ov = await call('rv_overview');
  const p = ov.packs.find((x) => x.pack === pack);
  if (!p) { await replace('home'); return; }
  const total = p.topics.reduce((a, t) => a + t.total, 0);
  const done = p.topics.reduce((a, t) => a + t.done, 0);
  mount(
    h('div', { class: 'topbar' }, iconOnly('back', 'Назад', () => back('home'), 'quiet'), h('h1', {}, `${p.title}, ${p.grade} клас`), topTools()),
    h('p', { class: 'muted' }, `Версія пакета ${p.version} · перевірено ${num(done)} із ${texts(total)}`),
    h('ul', { class: 'cards' }, p.topics.map((t) => h('li', {}, h('button', { type: 'button', class: 'card topic', onclick: () => go('topic', { skill: t.id }) },
      h('span', { class: 'cardtop' }, h('span', { class: 'ctitle' }, t.title), t.done >= t.total && t.total ? h('span', { class: `pill ${t.fix || t.remove ? 'st-fix' : 'st-ok'}` }, icon(t.fix || t.remove ? 'pen' : 'check', 14), t.fix || t.remove ? 'Є зауваження' : 'Усе перевірено') : null),
      h('span', { class: 'meta' },
        `${t.done} із ${t.total}`,
        t.fix ? ` · виправити ${t.fix}` : '', t.remove ? ` · прибрати ${t.remove}` : '',
        ` · ${PACK_STATUS[t.status] || t.status}`),
      progressBar(t.done, t.total, `${t.title}: ${t.done} із ${t.total}`))))));
});

// ---------------------------------------------------------------------------------------------------------------
// Тема: урок, завдання по одному, підсумок.
// ---------------------------------------------------------------------------------------------------------------
register('topic', async ({ skill, view = 'tasks', at = null }) => {
  loading();
  pref.set('last', skill);
  const ov = await call('rv_overview');
  const order = allTopics(ov);
  const meta = order.find((x) => x.id === skill);
  let t = await call('rv_topic', { skill });
  let mode = view;
  const isTask = (u) => u.kind !== 'lesson';
  const firstOpen = () => Math.max(0, t.units.filter(isTask).findIndex((u) => u.status === 'none'));
  let idx = at === null ? firstOpen() : at; // «Продовжити» відкриває перше завдання без позначки
  const drafts = new Map(); // незбережені виправлення: не губляться при переході між завданнями
  const body = h('div', { class: 'tbody' });
  const head = h('div', { class: 'thead' });
  const tabs = h('div', { class: 'tabs', role: 'group', 'aria-label': 'Розділи теми' });
  const lessonUnits = () => t.units.filter((u) => !isTask(u));
  const taskUnits = () => t.units.filter(isTask);
  const counts = () => {
    const c = { none: 0, ok: 0, fix: 0, remove: 0 };
    t.units.forEach((u) => { c[u.status] += 1; });
    return c;
  };
  const reload = async () => { t = await call('rv_topic', { skill }); };

  const renderHead = () => {
    const c = counts();
    const done = t.units.length - c.none;
    head.replaceChildren(
      progressBar(done, t.units.length, `Перевірено ${done} із ${t.units.length}`),
      h('p', { class: 'meta' }, `Перевірено ${done} із ${t.units.length}`, c.fix ? ` · виправити ${c.fix}` : '', c.remove ? ` · прибрати ${c.remove}` : ''));
  };
  const setMode = (m) => { mode = m; draw(); };
  const renderTabs = () => {
    const defs = [['lesson', `Урок (${lessonUnits().length})`], ['tasks', `Завдання (${taskUnits().length})`], ['summary', 'Підсумок']];
    tabs.replaceChildren(...defs.map(([m, label]) => h('button', { type: 'button', class: 'tab', 'aria-pressed': String(mode === m), onclick: () => setMode(m) }, label)));
  };

  // Після зміни перечитуємо тему й малюємо той самий розділ. `advance` відкриває наступне завдання, якщо користувач
  // за цей час не пішов далі сам (`from` це індекс, з якого натиснули).
  const changed = async (advance, from) => {
    await reload();
    let moved = false;
    if (advance && mode === 'tasks' && idx === from) {
      moved = true;
      if (idx < taskUnits().length - 1) { idx += 1; announce(`Збережено. Завдання ${idx + 1} із ${taskUnits().length}`); } else { mode = 'summary'; announce('Збережено. Це було останнє завдання'); }
    } else announce('Збережено');
    draw(moved);
  };

  function draw(top = true) {
    renderHead();
    renderTabs();
    if (mode === 'lesson') {
      const us = lessonUnits();
      body.replaceChildren(us.length ? h('div', { class: 'stack' }, us.map((u) => unitCard(skill, u, changed, { drafts }))) : h('p', { class: 'muted' }, 'Для цієї теми ще немає уроку.'));
    } else if (mode === 'tasks') {
      const us = taskUnits();
      if (!us.length) { body.replaceChildren(h('p', { class: 'muted' }, 'У цій темі ще немає завдань.')); }
      else {
        idx = Math.min(Math.max(idx, 0), us.length - 1);
        const grid = h('details', { class: 'allgrid' }, h('summary', {}, 'Усі завдання'),
          h('div', { class: 'grid' }, us.map((u, i) => h('button', { type: 'button', class: `dot st-${STATUS[u.status].cls}`, 'aria-label': `Завдання ${i + 1}: ${STATUS[u.status].label}`, 'aria-current': i === idx ? 'true' : null,
            onclick: () => { idx = i; draw(); } }, String(i + 1)))));
        body.replaceChildren(
          h('div', { class: 'stepper' },
            button([icon('back', 18), 'Попереднє'], () => { idx -= 1; draw(); }, 'quiet', { disabled: idx === 0 }),
            h('span', { class: 'count', 'aria-live': 'polite' }, `${idx + 1} із ${us.length}`),
            button(['Наступне', icon('next', 18)], () => { if (idx < us.length - 1) idx += 1; else mode = 'summary'; draw(); }, 'quiet')),
          unitCard(skill, us[idx], changed, { drafts, advance: true, number: idx + 1, at: idx }),
          grid);
      }
    } else {
      body.replaceChildren(summaryView());
    }
    if (top) {
      window.scrollTo(0, 0);
      const heading = body.querySelector('h2');
      if (heading) { heading.setAttribute('tabindex', '-1'); heading.focus({ preventScroll: true }); }
    }
  }

  function summaryView() {
    const c = counts();
    const pos = order.findIndex((x) => x.id === skill);
    const nextT = pos >= 0 ? order[pos + 1] : null;
    const msg = h('div', { 'aria-live': 'polite' });
    let armed = false;
    let timer = null;
    const label = `Решту без позначки (${c.none}) вважати «усе гаразд»`;
    const bulk = button(label, guarded(async (ev) => {
      const b = ev.currentTarget;
      if (!armed) {
        armed = true; b.textContent = `Підтвердити: ${c.none} без позначки стануть «усе гаразд»`; announce('Натисніть ще раз, щоб підтвердити');
        timer = setTimeout(() => { armed = false; if (b.isConnected) b.textContent = label; }, 6000);
        return;
      }
      clearTimeout(timer);
      try { await call('rv_topic_ok', { skill }); await reload(); draw(false); announce('Готово'); } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
    }), 'quiet', { disabled: c.none === 0 });
    return h('div', { class: 'stack' },
      h('section', { class: 'summary' },
        h('h2', {}, 'Підсумок теми'),
        h('p', { class: 'tally' },
          h('span', { class: 'pill st-ok' }, icon('check', 14), `Усе гаразд: ${c.ok}`),
          h('span', { class: 'pill st-fix' }, icon('pen', 14), `Виправити: ${c.fix}`),
          h('span', { class: 'pill st-remove' }, icon('cross', 14), `Прибрати: ${c.remove}`),
          h('span', { class: 'pill st-none' }, icon('circle', 14), `Без позначки: ${c.none}`)),
        c.none ? h('p', { class: 'muted' }, 'Тексти без позначки не потраплять до розробника як перевірені. Якщо ви їх прочитали й зауважень немає, позначте всі разом.') : h('p', {}, 'Усю тему перевірено.'),
        bulk, msg),
      h('div', { class: 'row' },
        iconButton('send', 'Надіслати файл цієї теми', guarded(async () => {
          try { await sendFile(skill, msg); } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
        }), '', { disabled: t.units.every((u) => u.status === 'none') }),
        nextT ? button(`Наступна тема: ${nextT.title}`, () => replace('topic', { skill: nextT.id }), 'primary') : null,
        meta ? button('До списку тем', () => replace('subject', { pack: meta.pack.pack })) : null));
  }

  mount(
    h('div', { class: 'topbar' }, iconOnly('back', 'Назад', () => back('subject', { pack: meta ? meta.pack.pack : '' }), 'quiet'),
      h('h1', {}, t.title || skill), topTools()),
    h('p', { class: 'meta' }, meta ? `${meta.pack.title}, ${meta.pack.grade} клас · ${PACK_STATUS[t.review] || t.review}` : ''),
    head, tabs, body);
  draw(false);
});

// Картка однієї одиниці: тексти за полями, виправлення, «усе гаразд», «прибрати».
// `opts`: drafts (Map незбережених виправлень), advance (перейти до наступного завдання після позначки), number, at.
function unitCard(skill, u, changed, opts = {}) {
  const { drafts, advance = false, number = 0, at = 0 } = opts;
  const msg = h('div', { 'aria-live': 'polite' });
  const card = h('article', { class: `unit st-${STATUS[u.status].cls}`, 'aria-label': unitTitle(u, number) });
  const act = (fn, adv = false) => guarded(async () => {
    try { await fn(); await changed(adv, at); } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  const star = u.unit === 'lesson:title' ? '*:title' : u.unit === 'lesson:notes' ? '*:notes' : '*';
  const base = { skill, kind: u.kind, index: u.index };

  const fieldRows = u.fields.map((f) => {
    const note = f.note && f.note.status === 'fix' ? f.note : null;
    const key = `${u.unit}|${f.field}`;
    const draft = drafts && drafts.get(key);
    const editor = h('div', { class: 'editor', hidden: !draft });
    const ta = h('textarea', { rows: 4, maxlength: 2000, 'aria-label': `Виправлений текст: ${fieldLabel(u.kind, f.field)}` });
    ta.value = draft ? draft.text : (note ? note.correction : f.text);
    const comm = h('input', { type: 'text', maxlength: 500, placeholder: 'Коментар (необов’язково)', 'aria-label': 'Коментар', value: draft ? draft.comment : (note ? note.comment : '') });
    const remember = () => { if (drafts) drafts.set(key, { text: ta.value, comment: comm.value }); };
    ta.addEventListener('input', remember);
    comm.addEventListener('input', remember);
    const edMsg = h('div', { 'aria-live': 'polite' });
    editor.append(ta, comm, edMsg, h('div', { class: 'row' },
      button('Зберегти виправлення', guarded(async () => {
        if (!ta.value.trim() || ta.value.trim() === f.text.trim()) { edMsg.replaceChildren(notice('Змініть текст або натисніть «Скасувати».', 'error')); return; }
        try {
          await call('rv_note', { ...base, field: f.field, status: 'fix', correction: ta.value, comment: comm.value });
          if (drafts) drafts.delete(key);
          await changed(false, at);
        } catch (e) { edMsg.replaceChildren(notice(errText(e), 'error')); }
      }), 'primary'),
      button('Скасувати', () => { if (drafts) drafts.delete(key); editor.hidden = true; opener.focus(); }, 'quiet')));
    const opener = button([icon('pen', 16), h('span', {}, note ? 'Змінити виправлення' : 'Виправити')], () => {
      editor.hidden = !editor.hidden; if (!editor.hidden) ta.focus();
    }, 'small quiet', { 'aria-label': `${note ? 'Змінити виправлення' : 'Виправити'}: ${fieldLabel(u.kind, f.field)}` });
    const isCorrect = f.field === 'correct';
    return h('div', { class: `frow ${note ? 'has-fix' : ''}` },
      h('div', { class: `flabel ${isCorrect ? 'right' : ''}` }, fieldLabel(u.kind, f.field)),
      h('p', { class: `ftext ${note ? 'struck' : ''}` }, f.text),
      note ? h('div', { class: 'fix' }, h('div', { class: 'flabel' }, 'Ваше виправлення'), h('p', {}, note.correction), note.comment ? h('p', { class: 'muted' }, note.comment) : null,
        button('Скасувати виправлення', act(() => call('rv_clear', { ...base, field: f.field })), 'small quiet')) : null,
      opener, editor);
  });

  const actions = h('div', { class: 'actions' });
  if (u.status === 'remove') {
    actions.append(h('p', { class: 'state remove' }, icon('cross', 16), 'Це завдання буде прибране.'),
      button('Скасувати позначку', act(() => call('rv_clear', { ...base, field: 'remove' })), 'quiet'));
  } else {
    if (u.status === 'ok') {
      actions.append(h('p', { class: 'state ok' }, icon('check', 16), 'Позначено: усе гаразд.'),
        button('Скасувати позначку', act(() => call('rv_clear', { ...base, field: star })), 'quiet'));
    } else if (u.status === 'fix' && u.ok_mark) {
      actions.append(h('p', { class: 'state ok' }, icon('check', 16), 'Решту позначено: гаразд.'),
        button('Скасувати «решта гаразд»', act(() => call('rv_clear', { ...base, field: star })), 'quiet'));
    } else {
      actions.append(button([icon('check', 18), h('span', {}, u.status === 'fix' ? 'Решта гаразд' : 'Усе гаразд')], act(() => call('rv_note', { ...base, field: star, status: 'ok' }), advance), 'primary'));
    }
    if (u.kind === 'bank' || u.kind === 'typed') {
      const why = h('input', { type: 'text', maxlength: 500, placeholder: 'Чому прибрати? (необов’язково)', 'aria-label': 'Причина' });
      const box = h('div', { class: 'confirm', hidden: true }, why,
        h('div', { class: 'row' },
          button('Так, прибрати завдання', act(() => call('rv_note', { ...base, field: 'prompt', status: 'remove', comment: why.value }), advance), 'danger'),
          button('Ні', () => { box.hidden = true; }, 'quiet')));
      actions.append(button([icon('cross', 18), h('span', {}, 'Прибрати завдання')], () => { box.hidden = !box.hidden; if (!box.hidden) why.focus(); }, 'quiet danger-text'), box);
    }
  }
  card.append(h('header', { class: 'uhead' }, h('h2', { class: 'utitle' }, unitTitle(u, number)), statusPill(u.status)), ...fieldRows, actions, msg);
  return card;
}

// ---------------------------------------------------------------------------------------------------------------
// Надсилання файлу.
// ---------------------------------------------------------------------------------------------------------------
async function sendFile(skill, msgBox) {
  const r = await call('rv_export', skill ? { skill } : {});
  if (await saveFile(r.data, r.filename, 'text/csv')) {
    msgBox.replaceChildren(notice(`Файл збережено: ${r.filename}. Перешліть його розробникові поштою чи месенджером.`, 'good'));
  } else {
    msgBox.replaceChildren(notice('Файл не збережено.', 'error'));
  }
}

register('send', async () => {
  loading();
  const ov = await call('rv_overview');
  const tot = ov.totals;
  const ok = tot.done - tot.fix - tot.remove;
  const msg = h('div', { 'aria-live': 'polite' });
  const touched = allTopics(ov).filter((t) => t.done > 0);
  mount(
    h('div', { class: 'topbar' }, iconOnly('back', 'Назад', () => back('home'), 'quiet'), h('h1', {}, 'Надіслати вичитку'), topTools()),
    tot.notes === 0
      ? h('p', { class: 'muted' }, 'Позначок ще немає. Відкрийте тему, прочитайте її й позначте тексти, тоді тут з’явиться файл.')
      : h('div', { class: 'stack' },
        h('section', { class: 'summary' },
          h('h2', {}, 'Що буде у файлі'),
          h('p', { class: 'tally' },
            h('span', { class: 'pill st-ok' }, icon('check', 14), `Усе гаразд: ${ok}`),
            h('span', { class: 'pill st-fix' }, icon('pen', 14), `Виправити: ${tot.fix}`),
            h('span', { class: 'pill st-remove' }, icon('cross', 14), `Прибрати: ${tot.remove}`)),
          h('p', { class: 'muted' }, `Тем із позначками: ${touched.length}. Тексти без позначки у файл не потраплять: розробник вважатиме їх неперевіреними.`)),
        button('Зберегти файл (CSV)', guarded(async () => { try { await sendFile(null, msg); } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); } }), 'primary big'),
        msg,
        h('ol', { class: 'steps' },
          h('li', {}, 'Збережіть файл на пристрої.'),
          h('li', {}, 'Перешліть його розробникові поштою чи месенджером.'),
          h('li', {}, 'Можна продовжити вичитку й надіслати новий файл пізніше: він міститиме всі ваші позначки.'))));
});

// ---------------------------------------------------------------------------------------------------------------
// Налаштування: розмір тексту, тема, код відновлення, видалення профілю.
// ---------------------------------------------------------------------------------------------------------------
const SCALES = [90, 100, 115, 130, 150];
function applyPrefs() {
  const sc = Number(pref.get('scale', 100));
  document.documentElement.style.setProperty('--scale', String((SCALES.includes(sc) ? sc : 100) / 100));
  const th = pref.get('theme', 'auto');
  if (th === 'light' || th === 'dark') document.documentElement.setAttribute('data-theme', th);
  else document.documentElement.removeAttribute('data-theme');
}

register('settings', async () => {
  const about = await call('about');
  const msg = h('div', { 'aria-live': 'polite' });
  const sizeRow = h('div', { class: 'chips', role: 'group', 'aria-label': 'Розмір тексту' }, SCALES.map((s) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(Number(pref.get('scale', 100)) === s),
    onclick: (ev) => { pref.set('scale', s); applyPrefs(); sizeRow.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false')); ev.currentTarget.setAttribute('aria-pressed', 'true'); } }, `${s} %`)));
  const themeRow = h('div', { class: 'chips', role: 'group', 'aria-label': 'Тема' }, [['auto', 'Авто'], ['light', 'Світла'], ['dark', 'Темна']].map(([v, l]) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(pref.get('theme', 'auto') === v),
    onclick: (ev) => { pref.set('theme', v); applyPrefs(); themeRow.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false')); ev.currentTarget.setAttribute('aria-pressed', 'true'); } }, l)));
  const erase = h('div', { class: 'confirm', hidden: true });
  const word_ = h('input', { type: 'text', autocomplete: 'off', 'aria-label': 'Слово для підтвердження' });
  erase.append(h('p', {}, 'Усі ваші позначки й профіль буде видалено з цього пристрою. Якщо ви ще не надіслали файл, позначки зникнуть. Для підтвердження напишіть слово «видалити».'), word_,
    h('div', { class: 'row' }, button('Видалити назавжди', guarded(async () => {
      if (word_.value.trim().toLowerCase() !== 'видалити') { msg.replaceChildren(notice('Напишіть слово «видалити».', 'error')); return; }
      try {
        const { profiles } = await call('profiles');
        for (const p of profiles) await call('erase_profile', { slot: p.slot });
        pref.clear();
        unlocked = false;
        await replace('welcome');
      } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
    }), 'danger'), button('Скасувати', () => { erase.hidden = true; }, 'quiet')));
  mount(
    h('div', { class: 'topbar' }, iconOnly('back', 'Назад', () => back('home'), 'quiet'), h('h1', {}, 'Налаштування')),
    h('h2', {}, 'Розмір тексту'), sizeRow,
    h('h2', {}, 'Тема'), themeRow,
    h('h2', {}, 'Захист'),
    button('Показати новий код відновлення', guarded(async () => {
      try { const r = await call('recovery_new'); msg.replaceChildren(notice('Новий код відновлення (старий більше не діє):', 'info'), h('p', { class: 'code' }, r.recovery_code)); }
      catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
    })),
    msg,
    h('h2', {}, 'Видалення'),
    button('Видалити профіль і позначки', () => { erase.hidden = !erase.hidden; if (!erase.hidden) word_.focus(); }, 'danger-text quiet'), erase,
    h('h2', {}, 'Про застосунок'),
    h('p', { class: 'muted' }, `Kmit Методист, версія ${about.version}. Застосунок працює без інтернету, не збирає даних і не передає їх. Позначки лежать на цьому пристрої в зашифрованому вигляді.`),
    button('Ліцензії та подяки', () => go('licenses'), 'quiet'));
});

// ---------------------------------------------------------------------------------------------------------------
// Ліцензії та подяки: той самий licenses.txt, що й у дитячому застосунку (копіює scripts/build-reviewer-ui.sh).
// Спершу перелік компонентів, повні тексти — у розгортному блоці; текст вставляється як є, без типографських замін.
// ---------------------------------------------------------------------------------------------------------------
register('licenses', async () => {
  let text = '';
  try {
    const r = await fetch('licenses.txt');
    if (!r.ok) throw new Error(String(r.status));
    text = await r.text();
  } catch { text = 'Не вдалося прочитати список ліцензій.'; }
  const mark = '======== ПОВНІ ТЕКСТИ ЛІЦЕНЗІЙ ========';
  const at = text.indexOf(mark);
  const list = at < 0 ? text : text.slice(0, at);
  const full = at < 0 ? '' : text.slice(at + mark.length).replace(/^\n+/, '');
  mount(
    h('div', { class: 'topbar' }, iconOnly('back', 'Назад', () => back(unlocked ? 'settings' : 'welcome'), 'quiet'), h('h1', {}, 'Ліцензії та подяки')),
    h('pre', { class: 'licenses' }, document.createTextNode(list)),
    full ? h('details', { class: 'licenses-full' }, h('summary', {}, 'Повні тексти ліцензій'),
      h('pre', { class: 'licenses' }, document.createTextNode(full))) : null);
});

// ---------------------------------------------------------------------------------------------------------------
// Запуск.
// ---------------------------------------------------------------------------------------------------------------
window.addEventListener('unhandledrejection', (ev) => {
  console.error(ev.reason);
  mount(h('h1', {}, 'Щось пішло не так'), h('p', { class: 'notice error', role: 'alert' }, 'Спробуйте ще раз. Ваші позначки збережено.'),
    button('На початок', () => start(), 'primary'));
});

async function start() {
  applyPrefs();
  // Знімки екрана дозволені: у застосунку немає даних дітей, а методисту треба показати проблему.
  try { if (window.__TAURI__) await window.__TAURI__.core.invoke('set_secure', { on: false }); } catch { /* не критично */ }
  const { profiles } = await call('profiles');
  if (profiles.length) await replace('unlock', { slot: profiles[0].slot });
  else await replace('welcome');
}
start();
