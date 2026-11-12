import { describe, expect, it } from 'vitest';
import { parseGovernor, parseStreak, type AtStake } from './streak';

// SPEC-076 R20, R21. The streak screen reads `GET /api/streak` and `GET /api/governor` by exact
// key: a body that is not the view is refused whole. Each rule below is one class over a generated
// population, and each population's size is asserted so a shrunken one fails.
const STUDY_DAY = '2001-02-03';
const AT_STAKE: readonly AtStake[] = ['freeze', 'break', 'none'];

/** A well-formed `GET /api/streak` body. */
function streakBody(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    study_day: STUDY_DAY,
    language: { current: 12, longest: 20, heat: 3, freezes: 2, freeze_cap: 3 },
    law: { current: 4, longest: 6, heat: 1 },
    at_stake: { language: 'freeze', law: 'break' },
    ...overrides
  };
}

/** A well-formed `GET /api/governor` body. */
function governorBody(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return { verdict: 'armed', strength: 0.75, lapse_since: null, relight_cards: null, ...overrides };
}

const DIGIT_POSITIONS = [0, 1, 2, 3, 5, 6, 8, 9];
const DASH_POSITIONS = [4, 7];

/** ISO dates every digit 0-9 reaches at every digit position, and their shape. */
function validDates(): string[] {
  const dates: string[] = [];
  for (let digit = 0; digit < 10; digit += 1) {
    dates.push(`${digit}${digit}${digit}${digit}-${digit}${digit}-${digit}${digit}`);
  }
  for (const position of DIGIT_POSITIONS) {
    for (let digit = 0; digit < 10; digit += 1) {
      dates.push(STUDY_DAY.slice(0, position) + String(digit) + STUDY_DAY.slice(position + 1));
    }
  }
  return dates;
}

/** Strings one edit away from an ISO date: a non-digit in a digit place, a digit for a dash, length +-1, junk. */
function invalidDates(): string[] {
  const dates: string[] = [];
  for (const position of DIGIT_POSITIONS) {
    for (const junk of ['x', '-', ' ', '/']) {
      dates.push(STUDY_DAY.slice(0, position) + junk + STUDY_DAY.slice(position + 1));
    }
  }
  for (const position of DASH_POSITIONS) {
    for (const junk of ['0', '/', ' ', 'x']) {
      dates.push(STUDY_DAY.slice(0, position) + junk + STUDY_DAY.slice(position + 1));
    }
  }
  for (let index = 0; index < STUDY_DAY.length; index += 1) {
    dates.push(STUDY_DAY.slice(0, index) + STUDY_DAY.slice(index + 1));
    dates.push(STUDY_DAY.slice(0, index) + '7' + STUDY_DAY.slice(index));
  }
  for (const junk of ['x', ' ', '\n', '2001-02-03']) {
    dates.push(junk + STUDY_DAY, STUDY_DAY + junk);
  }
  dates.push('', '2001-2-3', '20010203', 'today');
  return dates;
}

describe('the ISO date rule of the streak bodies', () => {
  it('admits every generated valid date, for the study day and the lapse day alike', () => {
    const dates = validDates();
    expect(dates.length, 'valid dates generated').toBe(90);
    for (const date of dates) {
      expect(parseStreak(streakBody({ study_day: date }))?.studyDay, date).toBe(date);
      expect(parseGovernor(governorBody({ verdict: 'lapse', lapse_since: date }))?.lapseSince, date).toBe(
        date
      );
    }
  });

  it('refuses every generated invalid date, for the study day and the lapse day alike', () => {
    const dates = invalidDates();
    expect(dates.length, 'invalid dates generated').toBe(72);
    for (const date of dates) {
      expect(parseStreak(streakBody({ study_day: date })), date).toBeNull();
      expect(parseGovernor(governorBody({ lapse_since: date })), date).toBeNull();
    }
  });
});

