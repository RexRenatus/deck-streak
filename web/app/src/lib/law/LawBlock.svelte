<script lang="ts">
  import type { LawView } from './law';

  let { view }: { view: LawView } = $props();
</script>

<section>
  <ul>
    <li data-line="dues">{view.dues ?? 0}</li>
    <li data-line="leeches">{view.leeches ?? 0}</li>
    <li data-line="mastery">{view.mastery ?? 0}</li>
  </ul>
</section>
