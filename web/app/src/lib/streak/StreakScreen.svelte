<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { AtStake, StreakTrack, StreakView } from './streak';

  // The streak screen (SPEC-076 R23): both tracks side by side, the law track first when it has
  // activity, the at-stake line stated as the rule, and the governor's chip with its reason. The
  // page computes nothing: every number is the server's.
  let { view }: { view: StreakView } = $props();

  type Row = { key: 'law' | 'language'; name: string; track: StreakTrack; stake: AtStake };

  const rows = $derived.by(() => {
    const language: Row = {
      key: 'language',
      name: m.streak_language(),
      track: view.language,
      stake: view.atStake.language
    };
    const law: Row = { key: 'law', name: m.streak_law(), track: view.law, stake: view.atStake.law };
    return view.law.current > 0 ? [law, language] : [language, law];
  });

  const governorText = $derived.by(() => {
    if (view.governor.verdict === 'standby') return m.governor_standby();
    if (view.governor.verdict === 'lapse') return m.governor_lapse({ count: view.governor.relightCards ?? 0 });
    return m.governor_armed();
  });
</script>

<section aria-labelledby="streak-title">
  <h2 id="streak-title" class="text-sm font-medium">{m.streak_title()}</h2>
  <p class="sr-only">{view.studyDay}</p>
  <div class="mt-2 grid grid-cols-2 gap-4">
    {#each rows as row (row.key)}
      <article data-track={row.key}>
        <h3 class="font-medium">{row.name}</h3>
        <p class="text-3xl font-semibold">{m.streak_days({ days: row.track.current })}</p>
        <p class="text-sm">{m.streak_best({ days: row.track.longest })}</p>
        {#if row.track.freezes !== undefined && row.track.freezeCap !== undefined}
          <p class="text-sm">{m.streak_freezes({ count: row.track.freezes, cap: row.track.freezeCap })}</p>
        {/if}
        {#if row.stake === 'freeze'}
          <p class="text-sm">{m.streak_at_stake_freeze()}</p>
        {:else if row.stake === 'break'}
          <p class="text-sm">{m.streak_at_stake_break()}</p>
        {/if}
      </article>
    {/each}
  </div>
  <p class="mt-4" data-verdict={view.governor.verdict}>{governorText}</p>
</section>
