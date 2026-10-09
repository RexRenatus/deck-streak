<script lang="ts">
  // SPEC-377 R9, R11; ADR-388 D5, D13. The choice screen: what each direction the engine offered
  // replaces, as the Worker counted it, and no button for a direction it did not offer; neither
  // direction chosen or focused for the owner; an upload's snapshot age before its tap, or that the
  // upload waits; a cancel at every step before the write; and, when the counts changed, the new
  // counts with a sentence saying why. Every value shown is a reply of the Worker's (D5).
  import { onMount } from 'svelte';
  import type { ChoiceCounts, Direction, SnapshotRead } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { snapshotAge, type ChoiceClient } from './status';

  let {
    client,
    lost,
    ondone
  }: { client: ChoiceClient; lost: boolean; ondone: (outcome: 'written' | 'cancelled') => void } = $props();

  /** Where the choice stands: counting, its counts shown, written, or cancelled. */
  let phase = $state<'counting' | 'shown' | 'written' | 'cancelled'>('counting');
  let counts = $state.raw<ChoiceCounts | null>(null);
  let snapshot = $state.raw<SnapshotRead | null>(null);
  /** The sentence beside the counts: which side changed since they were counted, or that the last
   * tap replaced nothing. */
  let sentence = $state<'server' | 'device' | 'refused' | null>(null);
  /** Whether a tap is with the Worker, so no second tap or cancel races it. */
  let busy = $state(false);

  /** Counts each offered direction. A count answered after a cancel changes nothing; a count that
   * counted nothing offers no direction and says the choice replaced nothing. */
  async function count(): Promise<void> {
    const answer = await client.choiceCount().catch(() => null);
    if (phase !== 'counting') return;
    counts = answer?.counts ?? null;
    snapshot = answer?.snapshot ?? null;
    if (counts === null) sentence = 'refused';
    phase = 'shown';
  }

  /** The owner's one tap on `direction`. */
  async function confirm(direction: Direction): Promise<void> {
    busy = true;
    sentence = null;
    const answer = await client.choiceConfirm(direction).catch(() => null);
    busy = false;
    if (answer?.outcome === 'written') {
      phase = 'written';
      ondone('written');
    } else if (answer?.outcome === 'changed') {
      counts = answer.counts;
      sentence = answer.why;
    } else {
      sentence = 'refused';
    }
  }

  /** Drops the counted choice: the Worker forgets its stage, and nothing is written. */
  async function cancel(): Promise<void> {
    phase = 'cancelled';
    await client.choiceCancel().catch(() => null);
    ondone('cancelled');
  }

  onMount(() => void count());
</script>

<section class="flex flex-col gap-3" aria-labelledby="sync-choice-title">
  <h2 id="sync-choice-title" class="text-lg font-medium">{m.sync_choose()}</h2>
  {#if phase === 'counting'}
    <p>{m.sync_choice_counting()}</p>
  {:else if phase === 'written'}
    <p>{m.sync_choice_written()}</p>
  {:else if phase === 'shown'}
    {#if sentence === 'server'}
      <p>{m.sync_choice_changed_server()}</p>
    {:else if sentence === 'device'}
      <p>{m.sync_choice_changed_device()}</p>
    {:else if sentence === 'refused'}
      <p>{m.sync_choice_refused()}</p>
    {/if}
    {#if counts?.upload}
      <div class="flex flex-col gap-2 rounded-md border p-4">
        <p>{m.sync_choice_upload_loses(counts.upload)}</p>
        {#if snapshot?.found === true}
          <p>{m.sync_choice_snapshot_age({ age: snapshotAge(snapshot.age, getLocale()) })}</p>
        {:else}
          <p>{m.sync_choice_snapshot_waits()}</p>
        {/if}
        <button
          type="button"
          class="min-h-11 self-start rounded-md border px-4 font-medium disabled:opacity-60"
          disabled={busy || snapshot?.found !== true}
          onclick={() => void confirm('upload')}>{m.sync_choice_upload()}</button
        >
      </div>
    {/if}
    {#if counts?.download}
      <div class="flex flex-col gap-2 rounded-md border p-4">
        <p>{m.sync_choice_download_loses(counts.download)}</p>
        <button
          type="button"
          class="min-h-11 self-start rounded-md border px-4 font-medium disabled:opacity-60"
          disabled={busy}
          onclick={() => void confirm('download')}>{lost ? m.sync_choice_restore() : m.sync_choice_download()}</button
        >
      </div>
    {/if}
  {/if}
  {#if phase === 'counting' || phase === 'shown'}
    <button
      type="button"
      class="min-h-11 self-start rounded-md border px-4 disabled:opacity-60"
      disabled={busy}
      onclick={() => void cancel()}>{m.sync_choice_cancel()}</button
    >
  {/if}
</section>
