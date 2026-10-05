<script lang="ts">
  // SPEC-350 R7; ADR-361. The review's route: the review screen over the app's one engine.
  import ReviewScreen from '$lib/study/ReviewScreen.svelte';
  import { studyEngine } from '$lib/study/engine';
</script>

<ReviewScreen client={() => studyEngine().client()} />
