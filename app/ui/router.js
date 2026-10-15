// Мінімальний маршрутизатор: екрани реєструються за назвою; кнопка «Назад» телефону повертає на попередній екран.
const routes = new Map();
export const state = { slot: null, kind: null };
// Екрани кабінету вчителя мають свою обкладинку (клас role-teacher на <html>).
const TEACHER_SCREENS = new Set(['cabinet', 'tworks', 'tnotes', 'tlesson', 'tplan', 'tdata', 'teacher_create']);
const stack = [];
// На ці екрани «Назад» не повертає (там одноразові дані, як-от код відновлення).
const NO_RETURN = new Set(['recovery']);

export function register(name, fn) { routes.set(name, fn); }

async function show(name, params) {
  const fn = routes.get(name);
  if (!fn) throw new Error(`немає екрана ${name}`);
  document.documentElement.classList.toggle('role-teacher', TEACHER_SCREENS.has(name) || (name === 'pin' && params && params.kind === 'teacher') || (name === 'recovery' && state.kind === 'teacher'));
  await fn(params);
}

export async function go(name, params = {}) {
  stack.push({ name, params });
  history.pushState({ n: stack.length }, '');
  await show(name, params);
}

window.addEventListener('popstate', async () => {
  stack.pop();
  while (stack.length > 1 && NO_RETURN.has(stack[stack.length - 1].name)) stack.pop();
  const t = stack[stack.length - 1];
  if (t) await show(t.name, t.params);
});

// «Назад» туди, звідки прийшли; якщо історії немає, то на запасний екран.
export async function goBack(name = 'home', params = {}) {
  if (stack.length > 1 && stack[stack.length - 2].name !== 'recovery') history.back();
  else await go(name, params);
}
