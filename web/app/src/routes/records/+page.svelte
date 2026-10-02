<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import RecordsScreen from '$lib/records/RecordsScreen.svelte';
  import type { RecordsView } from '$lib/records/records';
  import { m } from '$lib/paraglide/messages.js';

  // The records screen (SPEC-073 R13, R19): the owner's personal records and today's distance to
  // each, as the server answers them, the same figures the bot's /records reports.
  let records = $state<Answer<RecordsView>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.records()).then((answer) => {
      records = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if records === undefined}
      <div role="status">{m.loading()}</div>
    {:else if records.kind === 'ok'}
      <RecordsScreen view={records.value} />
    {:else if records.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
