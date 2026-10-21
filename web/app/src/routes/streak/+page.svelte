<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import StreakScreen from '$lib/streak/StreakScreen.svelte';
  import type { StreakView } from '$lib/streak/streak';
  import { m } from '$lib/paraglide/messages.js';

  // The streak screen (SPEC-076 R23): the owner's streak as the server answers it, the same numbers
  // the bot's /streak reports.
  let streak = $state<Answer<StreakView>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.streak()).then((answer) => {
      streak = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if streak === undefined}
      <div role="status">{m.loading()}</div>
    {:else if streak.kind === 'ok'}
      <StreakScreen view={streak.value} />
    {:else if streak.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
