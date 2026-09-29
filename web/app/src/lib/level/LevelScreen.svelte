<script lang="ts">
  import type { LevelView } from './level';

  // The level screen (SPEC-072 R26).
  let { view }: { view: LevelView } = $props();
</script>

<section aria-label="stub">{view.title}</section>
