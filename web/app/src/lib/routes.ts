/**
 * The Mini App's route table (SPEC-028 R3, R10; ADR-028): every screen the app routes to.
 *
 * The startapp token map (`startapp.ts`) opens only a path listed here, and the rendered
 * accessibility audit (`tests/a11y.spec.ts`) audits every path listed here in both of Telegram's
 * colour schemes. `a11y-coverage.test.ts` holds this table equal to the screens under
 * `src/routes`, so a new screen is audited from the day it exists. The module imports nothing, so
 * Playwright loads it outside SvelteKit.
 */
export const ROUTES = [
  '/',
  '/about',
  '/insights',
  '/score',
  '/level',
  '/streak',
  '/wallet',
  '/badges',
  '/records',
  '/progress',
  '/law',
  '/capture',
  '/remote',
  '/study',
  '/study/review',
  '/study/mapping',
  '/sync'
] as const;

/** A path the route table lists. */
export type RoutePath = (typeof ROUTES)[number];

/** Today, the screen every launch without a known destination opens. */
export const TODAY: RoutePath = '/';
