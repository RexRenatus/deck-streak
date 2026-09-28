import { describe, expect, it } from 'vitest';
import { PILLARS, parseScore } from './score';

// SPEC-071 R20, R22. The score screen reads `GET /api/score`'s body: the current study day, as an
// ISO date, and its score, or null while the day has none. A body of any other shape is refused
// whole, one defect at a time, so the screen never shows a number the server did not send; and
// only the retention pillar may be absent.
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** A body as the server answers it, with `retention` as its retention pillar. */
function body(retention: number | null = 85): Record<string, unknown> {
  return {
    study_day: '2001-02-03',
    score: {
      total: 72,
      grade: { label: 'SOLID', emoji: '✅' },
      pillars: { consistency: 76, retention, workload: 70, volume: 60.5, mastery: 55 },
      reviews: 40,
      retention
    }
  };
}

/** `body()` with `change` applied to a deep copy of it. */
function defect(change: (copy: Record<string, any>) => void): unknown {
  const copy = structuredClone(body()) as Record<string, any>;
  change(copy);
  return copy;
}

describe('the score body', () => {
  it('reads the study day, the total, the grade and every pillar', () => {
    expect(parseScore(body())).toEqual({
      studyDay: '2001-02-03',
      score: {
        total: 72,
        grade: { label: 'SOLID', emoji: '✅' },
        pillars: { consistency: 76, retention: 85, workload: 70, volume: 60.5, mastery: 55 }
      }
    });
    expect(PILLARS).toEqual(['consistency', 'retention', 'workload', 'volume', 'mastery']);
  });

  it('reads an absent retention as null, and a day with no score as no score', () => {
    expect(parseScore(body(null))?.score?.pillars.retention).toBeNull();
    expect(parseScore({ study_day: '2001-02-03', score: null })).toEqual({
      studyDay: '2001-02-03',
      score: null
    });
  });

  it('refuses a body of any other shape', () => {
    const refused: [string, unknown][] = [
      ['null', null],
      ['a string', 'score'],
      ['a number', 72],
      ['no study day', defect((copy) => delete copy.study_day)],
      ['a numeric study day', defect((copy) => (copy.study_day = 20102))],
      ...[
        'x2001-02-03',
        '2001-02-03x',
        '201-02-03',
        '20011-02-03',
        '2001-2-03',
        '2001-002-03',
        '2001-02-3',
        '2001-02-003',
        '2001/02/03',
        'abcd-ef-gh'
      ].map((day): [string, unknown] => [`the study day ${day}`, defect((copy) => (copy.study_day = day))]),
      ['no score', defect((copy) => delete copy.score)],
      ['a score that is a string', defect((copy) => (copy.score = 'high'))],
      ['no total', defect((copy) => delete copy.score.total)],
      ['a total in text', defect((copy) => (copy.score.total = '72'))],
      ['no grade', defect((copy) => delete copy.score.grade)],
      ['a grade in text', defect((copy) => (copy.score.grade = 'SOLID'))],
      ['a numeric label', defect((copy) => (copy.score.grade.label = 1))],
      ['no emoji', defect((copy) => delete copy.score.grade.emoji)],
      ['no pillars', defect((copy) => delete copy.score.pillars)],
      ['pillars in text', defect((copy) => (copy.score.pillars = 'none'))],
      ...PILLARS.map((pillar): [string, unknown] => [
        `no ${pillar}`,
        defect((copy) => delete copy.score.pillars[pillar])
      ]),
      ...PILLARS.map((pillar): [string, unknown] => [
        `the ${pillar} in text`,
        defect((copy) => (copy.score.pillars[pillar] = '50'))
      ]),
      ...PILLARS.filter((pillar) => pillar !== 'retention').map((pillar): [string, unknown] => [
        `an absent ${pillar}`,
        defect((copy) => (copy.score.pillars[pillar] = null))
      ])
    ];
    for (const [name, candidate] of examined('refused bodies', refused)) {
      expect(parseScore(candidate), name).toBeNull();
    }
  });
});
