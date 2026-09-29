<script lang="ts">
  import './layout.css';
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import TierAnimation from '$lib/ladder/TierAnimation.svelte';
  import { celebrations, type FeedItem } from '$lib/ladder/feed';
  import { m } from '$lib/paraglide/messages.js';
  import { telegram } from '$lib/telegram.svelte';

  let { children } = $props();

  // The owner's unseen celebrations, each animated at the tier the router rendered it at, on every
  // screen (SPEC-084 R10). The region is on the page before any arrives, so a screen reader
  // announces each one as it is added.
  let served = $state<FeedItem[]>([]);

  // Telegram shows its own placeholder until the Mini App says it is ready. The root layout mounts
  // after the first screen has rendered, so this is the moment (SPEC-028 R2); the feed is read then.
  onMount(() => {
    telegram.ready();
    void celebrations(api).then((items) => {
      served = items;
    });
  });
</script>

<section aria-live="polite" aria-label={m.ladder_region()} class="mx-auto max-w-prose space-y-3 px-6">
  {#each served as item, index (index)}
    <TierAnimation {item} />
  {/each}
</section>

{@render children()}
