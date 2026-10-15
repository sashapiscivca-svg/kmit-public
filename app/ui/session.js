import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, guarded, speakButton, speechOn, icon, sticker, iconButton } from './ui.js';
import { skillIconName, figureSvg, speakable, creature } from './art.js';
import { register, go } from './router.js';

export const BADGE_ART = { lesson: 'lesson', note: 'note', family: 'family', course: 'course', gap: 'mastery', recall: 'mastery' };

function stickerBadge(s) {
  return h('div', { class: 'sticker earned pop' },
    creature(s.skill, s.kind, s.subject, true, 72),
    h('strong', {}, T.kinds[s.kind] || s.kind), h('span', {}, s.skill_title));
}

register('session', async ({ start, skill, subject, testout }) => {
  if (start) shown = { number: -1, list: [] };
  const r = start === 'practice'
    ? await call('practice_start', { skill })
    : start === 'testout'
      ? await call('testout_start', { skill })
      : start === 'review'
        ? await call('review_start', subject ? { subject } : {})
      : await call(start ? 'session_start' : 'session_state', subject ? { subject } : {});
  if (r.denied) { await go('home'); return; }
  await render(r.model);
});

// Якщо екран уже змінився (двічі натиснуто), просто показуємо актуальний стан.
async function act(name, args) {
  try {
    return await call(name, args);
  } catch (e) {
    if (e.code === 'wrong_phase') return call('session_state');
    throw e;
  }
}

// Показані підказки лишаються на екрані, поки триває та сама задача.
let shown = { number: -1, list: [] };
// Остання оцінка впевненості (за бажанням учня): для м'якого коментаря після відповіді.
let lastConfidence = null;

const askConfidence = () => document.documentElement.dataset.confidence === '1';

async function render(model, flash) {
  switch (model.phase) {
    case 'intro': return renderIntro(model);
    case 'task': return renderTask(model, flash);
    case 'analog': return renderAnalog(model, flash);
    case 'feedback': return renderFeedback(model);
    case 'ended': return renderEnded(model);
    default: await go('home');
  }
}

// Те, що зачитує екранна читалка: дроби й знаки словами; видимий текст не змінюється.
function spoken(text, extraClass = '') {
  return [h('span', { 'aria-hidden': 'true', class: extraClass }, text), h('span', { class: 'sr-only' }, speakable(text))];
}

// Повідомлення про проблему із завданням: коротка панель із трьома причинами.
function flagBlock() {
  const out = h('div', { 'aria-live': 'polite' });
  const panel = h('div', { class: 'card', hidden: true },
    h('h3', { class: 'tight' }, T.flag.title),
    h('div', { class: 'chips' }, [['wrong_answer', T.flag.wrong], ['unclear', T.flag.unclear], ['other', T.flag.other]].map(([k, label]) =>
      h('button', { type: 'button', class: 'btn chip', onclick: guarded(async () => {
        await call('flag_item', { reason: k });
        panel.hidden = true;
        out.replaceChildren(notice(T.flag.thanks, 'good'));
      }) }, label))));
  return h('div', {}, h('button', { type: 'button', class: 'btn quiet', onclick: () => { panel.hidden = !panel.hidden; } }, T.flag.open), panel, out);
}

function endButton() {
  return button(T.task.end, guarded(async () => { const r = await act('end_session'); await render(r.model); }), 'quiet');
}

// Дія, що потребує другого дотику («Точно?»): випадковий дотик нічого не робить.
function confirmButton(label, confirmLabel, onconfirm, cls = '') {
  let armed = false;
  const b = button(label, () => {
    if (!armed) {
      armed = true;
      b.textContent = confirmLabel;
      b.classList.add('danger');
      setTimeout(() => { if (b.isConnected) { armed = false; b.textContent = label; b.classList.remove('danger'); } }, 4000);
      return;
    }
    onconfirm();
  }, cls);
  return b;
}

function figuresNode(list) {
  const nodes = (list || []).map(figureSvg).filter(Boolean);
  return nodes.length ? h('div', { class: 'figs' }, nodes) : null;
}

