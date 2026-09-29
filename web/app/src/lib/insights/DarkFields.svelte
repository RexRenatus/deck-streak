<script lang="ts">
  import type { DarkFieldsReport } from './insights';

  // SPEC-094 R19: Dark Fields' body. Inert until the green commit.
  let { report }: { report: DarkFieldsReport } = $props();
  void report;
</script>
