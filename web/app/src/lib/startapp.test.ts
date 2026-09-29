import { afterEach, describe, expect, it, vi } from 'vitest';
import { routeFor } from './startapp';
import { ROUTES, TODAY } from './routes';

// SPEC-028 R3, A2, A3; ADR-028. A startapp token is text anyone can put in a link, so it opens a
// screen only through a closed table, and everything else opens Today: never a 404, and never a
// navigation to the token's own text.
describe('the startapp token map', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('an unknown or empty startapp token opens Today', () => {
    const unknown = [
      '',
      null,
      undefined,
      'settings',
      'reading',
      'About',
      'TODAY',
      // names every plain object answers, which a lookup table must not
      'constructor',
      '__proto__',
      'toString',
      'hasOwnProperty',
      // the longest well-formed token, unknown to the table
      'a'.repeat(64)
    ];
    for (const token of unknown) {
      expect(routeFor(token), `token ${JSON.stringify(token)}`).toBe(TODAY);
    }

    // and the table's own tokens still open their screens
    expect(routeFor('today')).toBe('/');
    expect(routeFor('about')).toBe('/about');
  });

  it('a token shaped like a path or too long opens Today', () => {
    const malformed = [
      '/about',
      'about/',
      '/',
      '.',
      '..',
      './about',
      '../about',
      'about/../about',
      '%2Fabout',
      'about%00',
      '//example.org/about',
      'https://example.org/about',
      'javascript:alert(1)',
      'data:text/html,about',
      'about?x=1',
      'about#top',
      ' about',
      'about ',
      'ab out',
      'about\n',
      'about.',
      // one past the 64 characters a token may hold, and Telegram's own 512
      'a'.repeat(65),
      'about'.padEnd(65, 'x'),
      'x'.repeat(512)
    ];
    for (const token of malformed) {
      expect(routeFor(token), `token ${JSON.stringify(token)}`).toBe(TODAY);
    }
  });

  it('every destination is a screen of the route table', () => {
    const destinations = ['today', 'about'].map(routeFor).sort();

    expect(destinations).toEqual([...ROUTES].sort());
  });

  it('Today is the root path, and every launch without a known destination opens it', () => {
    expect(TODAY).toBe('/');
    expect(routeFor(null)).toBe('/');
    expect(routeFor('unlisted')).toBe('/');
  });

  it('a token reaches the table only when it has the shape of a token, and the table lists today', () => {
    const lookups = vi.spyOn(Map.prototype, 'get');
    const malformed = [null, undefined, '', '/about', 'about/', ' about', 'about ', 'a b', 'x'.repeat(65)];
    for (const token of malformed) routeFor(token);
    // a token the shape refuses is never looked up
    expect(lookups.mock.calls.map(([key]) => key)).toEqual([]);

    expect(routeFor('today')).toBe('/');
    // the table answers for today itself, before the fallback would
    expect(lookups).toHaveBeenLastCalledWith('today');
    expect(lookups.mock.results.at(-1)?.value).toBe('/');
    expect(routeFor('a'.repeat(64))).toBe('/');
    expect(lookups).toHaveBeenLastCalledWith('a'.repeat(64));
  });
});
