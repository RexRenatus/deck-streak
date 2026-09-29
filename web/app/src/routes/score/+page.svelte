<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import { m } from '$lib/paraglide/messages.js';
  import ScoreBreakdown from '$lib/score/ScoreBreakdown.svelte';
  import type { ScoreToday } from '$lib/score/score';

  // The score screen (SPEC-071 R22): the current study day's score as the server answers it, the
  // same numbers the bot's /score reports. A day no recompute has rolled up yet has no score, and
  // the screen says so.
  let today = $state<Answer<ScoreToday>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.score()).then((answer) => {
      today = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if today === undefined}
      <div role="status">{m.loading()}</div>
    {:else if today.kind === 'ok'}
      {#if today.value.score === null}
        <p>{m.score_none()}</p>
      {:else}
        <ScoreBreakdown score={today.value.score} />
      {/if}
    {:else if today.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
