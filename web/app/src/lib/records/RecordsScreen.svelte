<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { RecordsView } from './records';

  // The records screen (SPEC-073 R13, R19): each personal record with its value, the day it was
  // set and the value it beat, and a bar of today's figure toward it whose gap is the server's
  // distance; then the record to chase. The page computes no distance of its own.
  let { view }: { view: RecordsView } = $props();
</script>

<section aria-labelledby="records-title">
  <h2 id="records-title" class="text-sm font-medium">{m.records_title()}</h2>
  {#if view.records.length === 0}
    <p class="mt-2">{m.records_none()}</p>
  {:else}
    <ul class="mt-2 space-y-3" aria-label={m.records_title()}>
      {#each view.records as line (line.kind)}
        <li data-kind={line.kind}>
          <span class="font-medium">{line.label}</span>
          <span class="font-semibold">{line.value}</span>
          <span class="block text-sm">
            {m.records_set({ day: line.studyDay, previous: line.previous })}
          </span>
          <meter
            class="mt-1 block h-2 w-full"
            min="0"
            max={line.value}
            value={line.value - line.distance}
            aria-label={m.records_toward({ label: line.label })}
          ></meter>
          <span class="text-sm">
            {line.distance > 0 ? m.records_to_go({ gap: line.distance }) : m.records_reached()}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
  {#if view.chase !== null}
    <p class="mt-4">{m.records_chase({ gap: view.chase.gap, label: view.chase.label })}</p>
  {/if}
</section>
