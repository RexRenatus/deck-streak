<script lang="ts">
  import type { BoardView } from './board';

  // The personal board (SPEC-075 R3), inert: it renders none of the view yet.
  let { view }: { view: BoardView } = $props();
  void view;
</script>
