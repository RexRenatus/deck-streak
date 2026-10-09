<script lang="ts">
  // SPEC-377 R8, R10; ADR-388 D11. The sync screen's route, over the app's one engine.
  import SyncScreen from '$lib/sync/SyncScreen.svelte';
  import { studyEngine } from '$lib/study/engine';
</script>

<SyncScreen client={() => studyEngine().client()} lost={() => studyEngine().lost()} />
