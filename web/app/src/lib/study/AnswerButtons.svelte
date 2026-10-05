<script lang="ts">
  // SPEC-350 R7, R10, A13; ADR-361. The four answer buttons, in grade order: native buttons, so Tab
  // reaches each in turn and Enter or Space presses it, each named by its grade and the interval the
  // engine describes for it. Each is at least 44 by 44 CSS px, and its colour settles within 150 ms.
  import { m } from '$lib/paraglide/messages.js';

  type Grade = 'again' | 'hard' | 'good' | 'easy';

  let { labels, onanswer }: { labels: readonly string[]; onanswer: (grade: Grade) => void } = $props();

  /** Each grade, as the wire orders them, with its name. */
  const GRADES: readonly { grade: Grade; name: () => string }[] = [
    { grade: 'again', name: m.study_again },
    { grade: 'hard', name: m.study_hard },
    { grade: 'good', name: m.study_good },
    { grade: 'easy', name: m.study_easy }
  ];
</script>

<div class="grid grid-cols-4 gap-2">
  {#each GRADES as { grade, name }, index (grade)}
    <button
      type="button"
      class="flex min-h-11 min-w-11 flex-col items-center justify-center rounded-md border px-2 py-1 transition-colors duration-150 hover:bg-muted"
      onclick={() => onanswer(grade)}
    >
      <span class="font-medium">{name()}</span>
      <span class="text-sm">{labels[index]}</span>
    </button>
  {/each}
</div>