function answerBox(onSubmit, { keypad = true } = {}) {
  // На телефоні замість системної клавіатури (вона закриває пів екрана й не має «/») показуємо власну цифрову.
  const touch = keypad && !!(window.matchMedia && window.matchMedia('(pointer: coarse)').matches);
  const input = h('input', { type: 'text', class: 'answer', inputmode: touch ? 'none' : 'text', autocomplete: 'off',
    autocapitalize: 'off', autocorrect: 'off', spellcheck: 'false', maxlength: 40, 'aria-label': T.task.answerLabel, 'aria-describedby': 'ah' });
  input.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { ev.preventDefault(); onSubmit(); } });
  // Цифра замінює виділене (після хибної відповіді поле виділено), інакше дописується в кінець.
  const put = (t) => {
    const len = input.value.length;
    const a = Math.min(input.selectionStart ?? len, len);
    const b = Math.min(input.selectionEnd ?? len, len);
    if (a === 0 && b === 0 && len > 0) input.setRangeText(t, len, len, 'end');
    else input.setRangeText(t, a, b, 'end');
  };
  const erase = () => {
    const len = input.value.length;
    const a = input.selectionStart ?? len;
    const b = input.selectionEnd ?? len;
    if (a !== b) input.setRangeText('', a, b, 'end');
    else if (a === 0 && len > 0) input.value = input.value.slice(0, -1);
    else if (a > 0) input.setRangeText('', a - 1, a, 'end');
  };
  const key = (label, fn, aria) => h('button', { type: 'button', class: 'btn', 'aria-label': aria || label, onclick: fn }, label);
  const pad = touch ? h('div', { class: 'keypad', role: 'group', 'aria-label': T.task.keypad },
    ['7', '8', '9', '4', '5', '6', '1', '2', '3', '/', '0', ','].map((k) => key(k, () => put(k))),
    key('⌫', erase, T.task.erase),
    key('−', () => put('-'), T.task.minus),
    key('C', () => { input.value = ''; }, T.task.clear)) : null;
  return { input, node: h('div', { class: 'field' }, h('label', { for: 'ans' }, T.task.answerLabel), input,
    h('p', { class: 'hint', id: 'ah' }, keypad ? T.task.answerHint : T.task.answerHintText), pad) };
}

function confidenceChips() {
  if (!askConfidence()) return null;
  lastConfidence = null;
  const opts = [['sure', T.task.confSure], ['unsure', T.task.confUnsure], ['guess', T.task.confGuess]];
  const wrap = h('div', { class: 'confidence', role: 'group', 'aria-label': T.task.confTitle });
  const paint = () => wrap.replaceChildren(h('span', { class: 'hint' }, T.task.confTitle), ...opts.map(([k, label]) =>
    h('button', { type: 'button', class: 'btn', 'aria-pressed': String(lastConfidence === k), onclick: () => { lastConfidence = lastConfidence === k ? null : k; paint(); } }, label)));
  paint();
  return wrap;
}

function renderIntro(model) {
  const go_ = guarded(async () => { const r = await act('intro_continue'); await render(r.model); });
  mount(
    h('div', { class: 'intro' },
      h('span', { class: 'ico-big' }, icon(skillIconName(model.skill_id), 40)),
      h('p', { class: 'muted' }, T.intro.newTopic),
      h('h1', {}, model.skill_title),
      h('p', {}, T.intro.text)),
    h('div', { class: 'stack' },
      model.has_lesson ? iconButton('book', T.intro.read, () => go('lesson', { id: model.skill_id, returnTo: 'session' }), 'primary big') : null,
      button(T.intro.skip, go_, 'big')),
    h('p', { class: 'center' }, endButton()));
}

