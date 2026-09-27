import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import config from '../../svelte.config.js';

// SPEC-028 R1, R14, A13; ADR-007. SvelteKit writes the page's policy into a meta element at build
// and adds a sha256 hash for each inline script it generates (kit.csp in hash mode), so the only
// scripts that run are the app's own files, those hashed scripts and Telegram's. The directives a
// meta element cannot carry (frame-ancestors) are the Caddy header's (SPEC-032).
const TELEGRAM = 'https://telegram.org';
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
      'script-src': ['self', TELEGRAM],
      'object-src': ['none'],
      'base-uri': ['self'],
      'connect-src': ['self']
    });
    // the page shell runs one script, Telegram's, by its URL: nothing is inlined into it
    const scripts = [...SHELL.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)].map(
      ([, attributes, body]) => ({ attributes: attributes.trim(), body: body.trim() })
    );
    expect(scripts).toEqual([
      { attributes: `src="${TELEGRAM}/js/telegram-web-app.js"`, body: '' }
    ]);
    // and the judgement refuses a weaker policy
    expect(unsafe(["'self'", "'unsafe-inline'", "'unsafe-eval'", '*', 'https:', TELEGRAM])).toEqual([
      "'unsafe-inline'",
      "'unsafe-eval'",
      '*',
      'https:'
    ]);
    expect(unsafe(csp?.directives?.['script-src'] ?? [])).toEqual([]);
  });

  it('the page sends no referrer to another origin', () => {
    // A meta referrer policy governs only what loads after it, so it precedes Telegram's script.
    const policy = SHELL.indexOf('<meta name="referrer" content="same-origin" />');
    const telegram = SHELL.indexOf('<script src=');

    expect(policy).toBeGreaterThan(-1);
    expect(policy).toBeLessThan(telegram);
  });
});
