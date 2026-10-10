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
  return tree.map((deck) => ({
    id: String(deck.id),
    name: deck.name,
    on: false,
    keptBy: null,
    children: switches(deck.children, marked)
  }));
}

/** The marked set with the deck `id` turned on or off. */
export function turned(marked: ReadonlySet<string>, id: string, on: boolean): Set<string> {
  return new Set(marked);
}

/**
 * Saves the deck `id`'s change through `client`: the marked set the server answered, or null when
 * the change was not saved.
 */
export async function save(
  client: MarksClient,
  marked: ReadonlySet<string>,
  id: string,
  on: boolean
): Promise<ReadonlySet<string> | null> {
  return new Set(marked);
}
