<script lang="ts">
  // SPEC-350 R6; ADR-361. The deck list's route: the deck list over the app's one engine, which
  // opens the review once a deck is chosen.
  import { goto } from '$app/navigation';
  import DeckList from '$lib/study/DeckList.svelte';
  import { studyEngine } from '$lib/study/engine';
</script>

<DeckList client={() => studyEngine().client()} onopen={() => void goto('/study/review')} />