describe('a date that only reads as one', () => {
  it('is refused when it is not a string, though its text would pass the date rule', () => {
    const lookalikes: unknown[] = [[STUDY_DAY], { toString: () => STUDY_DAY }, [[STUDY_DAY]]];
    expect(lookalikes.length, 'lookalike dates').toBe(3);
    for (const date of lookalikes) {
      expect(parseStreak(streakBody({ study_day: date }))).toBeNull();
      expect(parseGovernor(governorBody({ verdict: 'lapse', lapse_since: date }))).toBeNull();
    }
  });
});

describe('the streak body reading', () => {
  it('reads the body into the view, camel-cased, with the law track carrying no freezes', () => {
    expect(parseStreak(streakBody())).toStrictEqual({
      studyDay: STUDY_DAY,
      language: { current: 12, longest: 20, heat: 3, freezes: 2, freezeCap: 3 },
      law: { current: 4, longest: 6, heat: 1 },
      atStake: { language: 'freeze', law: 'break' }
    });
  });

  it('drops freezes a law track was sent, and keeps zero counts', () => {
    const body = streakBody({
      language: { current: 0, longest: 0, heat: 0, freezes: 0, freeze_cap: 0 },
      law: { current: 0, longest: 0, heat: 0, freezes: 5, freeze_cap: 9 }
    });
    expect(parseStreak(body)).toStrictEqual({
      studyDay: STUDY_DAY,
      language: { current: 0, longest: 0, heat: 0, freezes: 0, freezeCap: 0 },
      law: { current: 0, longest: 0, heat: 0 },
      atStake: { language: 'freeze', law: 'break' }
    });
  });

  it('admits every pair of at-stake values and echoes each side to its own track', () => {
    let pairs = 0;
    for (const language of AT_STAKE) {
      for (const law of AT_STAKE) {
        pairs += 1;
        expect(parseStreak(streakBody({ at_stake: { language, law } }))?.atStake).toStrictEqual({
          language,
          law
        });
      }
    }
    expect(pairs, 'at-stake pairs').toBe(9);
  });

  it('refuses an at-stake value outside the three, on either side', () => {
    const others: unknown[] = ['Freeze', 'BREAK', ' none', 'none ', '', 'lose', null, undefined, 0, true, []];
    let refused = 0;
    for (const other of others) {
      refused += 2;
      expect(parseStreak(streakBody({ at_stake: { language: other, law: 'none' } })), String(other)).toBeNull();
      expect(parseStreak(streakBody({ at_stake: { language: 'none', law: other } })), String(other)).toBeNull();
    }
    expect(refused, 'refused at-stake bodies').toBe(22);
    expect(parseStreak(streakBody({ at_stake: undefined }))).toBeNull();
    expect(parseStreak(streakBody({ at_stake: null }))).toBeNull();
    expect(parseStreak(streakBody({ at_stake: {} }))).toBeNull();
  });

  it('refuses each count of each track that is missing or not a number, one at a time', () => {
    const wrong: unknown[] = ['3', null, undefined, true, {}, [3]];
    let refused = 0;
    for (const field of ['current', 'longest', 'heat']) {
      for (const bad of wrong) {
        refused += 2;
        const language = { current: 1, longest: 2, heat: 3, freezes: 1, freeze_cap: 2, [field]: bad };
        const law = { current: 1, longest: 2, heat: 3, [field]: bad };
        expect(parseStreak(streakBody({ language })), `language ${field} ${String(bad)}`).toBeNull();
        expect(parseStreak(streakBody({ law })), `law ${field} ${String(bad)}`).toBeNull();
      }
    }
    expect(refused, 'refused count bodies').toBe(36);
  });

  it('refuses a language track whose freezes or cap is missing or not a number', () => {
    const wrong: unknown[] = ['3', null, undefined, true, {}];
    let refused = 0;
    for (const field of ['freezes', 'freeze_cap']) {
      for (const bad of wrong) {
        refused += 1;
        const language = { current: 1, longest: 2, heat: 3, freezes: 1, freeze_cap: 2, [field]: bad };
        expect(parseStreak(streakBody({ language })), `${field} ${String(bad)}`).toBeNull();
      }
    }
    expect(refused, 'refused freeze bodies').toBe(10);
  });

  it('refuses a body with no study day, tracks or non-object shape', () => {
    const bodies: unknown[] = [
      null,
      undefined,
      {},
      'streak',
      7,
      [],
      streakBody({ study_day: undefined }),
      streakBody({ study_day: 20010203 }),
      streakBody({ study_day: null }),
      streakBody({ language: undefined }),
      streakBody({ language: null }),
      streakBody({ language: {} }),
      streakBody({ law: undefined }),
      streakBody({ law: null }),
      streakBody({ law: {} })
    ];
    expect(bodies.length, 'refused bodies').toBe(15);
    for (const body of bodies) expect(parseStreak(body), JSON.stringify(body)).toBeNull();
  });
});

