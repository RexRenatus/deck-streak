// The recorded exceptions to the phone-size audit (SPEC-639 R5; ADR-662). This table is the only
// place an axe rule is turned off for the verification: each entry names its rule, the route it is
// off on, the reason, and the issue that tracks the fix. It starts empty.

/** One rule turned off on one route, with its reason and its issue. */
export interface RecordedException {
  readonly rule: string;
  readonly route: string;
  readonly reason: string;
  readonly issue: string;
}

export const EXCEPTIONS: readonly RecordedException[] = [];

/** The rules the table turns off on `route`. */
export function disabledFor(route: string): string[] {
  return EXCEPTIONS.filter((entry) => entry.route === route).map((entry) => entry.rule);
}
