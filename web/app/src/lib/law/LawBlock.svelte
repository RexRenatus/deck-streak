<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { TIERS, type LawLineKey, type LawTiersView, type LawView } from './law';

  // The law tab's block (SPEC-077 R10 to R14): the lines the block shows, in the server's order,
  // when it is shown, and a note when it is omitted; each count the recompute has not stored, or
  // whose port is not wired, as pending and never as 0; and SPEC-072's law cards and today's law XP
  // by tier, or a note when they cannot be read. The page computes no figure of its own.
  let { view, tiers }: { view: LawView; tiers: LawTiersView | null } = $props();

  /** The pending counts, in the block's order. */
  const pending = $derived(
    (['dues', 'mastery', 'leeches'] as const).filter((key) => view[key] === null)
  );

  /** The text of one shown line; a shown line's count is never pending (the reader refuses it). */
  function line(key: LawLineKey): string {
    switch (key) {
      case 'total_xp':
        return view.levelShown
          ? m.law_total_xp_level({ xp: view.totalXp, level: view.level })
          : m.law_total_xp({ xp: view.totalXp });
      case 'streak':
        return m.law_streak({ days: view.streak });
      case 'xp_today':
        return m.law_xp_today({ xp: view.xpToday });
      case 'dues':
        return m.law_dues({ count: String(view.dues) });
      case 'mastery':
        return m.law_mastery({ pct: Math.round(Number(view.mastery)) });
      case 'leeches':
        return m.law_leeches({ count: String(view.leeches) });
    }
  }

  /** The text of one pending count. */
  function waiting(key: 'dues' | 'mastery' | 'leeches'): string {
    if (key === 'dues') return m.law_dues_pending();
    return key === 'mastery' ? m.law_mastery_pending() : m.law_leeches_pending();
  }
</script>

<section aria-labelledby="law-title">
  <h2 id="law-title" class="text-sm font-medium">{m.law_title()}</h2>
  {#if view.shown}
    <ul class="mt-2 space-y-1" aria-label={m.law_title()}>
      {#each view.lines as key (key)}
        <li data-line={key}>
          {line(key)}
          {#if key === 'mastery'}<span class="block text-sm" data-mastery-about="law">{m.law_mastery_about()}</span>{/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="mt-2">{m.law_none()}</p>
  {/if}
  {#if pending.length > 0}
    <h3 class="mt-4 text-sm font-medium">{m.law_pending_title()}</h3>
    <ul class="mt-1 space-y-1" aria-label={m.law_pending_title()}>
      {#each pending as key (key)}
        <li data-pending={key}>{waiting(key)}</li>
      {/each}
    </ul>
  {/if}
  {#if tiers === null}
    <p class="mt-4 text-sm">{m.law_tiers_unavailable()}</p>
  {:else}
    <table class="mt-4 w-full text-sm">
      <caption class="text-left font-medium">{m.law_tiers_title()}</caption>
      <thead>
        <tr>
          <th scope="col" class="text-left">{m.law_tiers_tier()}</th>
          <th scope="col" class="text-right">{m.law_tiers_cards()}</th>
          <th scope="col" class="text-right">{m.law_tiers_xp()}</th>
        </tr>
      </thead>
      <tbody>
        {#each TIERS as tier (tier)}
          <tr data-tier={tier}>
            <th scope="row" class="text-left font-normal">{tier === 'none' ? m.law_tiers_none() : tier}</th>
            <td class="text-right">{tiers.cards[tier]}</td>
            <td class="text-right">{tiers.xpToday[tier]}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>
