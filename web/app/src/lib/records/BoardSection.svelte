<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { BoardView } from './board';

  // The personal board (SPEC-075 R3, #79): the owner's best day, today, the language streak and
  // the level, in the server's order, each with the server's label, value and study day, the streak
  // with its longest run and the level with its title. The emoji is decoration and hidden from
  // assistive technology, because the label names the row. The board ranks the owner against their
  // own past.
  let { view }: { view: BoardView } = $props();
</script>

<section class="mt-6" aria-labelledby="records-board">
  <h2 id="records-board" class="text-sm font-medium">{m.records_board_title()}</h2>
  <ul class="mt-2 space-y-2" aria-label={m.records_board_title()}>
    {#each view.rows as row (row.kind)}
      <li data-kind={row.kind}>
        <span aria-hidden="true">{row.emoji}</span>
        <span class="font-medium">{row.label}</span>
        <span class="font-semibold">{row.value}</span>
        {#if row.studyDay !== null}
          <span class="text-sm">{row.studyDay}</span>
        {/if}
        {#if row.longest !== null}
          <span class="text-sm">{m.records_board_longest({ days: row.longest })}</span>
        {/if}
        {#if row.title !== null}
          <span class="text-sm">{row.title}</span>
        {/if}
      </li>
    {/each}
  </ul>
</section>
