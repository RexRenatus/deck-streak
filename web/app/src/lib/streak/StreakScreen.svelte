<script lang="ts">
  import type { StreakView } from './streak';

  let { view }: { view: StreakView } = $props();
</script>

<section aria-label="Streaks">
  <h1>Streaks</h1>
  <p class="sr-only">{view.studyDay}</p>
</section>
