import { go } from './router.js';
import './start.js';
import './home.js';
import './session.js';
import './extras.js';
import './learn.js';
import './lesson.js';
import './recall.js';
import './review.js';
import './hero.js';
import './teacher.js';
import './ceremony.js';
import { mount, h } from './ui.js';
import { T } from './uk.js';

window.addEventListener('unhandledrejection', (ev) => {
  console.error(ev.reason);
  mount(h('h1', {}, T.appName), h('p', { class: 'notice error', role: 'alert' }, T.genericError),
    h('div', { class: 'row' },
      h('button', { class: 'btn primary', type: 'button', onclick: () => go('home').catch(() => go('profiles')) }, T.errorHome),
      h('button', { class: 'btn', type: 'button', onclick: () => go('profiles') }, T.errorProfiles)));
});

try {
  const sc = Number(localStorage.getItem('scale'));
  if (sc >= 80 && sc <= 200) document.documentElement.style.setProperty('--scale', String(sc / 100));
  if (localStorage.getItem('reading') === '1') document.documentElement.classList.add('reading');
} catch { /* не критично */ }

go('profiles');
