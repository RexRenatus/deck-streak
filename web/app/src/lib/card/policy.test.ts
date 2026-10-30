import { describe, expect, it } from 'vitest';
import { FRAME_POLICY, FRAME_SANDBOX, HOST_POLICY } from './policy.js';

// SPEC-341 R2, R6, A1; ADR-352 D1, D3. The card frame runs no script (a sandbox with no token) and
// its own policy fetches nothing but `data:` images, media and fonts: no script, connection, frame,
// object, form target or base, whatever the card's markup asks for.

/** A policy's directives by name, each with its sources in order. */
function directives(policy: string): Record<string, string[]> {
  const map: Record<string, string[]> = {};
  for (const directive of policy.split(';')) {
    const [name, ...sources] = directive.trim().split(/\s+/);
    if (name !== '') map[name] = sources;
  }
  return map;
}

/** What a card frame may load, by directive; every directive this does not name admits nothing. */
const ADMITTED: Readonly<Record<string, readonly string[]>> = {
  'img-src': ['data:'],
  'media-src': ['data:'],
  'font-src': ['data:'],
  'style-src': ["'unsafe-inline'"]
};

/** Each directive of `policy` that admits more than a card frame may, and a missing default. */
function weaker(policy: string): string[] {
  const parsed = directives(policy);
  const found = Object.entries(parsed)
    .filter(([name, sources]) => sources.some((source) => !(ADMITTED[name] ?? ["'none'"]).includes(source)))
    .map(([name, sources]) => `${name} ${sources.join(' ')}`);
  return 'default-src' in parsed ? found : ['default-src missing', ...found];
}

describe('the card frame policy', () => {
  it("the card frame's policy fetches only data and runs no script", () => {
    expect(directives(FRAME_POLICY)).toEqual({
      'default-src': ["'none'"],
      'img-src': ['data:'],
      'media-src': ['data:'],
      'font-src': ['data:'],
      'style-src': ["'unsafe-inline'"],
      'form-action': ["'none'"],
      'base-uri': ["'none'"]
    });
    // the frame is sandboxed with no token: an opaque origin, no script, form, popup or navigation
    expect(FRAME_SANDBOX).toBe('');

    // and the judgement finds nothing weaker in it, where it names each weaker directive planted
    expect(weaker(FRAME_POLICY)).toEqual([]);
    expect(
      weaker(
        "default-src 'none'; script-src 'self'; img-src *; connect-src https:; frame-src *; style-src 'unsafe-inline'"
      )
    ).toEqual(["script-src 'self'", 'img-src *', 'connect-src https:', 'frame-src *']);
    expect(weaker('img-src data:')).toEqual(['default-src missing']);
  });

  // SPEC-407 R2; ADR-421 D3. The host policy adds nothing the frame policy does not enforce.
  it('the host policy holds only what the frame policy already holds, and no frame source', () => {
    const host = directives(HOST_POLICY);
    const frame = directives(FRAME_POLICY);

    expect(Object.keys(host)).toEqual(['img-src', 'script-src', 'object-src', 'base-uri']);
    expect(host['img-src']).toEqual(frame['img-src']);
    expect(host['base-uri']).toEqual(frame['base-uri']);
    expect(host['script-src']).toEqual(["'none'"]);
    expect(host['object-src']).toEqual(["'none'"]);
    expect(frame['default-src']).toEqual(["'none'"]);
    for (const name of ['default-src', 'frame-src', 'child-src']) expect(host[name]).toBeUndefined();
  });
});
