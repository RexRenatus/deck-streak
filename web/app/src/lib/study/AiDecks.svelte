<script lang="ts">
  // SPEC-381 R8, R9; ADR-392 D3. The AI-and-your-decks screen: the engine's deck tree with one
  // native switch per deck, off unless the server holds the deck's mark. A deck under a marked deck
  // shows on, cannot be changed, and names the marked deck that keeps it away. Turning a switch
  // saves at once; a change the server did not save is turned back and announced, and a failed load
  // is announced with a way to ask again.
  import { onMount } from 'svelte';
  import type { Deck } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import { save, switches, turned, type MarksClient, type Switch } from './ai-decks';

  /** What the screen asks of the engine: the deck tree. */
  interface DeckClient {
    decks(): Promise<Deck[]>;
  }

  let { client, marks }: { client: () => Promise<DeckClient>; marks: MarksClient } = $props();

  let tree = $state.raw<Deck[] | null>(null);
  let marked = $state.raw<ReadonlySet<string>>(new Set());
  let failure = $state<'load' | 'save' | null>(null);
  const shown = $derived(tree === null ? [] : switches(tree, marked));

  async function load(): Promise<void> {
    failure = null;
    tree = null;
    try {
      const [decks, answer] = await Promise.all([
        client().then((engine) => engine.decks()),
        marks.sensitiveDecks()
      ]);
      if (answer.kind !== 'ok') {
        failure = 'load';
        return;
      }
      marked = new Set(answer.value);
      tree = decks;
    } catch {
      failure = 'load';
    }
  }

  async function turn(id: string, on: boolean): Promise<void> {
    failure = null;
    marked = turned(marked, id, on);
    const saved = await save(marks, marked, id, on);
    if (saved === null) {
      marked = turned(marked, id, !on);
      failure = 'save';
    } else {
      marked = saved;
    }
  }

  onMount(() => void load());
</script>

{#snippet branch(list: readonly Switch[])}
  <ul class="flex flex-col gap-2">
    {#each list as deck (deck.id)}
      <li class="flex flex-col gap-1 [&>ul]:pl-4">
        <label class="flex min-h-11 items-center gap-3 rounded-md border px-4 py-2">
          <input
            type="checkbox"
            role="switch"
            class="size-5 shrink-0"
            checked={deck.on}
            disabled={deck.keptBy !== null}
            aria-describedby={deck.keptBy === null ? undefined : `kept-by-${deck.id}`}
            onchange={(event) => void turn(deck.id, event.currentTarget.checked)}
          />
          <span>Keep {deck.name} away from AI</span>
        </label>
        {#if deck.keptBy !== null}
          <p id="kept-by-{deck.id}" class="px-4 text-sm">Kept away because {deck.keptBy} is.</p>
        {/if}
        {#if deck.children.length > 0}
          {@render branch(deck.children)}
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<main class="mx-auto flex max-w-prose flex-col gap-4 px-6 py-12">
  <h1 class="text-2xl font-semibold tracking-tight">AI and your decks</h1>
  <p>Turn on a deck's switch to keep its cards away from AI features. Studying works the same either way.</p>
  <p role="status" class="min-h-6">
    {#if failure === 'load'}These settings could not be loaded.{:else if failure === 'save'}That change was not saved. Try again.{/if}
  </p>
  {#if failure === 'load'}
    <button
      type="button"
      class="min-h-11 self-start rounded-md bg-foreground px-4 font-medium text-background transition-colors duration-150"
      onclick={() => void load()}
    >
      Try again
    </button>
  {:else if tree === null}
    <p>{m.loading()}</p>
  {:else}
    {@render branch(shown)}
  {/if}
  <nav>
    <a href="/study" class="inline-flex min-h-11 items-center rounded-md border px-4">Back to decks</a>
  </nav>
</main>
