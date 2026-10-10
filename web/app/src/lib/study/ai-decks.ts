// SPEC-381 R7, R8, R11; ADR-392 D1, D3. The rules of the AI-and-your-decks screen, apart from its
// markup: the switch each deck shows, the marked deck above it that keeps it away, and a change
// saved at once and turned back when the server did not save it. The marks live on the server
// alone, so this module keeps none of its own.
import type { Answer } from '$lib/api';
import type { Deck } from '$lib/engine/protocol';

/** What the screen asks of the server: the marked decks, and one deck's change. */
export interface MarksClient {
  /** The marked decks' ids, as decimal strings. */
  sensitiveDecks(): Promise<Answer<string[]>>;
  /** Marks or unmarks one deck, and answers the marked decks' ids. */
  setSensitive(id: string, sensitive: boolean): Promise<Answer<string[]>>;
}

/** One deck's switch, as the screen shows it. */
export interface Switch {
  /** The deck's id, as a decimal string. */
  readonly id: string;
  readonly name: string;
  /** On when the deck is marked, or when a deck above it is. */
  readonly on: boolean;
  /** The name of the nearest marked deck above this one, which keeps it away; null when none is. */
  readonly keptBy: string | null;
  readonly children: readonly Switch[];
}

/** The switch of every deck of `tree`, in its order, under the marked set `marked`. */
export function switches(tree: readonly Deck[], marked: ReadonlySet<string>): Switch[] {
  return branch(tree, marked, null);
}

/** The switches of `tree`, whose nearest marked deck above is named `keptBy`, or null. */
function branch(tree: readonly Deck[], marked: ReadonlySet<string>, keptBy: string | null): Switch[] {
  return tree.map((deck) => {
    const id = String(deck.id);
    const own = marked.has(id);
    return {
      id,
      name: deck.name,
      on: own || keptBy !== null,
      keptBy,
      children: branch(deck.children, marked, own ? deck.name : keptBy)
    };
  });
}

/** The marked set with the deck `id` turned on or off. */
export function turned(marked: ReadonlySet<string>, id: string, on: boolean): Set<string> {
  const next = new Set(marked);
  if (on) next.add(id);
  else next.delete(id);
  return next;
}

/**
 * Saves the deck `id`'s change through `client`: the marked set the server answered, or null when
 * the change was not saved.
 */
export async function save(client: MarksClient, id: string, on: boolean): Promise<ReadonlySet<string> | null> {
  const answer = await client.setSensitive(id, on);
  return answer.kind === 'ok' ? new Set(answer.value) : null;
}

/** Where the server answers the marked decks (SPEC-381 R7). */
export const SENSITIVE_DECKS_PATH = '/api/decks/sensitive';

/** Where the server marks or unmarks the deck `id`. */
export function sensitiveDeckPath(id: string): string {
  return `/api/decks/${encodeURIComponent(id)}/sensitive`;
}

/** The body of a change: the deck kept away from AI, or not. */
export function sensitiveBody(sensitive: boolean): string {
  return JSON.stringify({ sensitive });
}

/** A deck id as the server writes it: a positive decimal with no sign and no leading zero. */
const DECK_ID = /^[1-9][0-9]*$/;

/** The marked decks' ids from the server's `{"decks": [...]}`, or null when it is not one. */
export function parseMarked(body: unknown): string[] | null {
  if (body === null || typeof body !== 'object') return null;
  const decks = (body as Record<string, unknown>).decks;
  if (!Array.isArray(decks)) return null;
  return decks.every((id) => typeof id === 'string' && DECK_ID.test(id)) ? (decks as string[]) : null;
}
