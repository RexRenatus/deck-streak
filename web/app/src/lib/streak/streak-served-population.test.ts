import { describe, expect, it } from 'vitest';
import { parseGovernor, parseStreak, type AtStake } from './streak';

// SPEC-076 A48, the served-pairs rule on the reader: every value the screen reads from
// `GET /api/streak` and `GET /api/governor` is read from a generated population in which each pair
// of values differs in some body, so a reader that takes one value in another's place is red. The
// population is A47's: the bodies the API test proves the routes serve, built from the same runs,
// freezes, studied days, lapse, standby, anchors and strengths.
const RUNS = [0, 1, 7, 30, 100, 365];
const HEAT_THRESHOLDS = [1, 7, 30, 100, 365];
const FREEZE_CAP = 3;
const RELIGHT_CARDS = 3;
const STUDY_DAY = '2025-01-14';

/** The heat tier R5 names for a run: how many thresholds it has reached. */
function tier(run: number): number {
  return HEAT_THRESHOLDS.filter((threshold) => run >= threshold).length;
}

/** What R20 says a missed day costs a track. */
function stake(current: number, freezes: number, studiedToday: boolean): AtStake {
  if (studiedToday || current === 0) return 'none';
  return freezes > 0 ? 'freeze' : 'break';
}

/** The lapse anchor of member `k`, as the governor route serves it. */
function anchor(k: number): string {
  return `2025-01-${String(10 - (k % 3)).padStart(2, '0')}`;
}

type Member = { streak: Record<string, unknown>; governor: Record<string, unknown> };

/** A47's population: 36 members, each a streak body and a governor body the routes serve. */
function population(): Member[] {
  const members: Member[] = [];
  RUNS.forEach((language, i) => {
    RUNS.forEach((law, j) => {
      const k = 6 * i + j;
      const freezes = k % 4;
      const lapse = k % 4 >= 2;
      const standby = k % 2 === 1;
      members.push({
        streak: {
          study_day: STUDY_DAY,
          language: {
            current: language,
            longest: language + 1 + j,
            heat: tier(language),
            freezes,
            freeze_cap: FREEZE_CAP
          },
          law: { current: law, longest: law + 2 + i, heat: tier(law) },
          at_stake: { language: stake(language, freezes, k % 3 === 0), law: stake(law, 0, k % 3 === 1) }
        },
        governor: {
          verdict: lapse ? 'lapse' : standby ? 'standby' : 'armed',
          strength: (k % 7) / 8,
          standby,
          lapse,
          lapse_since: lapse ? anchor(k) : null,
          relight_cards: lapse ? RELIGHT_CARDS : null
        }
      });
    });
  });
  return members;
}

/** Every leaf of `value` the reader takes, named by its path. */
function leaves(value: unknown, prefix = ''): Map<string, unknown> {
  const out = new Map<string, unknown>();
  if (value !== null && typeof value === 'object') {
    for (const [key, field] of Object.entries(value)) {
      const path = prefix === '' ? key : `${prefix}.${key}`;
      for (const [leaf, held] of leaves(field, path)) out.set(leaf, held);
    }
  } else {
    out.set(prefix, value);
  }
  return out;
}

/** The pairs of leaves no member separates, and how many it does. */
function unseparated(bodies: Map<string, unknown>[]): { missed: string[]; separated: number } {
  const names = [...(bodies[0] ?? new Map<string, unknown>()).keys()];
  const missed: string[] = [];
  let separated = 0;
  names.forEach((first, a) => {
    for (const second of names.slice(a + 1)) {
      if (bodies.some((body) => body.get(first) !== body.get(second))) separated += 1;
      else missed.push(`${first} and ${second}`);
    }
  });
  return { missed, separated };
}

describe('the served-pairs rule on the reader', () => {
  const members = population();
  const governorRead = (body: Record<string, unknown>): Record<string, unknown> => ({
    verdict: body.verdict,
    strength: body.strength,
    lapse_since: body.lapse_since,
    relight_cards: body.relight_cards
  });

  it('separates every pair of values the reader takes, in some member', () => {
    expect(members).toHaveLength(36);
    const streak = unseparated(members.map((member) => leaves(member.streak)));
    const governor = unseparated(members.map((member) => leaves(governorRead(member.governor))));
    expect(streak.missed).toEqual([]);
    expect(governor.missed).toEqual([]);
    expect([streak.separated, governor.separated]).toEqual([55, 6]);
  });

  it('reads each served value of the streak into its own place, for every member', () => {
    let read = 0;
    for (const { streak } of members) {
      const language = streak.language as Record<string, number>;
      const law = streak.law as Record<string, number>;
      const atStake = streak.at_stake as Record<string, AtStake>;
      expect(parseStreak(streak), JSON.stringify(streak)).toStrictEqual({
        studyDay: STUDY_DAY,
        language: {
          current: language.current,
          longest: language.longest,
          heat: language.heat,
          freezes: language.freezes,
          freezeCap: language.freeze_cap
        },
        law: { current: law.current, longest: law.longest, heat: law.heat },
        atStake: { language: atStake.language, law: atStake.law }
      });
      read += 1;
    }
    expect(read).toBe(36);
  });

  it('reads each served value of the governor into its own place, for every member', () => {
    const why: Record<string, string | null> = {
      armed: null,
      standby: 'strength_below_arming_level',
      lapse: 'no_study_days'
    };
    let read = 0;
    for (const { governor } of members) {
      expect(parseGovernor(governor), JSON.stringify(governor)).toStrictEqual({
        verdict: governor.verdict,
        strength: governor.strength,
        lapseSince: governor.lapse_since,
        why: why[String(governor.verdict)],
        relightCards: governor.relight_cards
      });
      read += 1;
    }
    expect(read).toBe(36);
  });
});
