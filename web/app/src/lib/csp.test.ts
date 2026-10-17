import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import config from '../../svelte.config.js';
import { PAGE_FRAME_SRC } from './card/policy.js';

// SPEC-028 R1, R14, A13; ADR-007. SvelteKit writes the page's policy into a meta element at build
// and adds a sha256 hash for each inline script it generates (kit.csp in hash mode), so the only
// scripts that run are the app's own files, those hashed scripts and Telegram's. The directives a
// meta element cannot carry (frame-ancestors) are the Caddy header's (SPEC-032).
const TELEGRAM = 'https://telegram.org';
// SPEC-338 R7, ADR-349: the web engine's Worker compiles WebAssembly, which this keyword admits and
// `'unsafe-eval'` would admit with `eval` beside it.
const WASM = 'wasm-unsafe-eval';
// SPEC-028's policy, before SPEC-338: what the page admitted with no engine.
const BEFORE: Record<string, readonly string[]> = {
  'script-src': ['self', TELEGRAM],
  'object-src': ['none'],
  'base-uri': ['self'],
  'connect-src': ['self']
};
const SHELL = readFileSync(new URL('../app.html', import.meta.url), 'utf8');

/** The sources in `sources` that would let a script run without the page vouching for it. */
function unsafe(sources: readonly string[]): string[] {
  return sources.filter(
    (source) =>
      /^'?unsafe-(?!hashes)/.test(source) ||
      ['*', 'data:', 'blob:', 'http:', 'https:', 'filesystem:'].includes(source)
  );
}

describe('the page policy', () => {
  it('the page policy admits scripts only from itself, its hashes and telegram.org', () => {
    const csp = config.kit?.csp;

    expect(csp?.mode).toBe('hash');
    expect(csp?.directives).toEqual({
      'script-src': ['self', TELEGRAM, WASM],
      'object-src': ['none'],
      'base-uri': ['self'],
      'connect-src': ['self'],
      'frame-src': ['none']
    });
    // and the judgement refuses a weaker policy
    expect(unsafe(["'self'", "'unsafe-inline'", "'unsafe-eval'", '*', 'https:', TELEGRAM])).toEqual([
      "'unsafe-inline'",
      "'unsafe-eval'",
      '*',
      'https:'
    ]);
    expect(unsafe(csp?.directives?.['script-src'] ?? [])).toEqual([]);
  });

  it('the page policy admits WebAssembly compilation and nothing else new', () => {
    const directives = (config.kit?.csp?.directives ?? {}) as Record<string, readonly string[]>;
    const difference = (from: typeof BEFORE, to: typeof BEFORE) =>
      Object.entries(from).flatMap(([directive, sources]) =>
        sources
          .filter((source) => !(to[directive] ?? []).includes(source))
          .map((source) => `${directive} ${source}`)
      );

    // two sources are new, WebAssembly's keyword in script-src and the card frame's frame-src
    // 'none' (SPEC-341 R13), and SPEC-028's policy is whole
    expect(difference(directives, BEFORE)).toEqual([`script-src ${WASM}`, 'frame-src none']);
    expect(difference(BEFORE, directives)).toEqual([]);
    // and the keyword is not the general eval: the judgement admits it and refuses that
    expect(unsafe([WASM, `'${WASM}'`, "'unsafe-eval'", 'unsafe-eval'])).toEqual([
      "'unsafe-eval'",
      'unsafe-eval'
    ]);
  });

  it('the page sends no referrer to another origin', () => {
    // A meta referrer policy governs only what loads after it, so it precedes everything SvelteKit
    // adds to the head; the start hook appends Telegram's script on a launch later still (SPEC-400 R3).
    const policy = SHELL.indexOf('<meta name="referrer" content="same-origin" />');
    const head = SHELL.indexOf('%sveltekit.head%');

    expect(policy).toBeGreaterThan(-1);
    expect(policy).toBeLessThan(head);
  });

  it('the page shell carries no script, inline or by URL', () => {
    // SPEC-400 R7, A11: the start hook adds Telegram's script only on a launch, so the shell runs
    // none, and nothing is inlined into it.
    const scriptsIn = (shell: string) =>
      [...shell.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)].map(([, attributes, body]) => ({
        attributes: attributes.trim(),
        body: body.trim()
      }));
    // the census finds a script by URL and an inline one in a planted shell
    expect(
      scriptsIn(`<head><script src="${TELEGRAM}/js/telegram-web-app.js"></script><script>go()</script></head>`)
    ).toEqual([
      { attributes: `src="${TELEGRAM}/js/telegram-web-app.js"`, body: '' },
      { attributes: '', body: 'go()' }
    ]);

    expect(scriptsIn(SHELL)).toEqual([]);
  });

  it('the page policy lets no frame navigate', () => {
    // SPEC-341 R5, A2 (SEC01-F13). A frame's own navigation, one the card frame starts itself
    // included, is checked against the embedding page's frame-src, so 'none' holds every frame to
    // the srcdoc document it was given; the value is the card module's, which the card harness
    // serves too.
    const directives = config.kit?.csp?.directives;

    expect(directives?.['frame-src']).toEqual(['none']);
    expect(directives?.['frame-src']).toBe(PAGE_FRAME_SRC);
    // and the card frame adds no other directive to the page policy
    expect(Object.keys(directives ?? {}).sort()).toEqual([
      'base-uri',
      'connect-src',
      'frame-src',
      'object-src',
      'script-src'
    ]);
  });
});
