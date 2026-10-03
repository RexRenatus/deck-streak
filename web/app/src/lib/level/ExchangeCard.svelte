<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import type { ExchangeView } from './exchange';

  // The XP exchange readout (SPEC-075 R9, #80): for each source bucket, the XP it paid, the cards
  // that graduated on the days it paid and the XP per graduated card, as the server answers them. A
  // bucket no card graduated for has no rate, and the card says so in words, never as 0.
  let { view }: { view: ExchangeView } = $props();

  // The level screen's names for the sources it knows; any other bucket shows as the server
  // names it.
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

  // A rate reads with one decimal, in the reader's locale.
  const rates = new Intl.NumberFormat(getLocale(), {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1
  });
</script>

<section class="mt-6" aria-labelledby="level-exchange">
  <h2 id="level-exchange" class="text-sm font-medium">{m.level_exchange_title()}</h2>
  {#if view.rates.length === 0}
    <p class="mt-2">{m.level_exchange_none()}</p>
  {:else}
    <table class="mt-2 w-full text-sm">
      <caption class="text-left">{m.level_exchange_caption()}</caption>
      <thead>
        <tr>
          <th scope="col" class="text-left">{m.level_exchange_source()}</th>
          <th scope="col" class="text-right">{m.level_exchange_xp()}</th>
          <th scope="col" class="text-right">{m.level_exchange_graduations()}</th>
          <th scope="col" class="text-right">{m.level_exchange_rate()}</th>
        </tr>
      </thead>
      <tbody>
        {#each view.rates as bucket (bucket.source)}
          <tr>
            <th scope="row" class="text-left font-medium">
              {SOURCES[bucket.source]?.() ?? bucket.source}
            </th>
            <td class="text-right">{bucket.totalXp}</td>
            <td class="text-right">{bucket.graduatedCards}</td>
            <td class="text-right">
              {bucket.rate === null ? m.level_exchange_undefined() : rates.format(bucket.rate)}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>
