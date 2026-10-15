import { call } from './api.js';
import { T } from './uk.js';
import { h, mount, button, notice, field, guarded, iconButton, iconOnly, icon, downloadBase64 } from './ui.js';
import { heroSvg } from './hero.js';
import { register, go, state } from './router.js';

function errText(e) {
  switch (e.code) {
    case 'wrong_pin': return T.wrongPin;
    case 'invalid_pin': return T.invalidPin;
    case 'profile_data_lost': return T.profileLost;
    case 'wrong_recovery_code': return T.wrongCode;
    case 'wrong_passphrase': return T.wrongPass;
    default: return e.message || T.genericError;
  }
}
export { errText };

register('profiles', async () => {
  const { profiles, can_create } = await call('profiles');
  const list = profiles.length === 0
    ? h('p', { class: 'muted' }, T.noProfiles)
    : h('div', { class: 'profiles' }, profiles.map((p) =>
        p.kind === 'teacher'
          ? h('button', { type: 'button', class: 'btn profile profile-teacher', onclick: () => go('pin', { slot: p.slot, kind: 'teacher' }) },
              h('span', { class: 'hero-mini teacher-mini', 'aria-hidden': 'true' }, icon('book', 30)),
              h('span', {}, T.teacher.profileLabel))
          : h('button', { type: 'button', class: 'btn profile', onclick: () => go('pin', { slot: p.slot, px: p.px }) },
              h('span', { class: 'hero-mini' }, heroSvg(p.px, 56)),
              h('span', {}, `Профіль ${p.slot + 1}`))));
  mount(
    h('h1', { class: 'logo' }, T.appName),
    h('p', { class: 'muted' }, T.tagline),
    profiles.length === 0 && can_create
      ? h('button', { type: 'button', class: 'cover s-math big-cover', 'aria-label': T.newProfile, onclick: () => go('role') },
          h('span', { class: 'label' }, h('span', { class: 'ico' }, icon('pencil', 22)), h('span', { class: 'name' }, T.firstNotebook)),
          h('span', { class: 'meta2' }, h('span', {}, T.newProfile)))
      : null,
    profiles.length ? h('h2', {}, T.whoIsLearning) : null,
    profiles.length ? list : null,
    h('div', { class: 'row' },
      can_create && profiles.length ? button(T.newProfile, () => go('role'), 'primary') : null,
      button(T.importProfile, () => go('import'))),
    h('p', { class: 'hint' }, T.legalNote, ' ', h('button', { type: 'button', class: 'btn quiet', onclick: () => go('privacy', { to: 'profiles' }) }, T.privacyLink)));
});

register('pin', async ({ slot, px, kind }) => {
  const input = h('input', { type: 'password', inputmode: 'text', autocomplete: 'off', 'aria-describedby': 'msg' });
  const msg = h('div', { id: 'msg' });
  const submit = guarded(async () => {
    try {
      await call('open_profile', { slot, pin: input.value });
      state.slot = slot;
      state.kind = kind === 'teacher' ? 'teacher' : 'child';
      await go(kind === 'teacher' ? 'cabinet' : 'home');
    } catch (e) {
      msg.replaceChildren(notice(errText(e), 'error'));
      input.select();
    }
  });
  input.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') submit({ currentTarget: null }); });
  mount(
    h('div', { class: 'row' }, kind === 'teacher' ? h('span', { class: 'hero-mini teacher-mini', 'aria-hidden': 'true' }, icon('book', 40)) : h('span', { class: 'hero-mini' }, heroSvg(px || [], 72))), h('h1', {}, kind === 'teacher' ? T.teacher.profileLabel : T.pinTitle),
    field(T.pinPlaceholder, input), msg,
    h('div', { class: 'row' },
      button(T.open, submit, 'primary'),
      button(T.back, () => go('profiles'))),
    button(kind === 'teacher' ? T.teacher.formal.forgotPin : T.forgotPin, () => go('recover', { slot, kind }), 'quiet'));
  input.focus();
});

register('adultnotice', async ({ next = 'create' } = {}) => {
  const a = T.adultNotice;
  mount(
    h('div', { class: 'toolbar' }, h('h1', {}, a.title), iconOnly('back', T.back, () => go('role'), 'quiet')),
    h('p', {}, a.intro),
    h('ul', { class: 'prose' }, a.items.map((x) => h('li', {}, x))),
    h('p', { class: 'hint' }, a.more),
    h('div', { class: 'row' },
      button(a.ack, () => go(next), 'primary'),
      button(T.privacyLink, () => go('privacy', { to: 'adultnotice' }), 'quiet')));
});

