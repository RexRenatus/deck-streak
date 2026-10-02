<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import BadgeGallery from '$lib/badges/BadgeGallery.svelte';
  import type { BadgesView } from '$lib/badges/badges';
  import { m } from '$lib/paraglide/messages.js';

  // The badges screen (SPEC-073 R16, R19): the owner's badges, earned and locked, as the server
  // answers them.
  let badges = $state<Answer<BadgesView>>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  $effect(() => {
    void untrack(() => api.badges()).then((answer) => {
      badges = answer;
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if badges === undefined}
      <div role="status">{m.loading()}</div>
    {:else if badges.kind === 'ok'}
      <BadgeGallery view={badges.value} />
    {:else if badges.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
