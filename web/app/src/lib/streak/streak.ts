/** The streak view `GET /api/streak` answers (SPEC-076 R20), as the screen reads it. */
export type StreakTrack = {
  current: number;
  longest: number;
  heat: number;
  freezes?: number;
  freezeCap?: number;
};

/** What is at stake if the open study day ends with no study review. */
export type AtStake = 'freeze' | 'break' | 'none';

/** The governor `GET /api/governor` answers (SPEC-076 R21). */
export type GovernorView = {
  verdict: 'armed' | 'standby' | 'lapse';
  strength: number;
  lapseSince: string | null;
  /** The reason code for a disarmed governor, or null when it is armed. */
  why: string | null;
  /** The reviews that relight a lapse, present during one. */
  relightCards: number | null;
};

/** Both tracks, the governor and the at-stake reading the screen shows. */
export type StreakView = {
  studyDay: string;
  language: StreakTrack;
  law: StreakTrack;
  atStake: { language: AtStake; law: AtStake };
  governor: GovernorView;
};

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/** The streak view without its governor: what `GET /api/streak` answers. */
export type StreakBody = Omit<StreakView, 'governor'>;

const AT_STAKE: readonly string[] = ['freeze', 'break', 'none'];

/** A track's counts, or undefined when `value` is not one. `withFreezes` names the language track. */
function track(value: unknown, withFreezes: boolean): StreakTrack | undefined {
  const given = (value ?? {}) as Record<string, unknown>;
  const { current, longest, heat, freezes, freeze_cap: freezeCap } = given;
  if (![current, longest, heat].every((n) => typeof n === 'number')) return undefined;
  const counts = { current: current as number, longest: longest as number, heat: heat as number };
  if (!withFreezes) return counts;
  if (typeof freezes !== 'number' || typeof freezeCap !== 'number') return undefined;
  return { ...counts, freezes, freezeCap };
}

/** The body of `GET /api/streak`, or null when it is not one. */
export function parseStreak(body: unknown): StreakBody | null {
  const given = (body ?? {}) as Record<string, unknown>;
  const { study_day: studyDay, language, law, at_stake: atStake } = given;
  if (typeof studyDay !== 'string' || !ISO_DATE.test(studyDay)) return null;
  const languageTrack = track(language, true);
  const lawTrack = track(law, false);
  const stake = (atStake ?? {}) as Record<string, unknown>;
  if (languageTrack === undefined || lawTrack === undefined) return null;
  if (!AT_STAKE.includes(String(stake.language)) || !AT_STAKE.includes(String(stake.law))) return null;
  return {
    studyDay,
    language: languageTrack,
    law: lawTrack,
    atStake: { language: stake.language as AtStake, law: stake.law as AtStake }
  };
}

/** The body of `GET /api/governor`, or null when it is not one. */
export function parseGovernor(body: unknown): GovernorView | null {
  const given = (body ?? {}) as Record<string, unknown>;
  const { verdict, strength, lapse_since: lapseSince, relight_cards: relightCards } = given;
  if (verdict !== 'armed' && verdict !== 'standby' && verdict !== 'lapse') return null;
  if (typeof strength !== 'number') return null;
  if (lapseSince !== null && (typeof lapseSince !== 'string' || !ISO_DATE.test(lapseSince))) return null;
  if (relightCards !== null && typeof relightCards !== 'number') return null;
  const why =
    verdict === 'standby' ? 'strength_below_arming_level' : verdict === 'lapse' ? 'no_study_days' : null;
  return { verdict, strength, lapseSince, why, relightCards };
}
