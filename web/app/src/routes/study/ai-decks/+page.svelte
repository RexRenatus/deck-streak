<script lang="ts">
  // SPEC-381 R8; ADR-392 D3. The AI-and-your-decks route: the deck tree of the app's one engine,
  // each deck's switch over the marks the server keeps.
  import { api } from '$lib/api';
  import AiDecks from '$lib/study/AiDecks.svelte';
  import { studyEngine } from '$lib/study/engine';
</script>

<AiDecks client={() => studyEngine().client()} marks={api} />
