import { afterEach, describe, expect, it, vi } from 'vitest';
import { isToken, routeFor } from './startapp';
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
      // SPEC-071 §10: refused by the shape itself, before the table is read
      expect(isToken(token), `token ${JSON.stringify(token)}`).toBe(false);
    }
  });

  // SPEC-077 R16, R17, T35. The progress screen and the law tab are opened by their paths alone:
  // the progress command's button names the path, and no startapp token opens either screen. The
  // list is exact, so any other new screen still needs a token or a place here. SPEC-343 R16: the
  // remote harness is opened by its path in a browser tab, and no token opens it. SPEC-350 R6, R7:
  // the deck list and the review are opened by their paths, and no token opens either.
  // SPEC-350 R18: the mapping screen is opened by its path, from the review.
  // SPEC-377 R8: the sync screen is opened by its path, and no token opens it.
  // SPEC-381 R8: the AI-and-your-decks screen is opened by its path, from the deck list.
  const BY_PATH = ['/progress', '/law', '/remote', '/study', '/study/review', '/study/mapping', '/study/ai-decks', '/sync'];

  it('every destination is a screen of the route table', () => {
    const tokens = ['today', 'about', 'insights', 'score', 'level', 'streak', 'wallet', 'badges', 'records', 'capture'];
    const destinations = [...tokens.map(routeFor), ...BY_PATH].sort();

    expect(destinations).toEqual([...ROUTES].sort());
  });

  it('the progress screen and the law tab are opened by path, and no token opens either', () => {
    expect(routeFor('progress')).toBe(TODAY);
    expect(routeFor('law')).toBe(TODAY);
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

  // SPEC-118 R11; ruling (m). The quick capture is a screen of the route table, so a launch link
  // opens it through its own token, like every other screen.
  it('the capture token opens the quick capture screen', () => {
    expect(routeFor('capture')).toBe('/capture');
  });

  // SPEC-071 §10: the table is read by exact key, so no screen shows whether the shape
  // holds; these pin each part of it (the anchors, the characters, the length) directly.
  it('a token is one to 64 of A-Z, a-z, 0-9, underscore and hyphen, and nothing else is', () => {
    const tokens = ['today', 'about', 'insights', 'score', 'a', 'Z', '0', '_', '-', 'A_b-9', 'a'.repeat(64)];
    for (const token of tokens) {
      expect(isToken(token), `token ${JSON.stringify(token)}`).toBe(true);
    }
    const none = [null, undefined, '', '!today', 'today!', 'to day', 'today\n', 'a'.repeat(65)];
    for (const token of none) {
      expect(isToken(token), `token ${JSON.stringify(token)}`).toBe(false);
    }
  });

  // SPEC-071 §10: the tests above compare with TODAY itself, so they hold whatever it names.
  it('Today is the root screen, and an unknown token opens it', () => {
    expect(TODAY).toBe('/');
    expect(routeFor('settings')).toBe('/');
  });
});