function renderTask(model, flash) {
  const status = h('div', { 'aria-live': 'polite' });
  if (flash) status.append(flash);
  let hintBtn;
  const kind = model.answer_kind || 'number';
  if (shown.number !== model.number) shown = { number: model.number, list: [] };
  const hints = h('div', {}, shown.list.map(hintBox));
  const box = kind === 'choice' ? null : answerBox(() => check({ currentTarget: null }), { keypad: kind === 'number' });
  if (box) box.input.id = 'ans';
  const conf = confidenceChips();
  const check = guarded(async () => {
    if (!box.input.value.trim()) { status.replaceChildren(notice(T.task.emptyAnswer, 'warn')); box.input.focus(); return; }
    const r = await act('answer', { input: box.input.value });
    if (r.model.phase !== 'task') return render(r.model);
    switch (r.result) {
      case 'unreadable': status.replaceChildren(notice(kind === 'text' ? T.task.unreadableText : T.task.unreadable, 'warn')); box.input.select(); break;
      case 'try_again':
        status.replaceChildren(...[
          notice(r.text_retry ? T.task.textRetry(r.letters) : (r.offer_hint ? `${T.task.tryAgain} ${T.task.offerHint}` : T.task.tryAgain)),
          r.diagnosis ? notice(r.diagnosis, 'info') : null].filter(Boolean));
        box.input.select();
        break;
      default: break;
    }
  });
  const askHint = guarded(async () => {
    const r = await act('hint');
    if (r.model.phase !== 'task') return render(r.model, null);
    if (r.wait) {
      status.replaceChildren(notice(T.task.wait));
      setTimeout(() => {
        hintBtn.disabled = true;
        setTimeout(() => { if (hintBtn.isConnected) hintBtn.disabled = false; }, Math.max(1, r.wait.secs) * 1000);
      }, 0);
    } else if (r.need_attempt) {
      status.replaceChildren(notice(T.task.needAttempt));
    } else if (r.blocked) {
      status.replaceChildren(notice(T.task.blocked, 'warn'));
    } else if (r.hint && r.hint.kind === 'text') {
      const item = { level: r.hint.level, text: r.hint.text };
      shown.list.push(item);
      hints.append(hintBox(item));
      status.replaceChildren(h('span', { class: 'sr-only' }, `${T.task.hintN(item.level)}: ${item.text}`));
      hints.lastChild.scrollIntoView({ block: 'nearest' });
    } else if (r.hint && r.hint.kind === 'none') {
      status.replaceChildren(notice(T.task.noMoreHints));
    } else if (r.hint && (r.hint.kind === 'analog' || r.hint.kind === 'ask_adult')) {
      return render(r.model);
    }
  });
  hintBtn = button([icon('hint', 20), T.task.hint], askHint);

  // Завдання з вибором: спершу вибрати, потім підтвердити (випадковий дотик не рахується помилкою).
  let picked = null;
  let confirmBtn = null;
  let choicesNode = null;
  if (kind === 'choice') {
    const buttons = model.choices.map((c, i) => h('button', { type: 'button', class: 'btn', 'aria-pressed': 'false', onclick: () => {
      picked = i;
      buttons.forEach((b, j) => b.setAttribute('aria-pressed', String(j === i)));
      confirmBtn.disabled = false;
    } }, h('span', { class: 'letter', 'aria-hidden': 'true' }, 'АБВГДЕ'[i] || String(i + 1)), ...spoken(c)));
    choicesNode = h('div', { class: 'choices', role: 'group', 'aria-label': T.task.choose }, buttons);
    confirmBtn = button(T.task.confirm, guarded(async () => {
      if (picked === null) return;
      const r = await act('answer', { input: String(picked + 1) });
      await render(r.model);
    }), 'primary', { disabled: true });
  }
  const notNow = confirmButton(T.task.notNow, T.task.notNowSure, guarded(async () => { const r = await act('hand_to_adult'); await render(r.model); }));
  const to = model.testout;
  mount(
    h('h1', { class: 'sr-only' }, model.skill_title),
    h('div', { class: 'meta' }, h('span', { class: 'chip-tag' }, icon(skillIconName(model.skill_id || ''), 16), h('span', { class: 't' }, model.skill_title)), h('span', { class: 'muted' }, T.task.kind[model.kind] || '')),
    to ? notice(T.task.testoutBanner(to.asked + 1, to.total), 'warn') : null,
    model.hard_return ? h('p', { class: 'hard-return' }, icon('refresh', 18), ' ', T.task.hardReturn) : null,
    model.wrap_up ? notice(T.task.wrapUp) : null,
    h('p', { class: 'statement' }, ...spoken(model.statement)),
    figuresNode(model.figures),
    speechOn() ? h('div', { class: 'row' }, speakButton(() => [model.statement, ...(model.choices || [])].join('. '), T.task.speak)) : null,
    hints, choicesNode || box.node, conf, status,
    h('div', { class: 'row' }, kind === 'choice' ? confirmBtn : button(T.task.check, check, 'primary'), model.hints_allowed === false ? null : hintBtn),
    h('div', { class: 'row' }, notNow),
    flagBlock(),
    endButton());
  if (box) box.input.focus();
}

