<script lang="ts">
  // SPEC-350 R6, R10, R11; ADR-361. The deck list (red stub: the loading text alone).
  import type { Deck } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';

  /** What the deck list asks of the engine: the deck tree, and the deck to study. */
  interface DeckClient {
    decks(): Promise<Deck[]>;
    study(deck: bigint): Promise<null>;
  }

  let { client, onopen }: { client: () => Promise<DeckClient>; onopen: () => void } = $props();
  void client;
  void onopen;
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <p>{m.loading()}</p>
</main>
