<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { DarkFieldsReport } from './insights';

  // SPEC-094 R19: Dark Fields' body. Zero dark fields is a checked result, stated with what was
  // checked; a cold collection says there was nothing to check; a list past the cap counts the rest.
  let { report }: { report: DarkFieldsReport } = $props();

  const hidden = $derived(Math.max(0, report.darkFieldsTotal - report.darkFields.length));
</script>

{#if report.isCold}
  <p class="mt-2">{m.dark_fields_cold()}</p>
{:else if report.darkFieldsTotal === 0}
  <p class="mt-2">
    {m.dark_fields_none({ types: report.notetypesChecked, notes: report.reviewedNoteCount })}
  </p>
{:else}
  <ul class="mt-2 space-y-1">
    {#each report.darkFields as item (item.noteType + '\u0000' + item.field)}
      <li>
        {m.dark_fields_item({
          noteType: item.noteType,
          field: item.field,
          notes: item.reviewedNotes
        })}
      </li>
    {/each}
  </ul>
  {#if hidden > 0}
    <p class="mt-2">{m.dark_fields_more({ count: hidden })}</p>
  {/if}
{/if}
{#if report.unparseable.length > 0}
  <p class="mt-2">
    {m.dark_fields_unparseable({ names: report.unparseable.map((u) => u.noteType).join(', ') })}
  </p>
{/if}