function hintBox(x) {
  return h('div', { class: 'hintbox', role: 'note' }, h('strong', {}, `${T.task.hintN(x.level)}. `), x.text);
}

function renderAnalog(model, flash) {
  const status = h('div', { 'aria-live': 'polite' });
  if (flash) status.append(flash);
  const box = answerBox(() => check({ currentTarget: null }));
  box.input.id = 'ans';
  const check = guarded(async () => {
    const r = await act('answer', { input: box.input.value });
    if (r.result === 'analog_done') return render(r.model, notice(T.task.analogDone, 'good'));
    if (r.result === 'unreadable') status.replaceChildren(notice(T.task.unreadable, 'warn'));
    else status.replaceChildren(notice(T.task.analogAgain));
  });
  mount(
    h('h1', { class: 'sr-only' }, T.task.analogTitle),
    h('div', { class: 'meta' }, h('span', { class: 'chip-tag' }, icon(skillIconName(model.skill_id || ''), 16), h('span', { class: 't' }, model.skill_title)), h('span', { class: 'muted' }, T.task.analogTitle)),
    h('p', { class: 'muted' }, T.task.analogIntro),
    h('p', { class: 'statement' }, ...spoken(model.statement)),
    figuresNode(model.figures),
    h('ol', { class: 'steps' }, model.steps.map((s) => h('li', {}, ...spoken(s)))),
    h('p', {}, h('strong', {}, model.prompt)),
    box.node, status,
    h('div', { class: 'row' },
      button(T.task.check, check, 'primary'),
      button(T.task.backToTask, guarded(async () => { const r = await act('back_to_task'); await render(r.model); }))),
    h('details', {}, h('summary', {}, T.task.yourTask), h('p', { class: 'statement' }, model.task_statement)),
    endButton());
  box.input.focus();
}

export function badgeList(list) {
  if (!list || !list.length) return null;
  return h('div', {}, h('h2', {}, T.newBadge),
    h('div', { class: 'stickers' }, list.map((b) => h('div', { class: 'sticker earned pop' },
      sticker(BADGE_ART[b.group] || 'mastery', 64), h('strong', {}, b.title)))));
}

function confidenceNote(model) {
  if (!askConfidence() || !lastConfidence) return null;
  const c = lastConfidence;
  lastConfidence = null;
  if (model.outcome === 'missed' && c === 'sure') return notice(T.feedback.confSurprise);
  if (model.outcome === 'solved' && c === 'guess') return notice(T.feedback.confLucky);
  return null;
}

function supportBlock(model) {
  if (!model.support) return null;
  const last = guarded(async () => { const r = await act('end_session'); await render(r.model); });
  if (model.support === 'prereq') {
    const weak = model.weak_prerequisites || [];
    return h('div', { class: 'support' }, h('p', {}, T.feedback.supportPrereq),
      h('div', { class: 'stack' }, weak.map((w) => button(T.feedback.repeatTopic(w.title), guarded(async () => {
        await act('end_session');
        await go('skill', { id: w.id });
      })))),
      h('div', { class: 'row tight' }, iconButton('book', T.feedback.readAgain, () => go('lesson', { id: model.skill_id, returnTo: 'session' })), button(T.feedback.pause, last)));
  }
  if (model.support === 'lesson') {
    return h('div', { class: 'support' }, h('p', {}, T.feedback.supportLesson),
      h('div', { class: 'row tight' },
        iconButton('book', T.feedback.readAgain, () => go('lesson', { id: model.skill_id, returnTo: 'session' }), 'primary'),
        button(T.feedback.pause, last)));
  }
  return h('div', { class: 'support' }, h('p', {}, T.feedback.supportBreak),
    h('div', { class: 'row tight' }, button(T.feedback.pause, last, 'primary'), button(T.calm.link, () => go('calm'))));
}

