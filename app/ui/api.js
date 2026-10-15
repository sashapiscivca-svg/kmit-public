// Єдина точка зв'язку з ядром: у браузері через fetch до dev-сервера, у Tauri через invoke.
// Годинник читає лише тут (ядро годинника не має).

export function makeCtx() {
  const now = Date.now();
  const d = new Date(now);
  const offsetSecs = -d.getTimezoneOffset() * 60;
  const wall = Math.floor(now / 1000);
  return {
    wall,
    day: Math.floor((wall + offsetSecs) / 86400),
    minute: d.getHours() * 60 + d.getMinutes(),
  };
}

let chain = Promise.resolve();

// Виклики до ядра виконуються по черзі: два одночасні натискання не можуть зіткнутися.
export function call(name, args = {}) {
  const run = chain.then(() => callNow(name, args));
  chain = run.catch(() => {});
  return run;
}

async function callNow(name, args) {
  const ctx = makeCtx();
  const tauri = window.__TAURI__ && window.__TAURI__.core;
  let out;
  if (tauri) {
    out = await tauri.invoke('call', { name, args, ctx });
  } else {
    const r = await fetch(`/api/${name}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ args, ctx }),
    });
    out = await r.json();
  }
  if (out && out.error) {
    const e = new Error(out.error.message);
    e.code = out.error.code;
    throw e;
  }
  return out;
}
