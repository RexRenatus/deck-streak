<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import LevelScreen from '$lib/level/LevelScreen.svelte';
  import type { LevelView } from '$lib/level/level';
  import { m } from '$lib/paraglide/messages.js';

  // The level screen (SPEC-072 R26): the owner's level as the server answers it, the same numbers
  // the bot's /level reports.
  let level = $state<Answer<LevelView>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.level()).then((answer) => {
      level = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if level === undefined}
      <div role="status">{m.loading()}</div>
    {:else if level.kind === 'ok'}
      <LevelScreen view={level.value} />
    {:else if level.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
