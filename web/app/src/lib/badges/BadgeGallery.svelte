<script lang="ts">
  import type { BadgesView } from './badges';

  let { view }: { view: BadgesView } = $props();
</script>

<section data-earned={view.earned.length}></section>
