<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import type { BadgesView } from './badges';

  // The badge gallery (SPEC-073 R16, R19): the earned badges in the server's order, newest first,
  // each with the day it was earned; then every locked badge with its criteria, and a locked badge
  // whose input the server keeps shows that input against its threshold. The page computes nothing:
  // every number is the server's.
  let { view }: { view: BadgesView } = $props();
</script>

<section aria-labelledby="badges-earned">
  <h2 id="badges-earned" class="text-sm font-medium">{m.badges_earned_title()}</h2>
  {#if view.earned.length === 0}
    <p class="mt-2">{m.badges_none()}</p>
  {:else}
    <ul class="mt-2 space-y-2" aria-label={m.badges_earned_title()}>
      {#each view.earned as badge (badge.key + badge.tier)}
        <li data-key={badge.key}>
          <span class="font-medium">{badge.emoji} {badge.name}</span>
          <span class="text-sm">{m.badges_earned_on({ day: badge.studyDay })}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="mt-6" aria-labelledby="badges-locked">
  <h2 id="badges-locked" class="text-sm font-medium">{m.badges_locked_title()}</h2>
  <ul class="mt-2 space-y-3" aria-label={m.badges_locked_title()}>
    {#each view.locked as badge (badge.key)}
      <li data-key={badge.key} data-family={badge.family}>
        <span class="font-medium">{badge.emoji} {badge.name}</span>
        <span class="block text-sm">{badge.criteria}</span>
        {#if badge.progress !== null}
          <meter
            class="mt-1 block h-2 w-full"
            min="0"
            max={badge.progress.threshold}
            value={badge.progress.value}
            aria-label={m.badges_progress({ name: badge.name })}
          ></meter>
          <span class="text-sm">
            {m.badges_progress_text({
              value: badge.progress.value,
              threshold: badge.progress.threshold
            })}
          </span>
        {/if}
      </li>
    {/each}
  </ul>
</section>
