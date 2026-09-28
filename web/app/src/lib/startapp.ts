import { TODAY, type RoutePath } from './routes';

/**
 * The startapp tokens and the screen each opens (SPEC-028 R3; ADR-028). A token is text anyone
 * can write into a launch link, so the table is closed: a token opens a screen only when it is
 * listed here, and it never becomes a path of its own. A new token names a screen of the route
 * table (`startapp.test.ts` holds every destination to it).
 */
const TOKENS: ReadonlyMap<string, RoutePath> = new Map<string, RoutePath>([
  ['today', '/'],
  ['about', '/about'],
  ['score', '/score']
]);

// Telegram admits up to 512 characters of A-Z, a-z, 0-9, underscore and hyphen. DeckStreak's
// tokens are short words, so a longer token, or one holding anything else, is none of them.
const SHAPE = /^[A-Za-z0-9_-]{1,64}$/;

/** The screen a startapp token opens: the table's screen for a listed token, and Today for any other. */
export function routeFor(token: string | null | undefined): RoutePath {
  if (typeof token !== 'string' || !SHAPE.test(token)) return TODAY;
  return TOKENS.get(token) ?? TODAY;
}
