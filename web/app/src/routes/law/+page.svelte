<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Answer } from '$lib/api';
  import LawBlock from '$lib/law/LawBlock.svelte';
  import type { LawTiersView, LawView } from '$lib/law/law';
  import { m } from '$lib/paraglide/messages.js';

  // The law tab (SPEC-077 R17): the law block and SPEC-072's law cards by tier, as the server
  // answers them. The block shows without the tiers when only they cannot be read; the page
  // computes no figure of its own.
  let read = $state<{ law: Answer<LawView>; tiers: Answer<LawTiersView> }>();

  // Ask once the screen is on the page, as Today does: untracked, and skipped by a server render.
  // Both answers arrive together, so the block never shows before its tiers are known.
  $effect(() => {
    void untrack(() => Promise.all([api.law(), api.lawTiers()])).then(([law, tiers]) => {
      read = { law, tiers };
    });
  });
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    {#if read === undefined}
      <div role="status">{m.loading()}</div>
    {:else if read.law.kind === 'ok'}
      <LawBlock view={read.law.value} tiers={read.tiers.kind === 'ok' ? read.tiers.value : null} />
    {:else if read.law.kind === 'reopen'}
      <div role="alert">{m.reopen_from_telegram()}</div>
    {:else}
      <div role="alert">{m.server_unavailable()}</div>
    {/if}
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
