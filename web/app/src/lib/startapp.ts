import { TODAY, type RoutePath } from './routes';

/**
 * The startapp tokens and the screen each opens (SPEC-028 R3; ADR-028). A token is text anyone
 * can write into a launch link, so the table is closed: a token opens a screen only when it is
 * listed here, and it never becomes a path of its own. A new token names a screen of the route
 * table (`startapp.test.ts` holds every destination to it).
 */
const TOKENS: ReadonlyMap<string, RoutePath> = new Map<string, RoutePath>([
  ['today', TODAY],
  ['about', '/about'],
  ['insights', '/insights'],
  ['score', '/score'],
  ['level', '/level'],
  ['streak', '/streak']
]);

// Telegram admits up to 512 characters of A-Z, a-z, 0-9, underscore and hyphen. DeckStreak's
// tokens are short words, so a longer token, or one holding anything else, is none of them.
const SHAPE = /^[A-Za-z0-9_-]{1,64}$/;

/**
 * Whether `token` has a token's shape. The table is read by exact key, so no screen depends on
 * this: it keeps text that is no token's shape from reaching the table at all, and its own tests
 * pin every part of the shape.
 */
export function isToken(token: string | null | undefined): token is string {
  return typeof token === 'string' && SHAPE.test(token);
}

/** The screen a startapp token opens: the table's screen for a listed token, and Today for any other. */
export function routeFor(token: string | null | undefined): RoutePath {
  return isToken(token) ? (TOKENS.get(token) ?? TODAY) : TODAY;
}