describe('the governor body reading', () => {
  it('names the reason of each verdict: none armed, a weak habit on standby, no study days in a lapse', () => {
    expect(parseGovernor(governorBody({ verdict: 'armed' }))).toStrictEqual({
      verdict: 'armed',
      strength: 0.75,
      lapseSince: null,
      why: null,
      relightCards: null
    });
    expect(parseGovernor(governorBody({ verdict: 'standby' }))).toStrictEqual({
      verdict: 'standby',
      strength: 0.75,
      lapseSince: null,
      why: 'strength_below_arming_level',
      relightCards: null
    });
    expect(
      parseGovernor(governorBody({ verdict: 'lapse', lapse_since: '2001-02-01', relight_cards: 12 }))
    ).toStrictEqual({
      verdict: 'lapse',
      strength: 0.75,
      lapseSince: '2001-02-01',
      why: 'no_study_days',
      relightCards: 12
    });
  });

  it('keeps zero strength and zero relight cards', () => {
    expect(parseGovernor(governorBody({ strength: 0, relight_cards: 0 }))).toStrictEqual({
      verdict: 'armed',
      strength: 0,
      lapseSince: null,
      why: null,
      relightCards: 0
    });
  });

  it('refuses a verdict outside the three', () => {
    const verdicts: unknown[] = ['Armed', 'armed ', '', 'lapsed', 'off', null, undefined, 1, true];
    expect(verdicts.length, 'refused verdicts').toBe(9);
    for (const verdict of verdicts) expect(parseGovernor(governorBody({ verdict })), String(verdict)).toBeNull();
  });

  it('refuses a strength, lapse day or relight count of the wrong type or absent', () => {
    const wrong: unknown[] = ['1', null, undefined, true, {}];
    for (const bad of wrong) {
      expect(parseGovernor(governorBody({ strength: bad })), `strength ${String(bad)}`).toBeNull();
    }
    for (const bad of ['1', 20010203, undefined, true, {}]) {
      expect(parseGovernor(governorBody({ lapse_since: bad })), `lapse ${String(bad)}`).toBeNull();
    }
    for (const bad of ['1', undefined, true, {}]) {
      expect(parseGovernor(governorBody({ relight_cards: bad })), `relight ${String(bad)}`).toBeNull();
    }
    expect(parseGovernor(governorBody()), 'the same body with every field valid').toEqual({
      verdict: 'armed',
      strength: 0.75,
      lapseSince: null,
      why: null,
      relightCards: null
    });
  });

  it('refuses a body that is not an object, or that lacks any key', () => {
    const bodies: unknown[] = [null, undefined, {}, 'armed', 3, []];
    for (const body of bodies) expect(parseGovernor(body), String(body)).toBeNull();
    for (const key of ['verdict', 'strength', 'lapse_since', 'relight_cards']) {
      const body = governorBody();
      delete body[key];
      expect(parseGovernor(body), key).toBeNull();
    }
    expect(parseGovernor(governorBody()), 'the same body with every field valid').toEqual({
      verdict: 'armed',
      strength: 0.75,
      lapseSince: null,
      why: null,
      relightCards: null
    });
  });
});
