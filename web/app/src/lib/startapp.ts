import { TODAY, type RoutePath } from './routes';

/**
 * The startapp tokens and the screen each opens (SPEC-028 R3; ADR-028). A token is text anyone
 * can write into a launch link, so the table is closed: a token opens a screen only when it is
 * listed here, and it never becomes a path of its own. A new token names a screen of the route
 * table (`startapp.test.ts` holds every destination to it).
 */
const TOKENS: ReadonlyMap<string, RoutePath> = new Map<string, RoutePath>([
  // Stryker disable next-line ArrayDeclaration,StringLiteral: EQUIVALENT: 'today' opens TODAY, which every unlisted token opens too, so no token's screen changes without this entry (#294)
  ['today', TODAY],
  ['about', '/about'],
  ['score', '/score']
]);

// Telegram admits up to 512 characters of A-Z, a-z, 0-9, underscore and hyphen. DeckStreak's
// tokens are short words, so a longer token, or one holding anything else, is none of them.
// Stryker disable next-line Regex: EQUIVALENT: every listed token matches SHAPE whole and the table is read by exact key, so an unanchored SHAPE opens no other screen (#294)
const SHAPE = /^[A-Za-z0-9_-]{1,64}$/;

/** The screen a startapp token opens: the table's screen for a listed token, and Today for any other. */
export function routeFor(token: string | null | undefined): RoutePath {
  // Stryker disable next-line ConditionalExpression,LogicalOperator: EQUIVALENT: every listed token is a string SHAPE matches, so a token this guard refuses misses the exact-key lookup below and opens TODAY anyway (#294)
  if (typeof token !== 'string' || !SHAPE.test(token)) return TODAY;
  return TOKENS.get(token) ?? TODAY;
}
