<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { BEATS, DICE, reducedMotion, type FeedItem } from './feed';

  // One celebration of the owner's feed, at the tier the router rendered it at (SPEC-084 R10): a T3
  // is a two-beat reveal, a T4 a dice then the message, a T5 a dice, the message and a card that
  // stays until the owner dismisses it. When the device asks for reduced motion, each tier shows
  // the same content without motion: the reveal's placeholder, which only moves, is not shown, and
  // the stylesheet moves nothing.
  let { item }: { item: FeedItem } = $props();

  const motion = reducedMotion() ? 'off' : 'on';
  const beats = $derived(
    BEATS[item.tier].filter((beat) => motion === 'on' || beat !== 'placeholder')
  );
  let dismissed = $state(false);
</script>

{#if !dismissed && beats.length > 0}
  <article
    data-kind={item.kind}
    data-tier={item.tier}
    data-motion={motion}
    class="rounded-lg bg-card p-4 text-card-foreground"
  >
    {#each beats as beat (beat)}
      {#if beat === 'placeholder'}
        <p data-beat={beat} aria-hidden="true" class="ladder-placeholder text-muted-foreground">
          {m.ladder_opening()}
        </p>
      {:else if beat === 'dice'}
        <span data-beat={beat} role="img" aria-label={m.ladder_dice()} class="ladder-dice block text-4xl">
          {DICE}
        </span>
      {:else if beat === 'message'}
        <p data-beat={beat} class="ladder-message">{item.text}</p>
      {:else}
        <div data-beat={beat} class="ladder-card mt-3 flex justify-end">
          <button
            type="button"
            class="rounded-md border px-3 py-1 text-sm text-link"
            onclick={() => (dismissed = true)}
          >
            {m.ladder_dismiss()}
          </button>
        </div>
      {/if}
    {/each}
  </article>
{/if}

<style>
  /* Each beat enters in turn; the reveal's placeholder shows first and gives way to the message. */
  @keyframes ladder-enter {
    from {
      opacity: 0;
      transform: translateY(0.5rem);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes ladder-reveal {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
      height: 0;
      margin: 0;
    }
  }
  [data-motion='on'] .ladder-dice {
    animation: ladder-enter 400ms ease-out both;
  }
  [data-motion='on'] .ladder-message {
    animation: ladder-enter 400ms ease-out 600ms both;
  }
  [data-motion='on'] .ladder-card {
    animation: ladder-enter 400ms ease-out 1000ms both;
  }
  [data-motion='on'] .ladder-placeholder {
    animation: ladder-reveal 300ms ease-in 600ms both;
  }
  /* The device's own setting wins over the check made when the item mounted: nothing moves, and
   * the placeholder, which only moves, is not shown. */
  @media (prefers-reduced-motion: reduce) {
    .ladder-dice,
    .ladder-message,
    .ladder-card {
      animation: none;
    }
    .ladder-placeholder {
      display: none;
    }
  }
</style>
