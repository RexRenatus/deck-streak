<script lang="ts">
  // SPEC-507 R2 to R9; ADR-525. The card health panel: a dialog over the review that shows the shown
  // card's own record. Its counters and its answers' capped times, the newest review rows first, the
  // recall rows (the scheduler's labelled estimate only when it is Ready, the state's line otherwise),
  // and where the card came from. It judges no card and reaches nothing: the review hands it the
  // record. The focus moves to Close when it opens; the review screen keeps Tab inside it and closes
  // it on Escape (`trap` below).
  import type { CardHealth } from '$lib/engine/protocol';
  import { m } from '$lib/paraglide/messages.js';
  import { answerName, createdDate, recallLines, seconds } from './card-health';

  let { health, loading, onclose }: { health: CardHealth | null; loading: boolean; onclose: () => void } =
    $props();

  let dialog: HTMLElement;
  let closer: HTMLButtonElement;

  const recall = $derived(health === null ? null : recallLines(health.scheduler));
  const differs = $derived(
    health !== null && (health.logged.answers !== health.reps || health.logged.lapses !== health.lapses)
  );

  // the focus moves into the dialog when it opens
  $effect(() => closer.focus());

  /** Keeps Tab and Shift+Tab inside the dialog: from its last control to its first, and back. */
  export function trap(event: KeyboardEvent): void {
    const controls = [...dialog.querySelectorAll<HTMLElement>('button, [href], [tabindex]:not([tabindex="-1"])')];
    const first = controls[0];
    const last = controls[controls.length - 1];
    event.preventDefault();
    const at = controls.indexOf(document.activeElement as HTMLElement);
    if (event.shiftKey) (at <= 0 ? last : controls[at - 1]).focus();
    else (at === -1 || at === controls.length - 1 ? first : controls[at + 1]).focus();
  }
</script>

<div
  bind:this={dialog}
  role="dialog"
  aria-modal="true"
  aria-labelledby="card-health-title"
  class="mt-4 flex flex-col gap-3 rounded-md border bg-card p-4"
>
  <h2 id="card-health-title" class="text-lg font-semibold">{m.card_health_title()}</h2>
  {#if loading}
    <p>{m.loading()}</p>
  {:else if health === null}
    <p>{m.card_health_unreadable()}</p>
  {:else}
    <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
      <dt class="font-medium">{m.card_health_reviews()}</dt>
      <dd>{health.reps}</dd>
      <dt class="font-medium">{m.card_health_lapses()}</dt>
      <dd>{health.lapses}</dd>
      <dt class="font-medium">{m.card_health_time_total()}</dt>
      <dd>{seconds(health.totalMs)}</dd>
      <dt class="font-medium">{m.card_health_time_mean()}</dt>
      <dd>{health.meanMs === null ? m.card_health_no_answers() : seconds(health.meanMs)}</dd>
      <dt class="font-medium">{m.card_health_time_median()}</dt>
      <dd>{health.medianMs === null ? m.card_health_no_answers() : seconds(health.medianMs)}</dd>
    </dl>
    {#if differs}
      <p>{m.card_health_logged({ answers: String(health.logged.answers), lapses: String(health.logged.lapses) })}</p>
    {/if}
    <section aria-labelledby="card-health-recall" class="flex flex-col gap-1">
      <h3 id="card-health-recall" class="font-semibold">{m.card_health_recall_title()}</h3>
      {#if recall !== null && recall.label !== null}
        <p class="font-medium">{recall.label}</p>
        <p>{recall.lines[0]}</p>
        <p>{recall.lines[1]}</p>
      {:else if recall !== null}
        <p>{recall.lines[0]}</p>
      {/if}
    </section>
    <section aria-labelledby="card-health-origin" class="flex flex-col gap-1">
      <h3 id="card-health-origin" class="font-semibold">{m.card_health_origin_title()}</h3>
      <p>{m.card_health_origin_collection({ date: createdDate(health.provenance.createdMs) })}</p>
    </section>
    {#if health.reviews.length > 0}
      <section aria-labelledby="card-health-history" class="flex flex-col gap-1">
        <h3 id="card-health-history" class="font-semibold">{m.card_health_history_title()}</h3>
        {#if health.rows > health.reviews.length}
          <p>{m.card_health_history_shown({ shown: String(health.reviews.length), rows: String(health.rows) })}</p>
        {/if}
        <table class="text-sm">
          <thead>
            <tr>
              <th scope="col" class="pr-3 text-left">{m.card_health_col_date()}</th>
              <th scope="col" class="pr-3 text-left">{m.card_health_col_answer()}</th>
              <th scope="col" class="text-left">{m.card_health_col_time()}</th>
            </tr>
          </thead>
          <tbody>
            {#each health.reviews as review (review.at)}
              <tr>
                <td class="pr-3">{createdDate(review.at)}</td>
                <td class="pr-3">{answerName(review.grade)}</td>
                <td>{seconds(review.ms)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  {/if}
  <button
    bind:this={closer}
    type="button"
    class="min-h-11 self-start rounded-md border px-4 transition-colors duration-150"
    onclick={onclose}
  >
    {m.card_health_close()}
  </button>
</div>