function renderFeedback(model) {
  shown = { number: -1, list: [] };
  const next = guarded(async () => { const r = await act('continue'); await render(r.model); });
  const last = guarded(async () => { const r = await act('end_session'); await render(r.model); });
  const testout = model.testout;
  mount(
    h('h1', { class: 'sr-only' }, T.feedback.title),
    h('div', { class: 'feedback-top' }, model.outcome === 'solved'
      ? h('span', { class: 'stamp', role: 'status' }, icon('check', 34), T.feedback.stampSolved)
      : model.outcome === 'missed'
        ? h('span', { class: 'stamp note', role: 'status' }, T.feedback.stampMissed)
        : h('span', { class: 'stamp note', role: 'status' }, T.feedback.stampAdult),
      h('p', { class: 'muted' }, model.outcome === 'solved' ? T.feedback.solved : model.outcome === 'missed' ? T.feedback.missed : T.feedback.adult)),
    model.reveal ? h('div', { class: 'card' },
      h('p', {}, h('strong', {}, `${T.feedback.correctWas}: `), ...spoken(model.reveal.answer)),
      model.reveal.explanation ? h('p', {}, h('strong', {}, `${T.feedback.why}: `), ...spoken(model.reveal.explanation)) : null,
      speechOn() ? speakButton(() => `${model.reveal.answer}. ${model.reveal.explanation || ''}`, T.task.speak) : null) : null,
    confidenceNote(model),
    model.gap === 'closed' ? h('div', { class: 'gap-win pop' }, h('strong', {}, T.feedback.gapClosedTitle), h('p', {}, T.feedback.gapClosed)) : null,
    model.gap === 'opened' ? notice(T.feedback.gapOpened, 'info') : null,
    model.gap === 'better' ? notice(T.feedback.gapBetter, 'good') : null,
    model.gap === 'still' ? notice(T.feedback.gapStill, 'info') : null,
    model.mastered ? notice(T.feedback.mastered(model.mastered), 'good') : null,
    badgeList(model.badges),
    model.stickers.length ? h('div', {}, h('h2', {}, T.feedback.newSticker), h('div', { class: 'stickers' }, model.stickers.map(stickerBadge))) : null,
    supportBlock(model),
    model.suggest_break && !model.support ? notice(T.feedback.breakSuggest) : null,
    model.wrap_up ? notice(T.task.wrapUp) : null,
    h('div', { class: 'row' }, button(T.feedback.next, next, 'primary'), button(T.feedback.finish, last)),
    flagBlock());
}

function renderEnded(model) {
  const to = model.testout;
  const reflect = (!to && model.practiced && model.practiced.length) ? reflectionBlock(model.practiced) : null;
  mount(
    h('h1', { class: 'sr-only' }, T.ended.title),
    h('div', { class: 'feedback-top' },
      h('span', { class: `stamp ${to && !to.passed ? 'note' : ''}`.trim(), role: 'status' }, to ? (to.passed ? [icon('check', 34), T.testout.passed] : T.testout.notYet) : T.ended.stamp),
      h('p', { class: 'muted' }, to ? (to.passed ? T.testout.passedText(to.skill_title) : T.testout.notYetText) : (T.ended[model.reason] || T.ended.user))),
    to ? null : h('p', { class: 'center' }, T.ended.summary(model.summary.tasks, model.summary.solved)),
    model.stickers.length ? h('div', { class: 'stickers' }, model.stickers.map(stickerBadge)) : null,
    reflect,
    h('p', { class: 'muted center' }, T.ended.go_play),
    h('div', { class: 'row' }, button(T.ended.home, () => go('home'), 'primary')));
}

// Метапізнання: що було найскладнішим (одна необов'язкова відмітка; потрапляє в аркуш для вчителя).
function reflectionBlock(practiced) {
  const out = h('div', { 'aria-live': 'polite' });
  const wrap = h('div', { class: 'card' }, h('h3', { class: 'tight' }, T.reflect.title), h('p', { class: 'hint' }, T.reflect.hint),
    h('div', { class: 'chips' }, [
      ...practiced.map((p) => h('button', { type: 'button', class: 'btn chip', onclick: guarded(async () => {
        await call('reflect', { skill: p.id });
        out.replaceChildren(notice(T.reflect.thanks, 'good'));
      }) }, p.title)),
      h('button', { type: 'button', class: 'btn chip', onclick: () => out.replaceChildren(notice(T.reflect.fine, 'good')) }, T.reflect.none)]), out);
  return wrap;
}
