import { describe, expect, it } from 'vitest';
import { parseExchange } from './exchange';

// SPEC-075 R7, R9, #80. The level screen reads `GET /api/xp/exchange` as the server answers it. An
// undefined rate is JSON null beside `rate_defined` false, and the reading keeps it null; a null
// rate the server calls defined, or a number it calls undefined, is a malformed body, refused whole.
const DEFINED = { source: 'reviews', total_xp: 10, graduated_cards: 4, rate: 2.5, rate_defined: true };
const UNDEFINED = { source: 'focus', total_xp: 50, graduated_cards: 0, rate: null, rate_defined: false };
const WINDOW = { first: '2025-01-13', last: '2025-01-14' };
const BODY = { window: WINDOW, rates: [UNDEFINED, DEFINED] };

describe('parseExchange', () => {
  it('reads each bucket, an undefined rate as null', () => {
    expect(parseExchange(BODY)).toEqual({
      window: { first: '2025-01-13', last: '2025-01-14' },
      rates: [
        { source: 'focus', totalXp: 50, graduatedCards: 0, rate: null, rateDefined: false },
        { source: 'reviews', totalXp: 10, graduatedCards: 4, rate: 2.5, rateDefined: true }
      ]
    });
  });

  it('reads a readout over every day, and one with no bucket', () => {
    expect(parseExchange({ window: null, rates: [DEFINED] })?.window).toBeNull();
    expect(parseExchange({ window: null, rates: [] })).toEqual({ window: null, rates: [] });
  });

  it('refuses a rate that disagrees with rate_defined', () => {
    // The positive controls: each pair the server sends parses.
    expect(parseExchange({ window: null, rates: [DEFINED] })?.rates[0]?.rate).toBe(2.5);
    expect(parseExchange({ window: null, rates: [UNDEFINED] })?.rates[0]?.rate).toBeNull();
    const rates = [
      { ...DEFINED, rate: null },
      { ...UNDEFINED, rate: 0 },
      { ...DEFINED, rate_defined: false },
      { ...DEFINED, rate: '2.5' },
      { ...UNDEFINED, rate: undefined }
    ];
    expect(rates.map((rate) => parseExchange({ window: null, rates: [rate] }))).toEqual(
      rates.map(() => null)
    );
  });

  it('refuses each field that is missing or of the wrong type, one at a time', () => {
    const wrong: Record<string, unknown> = {
      source: 7,
      total_xp: '10',
      graduated_cards: '4',
      rate_defined: 1
    };
    const bodies: unknown[] = [];
    for (const key of Object.keys(wrong)) {
      bodies.push({ window: null, rates: [{ ...DEFINED, [key]: wrong[key] }] });
      const { [key]: _dropped, ...without } = DEFINED as Record<string, unknown>;
      bodies.push({ window: null, rates: [without] });
    }
    // A rate_defined that is not a boolean is refused even when the rate it stands beside is null.
    bodies.push({ window: null, rates: [{ ...UNDEFINED, rate_defined: 0 }] });
    const { rate_defined: _undefined, ...unmarked } = UNDEFINED;
    bodies.push({ window: null, rates: [unmarked] });
    expect(bodies.map(parseExchange), JSON.stringify(bodies)).toEqual(bodies.map(() => null));
  });

  it('refuses a body that is not the readout', () => {
    const refused = [
      null,
      'rates',
      {},
      { rates: [DEFINED] },
      { window: null },
      { window: null, rates: {} },
      { window: null, rates: [null] },
      { window: null, rates: [DEFINED, 'reviews'] },
      { window: 'all', rates: [DEFINED] },
      { window: {}, rates: [DEFINED] },
      { window: { first: '2025-01-13' }, rates: [DEFINED] },
      { window: { last: '2025-01-14' }, rates: [DEFINED] },
      { window: { ...WINDOW, first: 20250113 }, rates: [DEFINED] },
      { window: { ...WINDOW, last: ['2025-01-14'] }, rates: [DEFINED] },
      { window: { ...WINDOW, first: ['2025-01-13'] }, rates: [DEFINED] }
    ];
    expect(refused.map(parseExchange), JSON.stringify(refused)).toEqual(refused.map(() => null));
  });

  it('refuses a window day that is anything but four, two and two digits', () => {
    const days = ['2025-01-1', '2025-1-13', 'x2025-01-13', '2025-01-13x', '2025-01-13\n', '25-01-13'];
    const bodies = days.flatMap((day) => [
      { window: { ...WINDOW, first: day }, rates: [] },
      { window: { ...WINDOW, last: day }, rates: [] }
    ]);
    expect(bodies.map(parseExchange)).toEqual(bodies.map(() => null));
  });
});
