<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '$lib/paraglide/messages.js';
  import type { Envelope } from './insights';

  // SPEC-094 R19: one instrument's section. A run that failed, or a read that failed, is shown as
  // a failure line naming what failed; the body, an all-clear included, is drawn only for a clean
  // read, since a report that names a failed read claims nothing.
  let {
    title,
    envelope,
    children
  }: { title: string; envelope: Envelope; children: Snippet } = $props();

  const failed = $derived(envelope.failedReads.length > 0 || envelope.report === null);
</script>

<section class="mt-6" aria-label={title}>
  <h2 class="text-lg font-semibold">{title}</h2>
  {#if failed}
    <div role="alert" class="mt-2">{m.insights_failed_read({ reads: envelope.failedReads.join(', ') })}</div>
  {:else}
    {@render children()}
  {/if}
</section>