register('create', async () => {
  const opts = await call('hero_options');
  const look = { skin: 1, hair_color: 1, hair_style: 'hair_short' };
  const nick = h('input', { type: 'text', maxlength: 24, autocomplete: 'off' });
  const pin1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const pin2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div');
  const stage = h('div', { class: 'hero-stage' });
  const controls = h('div', {});
  const drawHero = async () => {
    const r = await call('hero_preview', look);
    stage.replaceChildren(heroSvg(r.px, 150));
  };
  const swatches = (list, key, label) => h('div', { class: 'swatches', role: 'group', 'aria-label': label }, list.map((s) => {
    const b = h('button', { type: 'button', class: 'swatch', 'aria-label': s.name, 'aria-pressed': String(s.i === look[key]), onclick: () => { look[key] = s.i; paint(); } });
    b.style.background = s.color;
    return b;
  }));
  const paint = () => {
    controls.replaceChildren(
      h('p', {}, h('strong', {}, T.hero.skin)), swatches(opts.skins, 'skin', T.hero.skin),
      h('p', {}, h('strong', {}, T.hero.hairColor)), swatches(opts.hair_colors, 'hair_color', T.hero.hairColor),
      h('p', {}, h('strong', {}, T.hero.hairStyle)),
      h('div', { class: 'tabs wrap', role: 'group', 'aria-label': T.hero.hairStyle }, opts.hair_styles.map((s) =>
        h('button', { type: 'button', class: 'btn', 'aria-pressed': String(s.id === look.hair_style), onclick: () => { look.hair_style = s.id; paint(); } }, s.name))));
    drawHero();
  };
  paint();
  const submit = guarded(async () => {
    if (pin1.value !== pin2.value) { msg.replaceChildren(notice(T.pinMismatch, 'error')); return; }
    try {
      const r = await call('create_profile', { nickname: nick.value, pin: pin1.value, ...look });
      state.slot = r.slot;
      state.kind = 'child';
      try { await call('settings_set', { adult_notice_ack: true }); } catch { /* не критично */ }
      await go('recovery', { code: r.recovery_code, next: 'welcome' });
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('h1', {}, T.createHero),
    h('p', { class: 'muted' }, T.createHeroText),
    stage, controls,
    field(T.nicknameLabel, nick, T.nicknameHint),
    field(T.pinNewLabel, pin1, T.pinNewHint),
    field(T.pinRepeat, pin2), msg,
    h('div', { class: 'row' }, button(T.create, submit, 'primary'), button(T.back, () => go('profiles'))),
    h('p', { class: 'hint' }, T.legalNote, ' ', h('button', { type: 'button', class: 'btn quiet', onclick: () => go('privacy', { to: 'profiles' }) }, T.privacyLink)));
  nick.focus();
});

register('recovery', async ({ code, next }) => {
  const F = (k) => (state.kind === 'teacher' ? T.teacher.formal[k] || T[k] : T[k]);
  const note = h('div', { 'aria-live': 'polite' });
  const nickOf = async () => {
    try { return (await call(state.kind === 'teacher' ? 'teacher_home' : 'home')).nickname || ''; } catch { return ''; }
  };
  const ok = h('input', { type: 'checkbox' });
  const go_ = button(T.recoverySaved, async () => { await go(next); }, 'primary', { disabled: true });
  ok.addEventListener('change', () => { go_.disabled = !ok.checked; });
  mount(
    h('h1', {}, T.recoveryTitle),
    h('p', {}, F('recoveryText')),
    h('p', { class: 'code', 'aria-label': code }, code.split('-').map((g) => h('span', {}, g))),
    h('div', { class: 'row' },
      button(T.recoverySavePdf, guarded(async () => {
        try {
          const r = await call('recovery_sheet', { code, nickname: await nickOf() });
          if (await downloadBase64(r.data, r.filename, 'application/pdf')) note.replaceChildren(notice(T.saved, 'good'));
        } catch (e) { note.replaceChildren(notice(errText(e), 'error')); }
      })),
      button(T.recoveryCopy, guarded(async () => {
        try { await navigator.clipboard.writeText(code); note.replaceChildren(notice(F('recoveryCopied'), 'good')); } catch { note.replaceChildren(notice(F('recoveryCopyFail'), 'warn')); }
      }), 'quiet')),
    note,
    h('label', { class: 'check' }, ok, h('span', {}, T.recoveryConfirm)),
    h('div', { class: 'row' }, go_),
    h('p', { class: 'hint', 'aria-live': 'polite' }, F('recoveryLockedHint')));
});

register('recover', async ({ slot, kind }) => {
  const F = (k) => (kind === 'teacher' ? T.teacher.formal[k] || T[k] : T[k]);
  const code = h('input', { type: 'text', autocomplete: 'off', autocapitalize: 'characters', autocorrect: 'off', spellcheck: 'false' });
  const pin1 = h('input', { type: 'password', autocomplete: 'new-password' });
  const pin2 = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div');
  const submit = guarded(async () => {
    if (pin1.value !== pin2.value) { msg.replaceChildren(notice(T.pinMismatch, 'error')); return; }
    try {
      const r = await call('recover_profile', { slot, code: code.value, new_pin: pin1.value });
      state.slot = slot;
      state.kind = r.kind === 'teacher' ? 'teacher' : 'child';
      await go(state.kind === 'teacher' ? 'cabinet' : 'home');
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('h1', {}, T.recoverTitle), h('p', {}, F('recoverText')),
    field(T.recoverCode, code), field(F('pinNewLabel'), pin1), field(F('pinRepeat'), pin2), msg,
    h('div', { class: 'row' }, button(T.recoverDo, submit, 'primary'), button(T.back, () => go('profiles'))));
});

register('import', async () => {
  const file = h('input', { type: 'file', accept: '.kmit' });
  const pass = h('input', { type: 'password', autocomplete: 'off' });
  const pin = h('input', { type: 'password', autocomplete: 'new-password' });
  const msg = h('div');
  const submit = guarded(async () => {
    const f = file.files && file.files[0];
    if (!f) return;
    const buf = new Uint8Array(await f.arrayBuffer());
    let bin = '';
    for (let i = 0; i < buf.length; i++) bin += String.fromCharCode(buf[i]);
    try {
      const r = await call('import_profile', { data: btoa(bin), passphrase: pass.value, pin: pin.value });
      state.slot = r.slot;
      state.kind = r.kind === 'teacher' ? 'teacher' : 'child';
      await go('recovery', { code: r.recovery_code, next: r.kind === 'teacher' ? 'cabinet' : 'welcome' });
    } catch (e) { msg.replaceChildren(notice(errText(e), 'error')); }
  });
  mount(
    h('h1', {}, T.importTitle),
    field(T.importFile, file), field(T.importPass, pass), field(T.importPin, pin), msg,
    h('div', { class: 'row' }, button(T.importDo, submit, 'primary'), button(T.back, () => go('profiles'))));
});

// Вступ при першому запуску: як тут вчитися, вибір класу.
register('welcome', async () => {
  const st = await call('settings_get');
  if (st.onboarded) { await go('home'); return; }
  let step = 0;
  let grade = st.grade || 5;
  const texts = [T.welcome.s1, T.welcome.s2, T.welcome.s3];
  const finish = async () => { await call('settings_set', { onboarded: true, grade }); await go('home'); };
  const paint = () => {
    if (step < 3) {
      mount(
        h('div', { class: 'welcome' },
          h('h1', { class: 'logo' }, T.appName),
          h('h1', {}, T.welcome.title),
          h('div', { class: 'pager-dots', role: 'img', 'aria-label': T.learn.stepOf(step + 1, 4) }, [0, 1, 2, 3].map((k) => h('i', { class: k <= step ? 'on' : '' }))),
          h('p', { class: 'prose' }, texts[step])),
        h('div', { class: 'row' }, button(T.welcome.next, () => { step += 1; paint(); }, 'primary')));
    } else {
      mount(
        h('div', { class: 'welcome' }, h('h1', {}, T.welcome.gradeTitle),
          h('div', { class: 'grade-row', role: 'group', 'aria-label': T.welcome.gradeTitle },
            [5, 6, 7, 8, 9, 10, 11, 12].map((g) => h('button', { type: 'button', class: 'btn', 'aria-pressed': String(g === grade), onclick: () => { grade = g; paint(); } }, g === 12 ? T.grades.nmt : String(g)))),
          h('p', { class: 'hint' }, T.welcome.gradeNote)),
        h('div', { class: 'row' }, button(T.welcome.start, guarded(finish), 'primary big')));
    }
  };
  paint();
});
