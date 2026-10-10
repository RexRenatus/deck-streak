<script lang="ts">
  // SPEC-350 R6, R10, R11; ADR-361. The deck list: the engine's deck tree, each deck a native
  // button named by its name and its new, learning and review counts, its children nested under it.
  // Choosing a deck makes it the deck studied, then opens the review. A refusal is announced in the
  // status region with a way to ask again; a collection with no deck says so, and offers nothing
  // else (ruling 317 OQ4).
  import { onMount } from 'svelte';
  import { EngineError } from '$lib/engine/client';
  import type { Deck, ErrorCode } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import { statusText } from './refusal';

  /** What the deck list asks of the engine: the deck tree, and the deck to study. */
  interface DeckClient {
    decks(): Promise<Deck[]>;
    study(deck: bigint): Promise<null>;
  }

  let { client, onopen }: { client: () => Promise<DeckClient>; onopen: () => void } = $props();

  let decks = $state.raw<Deck[] | null>(null);
  let refusal = $state<ErrorCode | null>(null);

  /** A refusal of the engine's names its code; any other failure reads as the engine stopping. */
  function refused(error: unknown): void {
    refusal = error instanceof EngineError ? error.code : 'engine-failed';
  }

  async function load(): Promise<void> {
    refusal = null;
    decks = null;
    try {
      decks = await (await client()).decks();
    } catch (error) {
      refused(error);
    }
  }

  async function choose(deck: bigint): Promise<void> {
    try {
      await (await client()).study(deck);
      onopen();
    } catch (error) {
      refused(error);
    }
  }

  onMount(() => void load());
</script>

{#snippet tree(list: Deck[])}
  <ul class="flex flex-col gap-2">
    {#each list as deck (deck.id)}
      <li class="flex flex-col gap-2 [&>ul]:pl-4">
        <button
          type="button"
          class="flex min-h-11 flex-wrap items-baseline gap-x-2 rounded-md border px-4 py-2 text-left transition-colors duration-150 hover:bg-card"
          onclick={() => void choose(deck.id)}
        >
          <span class="font-medium">{deck.name}</span>
          <span class="text-sm">
            {m.study_deck_counts({ fresh: deck.new, learning: deck.learning, review: deck.review })}
          </span>
        </button>
        {#if deck.children.length > 0}
          {@render tree(deck.children)}
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<main class="mx-auto flex max-w-prose flex-col gap-4 px-6 py-12">
  <h1 class="text-2xl font-semibold tracking-tight">{m.study_title()}</h1>
  <p role="status" class="min-h-6">
    {#if refusal !== null}{statusText(refusal)}{/if}
  </p>
  {#if refusal !== null}
    <button
      type="button"
      class="min-h-11 self-start rounded-md bg-foreground px-4 font-medium text-background transition-colors duration-150"
      onclick={() => void load()}
    >
      {m.study_retry()}
    </button>
  {:else if decks === null}
    <p>{m.loading()}</p>
  {:else if decks.length === 0}
    <p>{m.study_no_decks()}</p>
  {:else}
    <h2 class="text-lg font-medium">{m.study_decks()}</h2>
    {@render tree(decks)}
  {/if}
  <nav class="flex flex-wrap gap-2">
    <a href="/study/ai-decks" class="inline-flex min-h-11 items-center rounded-md border px-4">AI and your decks</a>
    <a href="/" class="inline-flex min-h-11 items-center rounded-md border px-4">{m.back_to_today()}</a>
  </nav>
</main>
