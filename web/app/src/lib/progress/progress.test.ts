import { describe, expect, it } from 'vitest';
import { parseProgress } from './progress';

// SPEC-077 R15, R17. The progress screen reads `GET /api/progress` as the route answers it, and
// refuses, whole, a body that is not that answer: a screen that drew half a ladder would state a
// band the owner has not reached. Mutation coverage of the reader, not an acceptance criterion.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

const CELL = { band: 'A1', total: 12, mature: 0, pct: 0, achieved: false };
const COURSE = {
  code: 'de',
  name: 'German',
  flag: '🇩🇪',
  mastery_pct: 100,
  current_band: 'A1',
  current_unit: 0,
  bands: [CELL, { band: 'A2', total: 0, mature: 0, pct: 100, achieved: true }]
};

describe('the progress reader', () => {
  it('reads each course with its band cells, at the edges of every bound', () => {
    expect(parseProgress({ courses: [COURSE, { ...COURSE, code: 'fr', current_unit: null }] })).toEqual({
      courses: [
        {
          code: 'de',
          name: 'German',
          flag: '🇩🇪',
          masteryPct: 100,
          currentBand: 'A1',
          currentUnit: 0,
          bands: [
            { band: 'A1', total: 12, mature: 0, pct: 0, achieved: false },
            { band: 'A2', total: 0, mature: 0, pct: 100, achieved: true }
          ]
        },
        {
          code: 'fr',
          name: 'German',
          flag: '🇩🇪',
          masteryPct: 100,
          currentBand: 'A1',
          currentUnit: null,
          bands: [
            { band: 'A1', total: 12, mature: 0, pct: 0, achieved: false },
            { band: 'A2', total: 0, mature: 0, pct: 100, achieved: true }
          ]
        }
      ]
    });
  });

  it('reads an owner with no course yet', () => {
    expect(parseProgress({ courses: [] })).toEqual({ courses: [] });
  });

  it('refuses a body that is not the progress answer, whole', () => {
    // a positive control: the body every case below spoils one field of
    expect(parseProgress({ courses: [COURSE] })?.courses.map((read) => read.code)).toEqual(['de']);

    const bodies: unknown[] = [null, undefined, 'courses', [], {}, { courses: null }, { courses: {} }];
    const course: [string, unknown][] = [
      ['code', 7],
      ['name', null],
      ['flag', undefined],
      ['mastery_pct', '50'],
      ['mastery_pct', null],
      ['mastery_pct', -0.5],
      ['mastery_pct', 100.5],
      ['current_band', 1],
      ['current_unit', '3'],
      ['current_unit', -1],
      ['current_unit', 2.5],
      ['current_unit', undefined],
      ['bands', null],
      ['bands', {}]
    ];
    const cell: [string, unknown][] = [
      ['band', null],
      ['total', -1],
      ['total', 1.5],
      ['total', '12'],
      ['mature', -1],
      ['mature', null],
      ['pct', -0.5],
      ['pct', 100.5],
      ['pct', '50'],
      ['pct', null],
      ['achieved', 'true'],
      ['achieved', 1]
    ];
    const spoiled = [
      ...bodies,
      { courses: [COURSE, null] },
      { courses: [COURSE, 'German'] },
      { courses: [{ ...COURSE, bands: [CELL, null] }] },
      ...course.map(([key, value]) => ({ courses: [COURSE, { ...COURSE, [key]: value }] })),
      ...cell.map(([key, value]) => ({ courses: [{ ...COURSE, bands: [CELL, { ...CELL, [key]: value }] }] }))
    ];
    for (const body of examined('spoiled bodies', spoiled)) {
      expect(parseProgress(body), JSON.stringify(body) ?? String(body)).toBeNull();
    }
  });
});
