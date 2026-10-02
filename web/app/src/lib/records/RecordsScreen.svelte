<script lang="ts">
  import type { RecordsView } from './records';

  let { view }: { view: RecordsView } = $props();
</script>

<section data-records={view.records.length}></section>
