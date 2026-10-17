import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// SPEC-028 R2, A1: `telegram.svelte.ts` is the only module of the Mini App that reaches Telegram's
// Mini App object, its raw bridge or the signed launch data. A second reader could call a method
// past the wrapper's version gates, or carry the launch data somewhere it must never go, so every
// other shipped module is held to none of these names. The census reads text: a comment counts.
const APP = fileURLToPath(new URL('../..', import.meta.url));
const SRC = join(APP, 'src');
const WRAPPER = 'src/lib/telegram.svelte.ts';
// SPEC-400 R8: the launch module is the one reader of the launch parameters, and the start hook
// reaches it.
const LAUNCH = 'src/lib/telegram-launch.ts';
const HOOK = 'src/hooks.client.ts';
const PARAMETER = /\btgWebApp[A-Z]/;

const TOUCHES: readonly RegExp[] = [
  // a member of the global `Telegram` object, reached with `.`, `?.` or brackets (a sentence that
  // ends in the word, as in "outside Telegram.", is prose, not a member)
  /\bTelegram\s*(?:(?:\?\.|\.)[A-Za-z_$]|\[)/,
  // the global object by a computed name, as in `window['Telegram']`
  /\[\s*(['"`])Telegram\1\s*\]/,
  // the raw bridge Telegram's script speaks to the client through
  /\bTelegramWebviewProxy\b/,
  // the signed launch data, or the client-side parse of it that nothing may trust
  /\binitData(?:Unsafe)?\b/
];

function touches(text: string): boolean {
  return TOUCHES.some((pattern) => pattern.test(text));
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every shipped source file under `dir`: code, components and the page shell, never a test or the generated messages. */
function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : sources(path);
      if (!/\.(?:ts|js|svelte|html)$/.test(entry.name)) return [];
      if (/\.(?:test|spec)\.[jt]s$/.test(entry.name)) return [];
      return [relative(APP, path)];
    })
    .sort();
}

describe('the Telegram boundary', () => {
  it('only the wrapper touches the Telegram WebApp object', () => {
    const files = examined('Mini App source files', sources(SRC));

    const touching = files.filter((file) => touches(readFileSync(join(APP, file), 'utf8')));

    expect(touching).toEqual([WRAPPER]);
  });

  it('the census recognises every way a module reaches the object', () => {
    // The first reader is assembled at run time: a text scanner that walks test files would
    // otherwise read this fixture as a module that runs the Mini App.
    const planted = [
      `window.${'Telegram'}.WebApp.ready();`,
      'const app = window.Telegram?.WebApp;',
      "const app = globalThis['Telegram'].WebApp;",
      "window.TelegramWebviewProxy.postEvent('web_app_ready');",
      "fetch('/api/session', { body: app.initData });",
      'const user = app.initDataUnsafe.user;'
    ];

    expect(planted.filter(touches)).toEqual(planted);
    expect(touches("import { telegram } from '$lib/telegram.svelte'; telegram.ready();")).toBe(false);
    expect(touches('// null outside Telegram. The wrapper says so.')).toBe(false);
  });

  it("only the launch module names Telegram's launch parameters", () => {
    // SPEC-400 R8, A9: one module decides a launch, so no second reader can decide it by another
    // rule. The planted names are assembled at run time, so this file never names one itself.
    const planted = [`${'tgWebApp'}Data`, `${'tgWebApp'}Version`, `${'tgWebApp'}Platform`];
    expect(planted.filter((name) => PARAMETER.test(name))).toEqual(planted);

    const files = examined('Mini App source files', sources(SRC));
    const naming = files.filter((file) => PARAMETER.test(readFileSync(join(APP, file), 'utf8')));

    expect(naming).toEqual([LAUNCH]);
  });

  it("the hook that loads Telegram's script never imports the wrapper", () => {
    // SPEC-400 R8, A10: the start hook runs before any component and before the script has
    // loaded, so it reaches the launch module and never the wrapper that reads the script's object.
    const wrapper = /telegram\.svelte/;
    expect(wrapper.test("import { telegram } from '$lib/telegram.svelte';")).toBe(true);

    const hook = readFileSync(join(APP, HOOK), 'utf8');
    const launch = readFileSync(join(APP, LAUNCH), 'utf8');

    expect(wrapper.test(hook), HOOK).toBe(false);
    expect(wrapper.test(launch), LAUNCH).toBe(false);
    expect(/from\s+['"]\$lib\/telegram-launch['"]/.test(hook), `${HOOK} imports the launch module`).toBe(true);
  });
});
