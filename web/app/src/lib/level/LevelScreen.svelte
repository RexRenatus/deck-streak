<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import AscendantChip from './AscendantChip.svelte';
  import type { LevelView } from './level';

  // The level screen (SPEC-072 R26): the level bar, today's XP by source with the provisional rows
  // marked as settling at the day's close, the run as a flame meter with its multiplier and the
  // one-miss preview beside it, and the Ascendant chip on an Ascendant day. The page computes
  // nothing: every number is the server's.
  let { view }: { view: LevelView } = $props();

  const SOURCES: Record<string, () => string> = {
    reviews: m.level_source_reviews,
    reviews_law: m.level_source_reviews_law,
    studied: m.level_source_studied,
    backlog_zero: m.level_source_backlog_zero,
    streak: m.level_source_streak,
    score90: m.level_source_score90,
    graduations: m.level_source_graduations,
    consistency: m.level_source_consistency,
    ascendant: m.level_source_ascendant
  };

  /** The meter's top: a week, or the run itself once it is longer. */
  const WEEK = 7;
  const flameMax = $derived(Math.max(view.run, WEEK));
</script>

<section aria-labelledby="level-title">
  <h2 id="level-title" class="text-sm font-medium">{m.level_title()}</h2>
  <p class="mt-1 text-3xl font-semibold">{view.emoji} {view.level} {view.title}</p>
  <meter
    class="mt-2 block h-2 w-full"
    min="0"
    max={view.xpForNext}
    value={view.xpIntoLevel}
    aria-label={m.level_progress()}
  ></meter>
  <p class="mt-1 text-sm">{m.level_progress_text({ into: view.xpIntoLevel, next: view.xpForNext })}</p>
  {#if view.ascendant}
    <AscendantChip />
  {/if}
</section>

<section class="mt-6" aria-labelledby="level-today">
  <h2 id="level-today" class="text-sm font-medium">{m.level_today()}</h2>
  {#if view.today.length === 0}
    <p class="mt-2">{m.level_today_none()}</p>
  {:else}
    <ul class="mt-2 space-y-2" aria-label={m.level_today()}>
      {#each view.today as row (row.source + row.track)}
        <li data-source={row.source} data-state={row.state}>
          <span class="font-medium">{SOURCES[row.source]?.() ?? row.source}</span>
          <span>{row.amount}</span>
          {#if row.state === 'provisional'}
            <span class="text-sm">{m.level_settling()}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="mt-6" aria-labelledby="level-run">
  <h2 id="level-run" class="text-sm font-medium">{m.level_run_title()}</h2>
  <meter
    class="mt-2 block h-2 w-full"
    min="0"
    max={flameMax}
    value={view.run}
    aria-labelledby="level-run"
  ></meter>
  <p class="mt-1">{m.level_run_days({ days: view.run })}</p>
  <p class="mt-1">{m.level_multiplier({ value: view.multiplier.toFixed(2) })}</p>
  <p class="mt-1 text-sm">{m.level_one_miss({ value: view.multiplierAfterAMiss.toFixed(2) })}</p>
</section>
