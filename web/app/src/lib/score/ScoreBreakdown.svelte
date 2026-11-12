<script lang="ts">
  import { m } from '#lib/paraglide/messages.js';
  import { PILLARS, type DayScore, type Pillar } from './score';

  // The score screen's breakdown (SPEC-071 R22): the total and the grade, then the five pillars,
  // each with its name, its value as text and a bar. A pillar the day does not have, the retention
  // of a day with no answered review, is a gap: its name and a sentence, and no value or bar.
  // Each pillar's row ends with a plain sentence saying what that pillar measures, as ordinary text
  // after its bar or its gap line, and the bar names that sentence as its description (SPEC-606
  // R1, R2, R6; ADR-627 D1, D5, D6).
  let { score }: { score: DayScore } = $props();

  const NAMES: Record<Pillar, () => string> = {
    consistency: m.pillar_consistency,
    retention: m.pillar_retention,
    workload: m.pillar_workload,
    volume: m.pillar_volume,
    mastery: m.pillar_mastery
  };

  const ABOUT: Record<Pillar, () => string> = {
    consistency: m.pillar_consistency_about,
    retention: m.pillar_retention_about,
    workload: m.pillar_workload_about,
    volume: m.pillar_volume_about,
    mastery: m.pillar_mastery_about
  };
</script>

<section aria-labelledby="score-total">
  <h2 id="score-total" class="text-sm font-medium">{m.score_title()}</h2>
  <p class="mt-1 text-4xl font-semibold">{score.total}</p>
  <p class="mt-1 text-lg">{score.grade.emoji} {score.grade.label}</p>
  <ul class="mt-6 space-y-3" aria-label={m.score_pillars()}>
    {#each PILLARS as pillar (pillar)}
      {@const value = score.pillars[pillar]}
      <li data-pillar={pillar}>
        <div class="flex justify-between">
          <span id="pillar-{pillar}" class="font-medium">{NAMES[pillar]()}</span>
          {#if value === null}
            <span>{m.retention_absent()}</span>
          {:else}
            <span>{Math.round(value)}</span>
          {/if}
        </div>
        {#if value !== null}
          <meter
            class="mt-1 block h-2 w-full"
            min="0"
            max="100"
            {value}
            aria-labelledby="pillar-{pillar}"
            aria-describedby="pillar-{pillar}-about"
          ></meter>
        {/if}
        <p id="pillar-{pillar}-about" class="mt-1 text-sm">{ABOUT[pillar]()}</p>
      </li>
    {/each}
  </ul>
</section>
