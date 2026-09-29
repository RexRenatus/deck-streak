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
  /** Why the governor is disarmed, or null when it is armed. */
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
