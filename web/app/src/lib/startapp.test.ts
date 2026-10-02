import { describe, expect, it } from 'vitest';
import { isToken, routeFor } from './startapp';
import { ROUTES, TODAY } from './routes';

// SPEC-028 R3, A2, A3; ADR-028. A startapp token is text anyone can put in a link, so it opens a
// screen only through a closed table, and everything else opens Today: never a 404, and never a
// navigation to the token's own text.
describe('the startapp token map', () => {
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

  it('every destination is a screen of the route table', () => {
    const destinations = ['today', 'about', 'insights', 'score', 'level', 'streak', 'badges', 'records']
      .map(routeFor)
      .sort();

    expect(destinations).toEqual([...ROUTES].sort());
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
