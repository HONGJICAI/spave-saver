<script lang="ts">
  import '../style.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import PathSelector from '$lib/components/PathSelector.svelte';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  // @ts-ignore
  import { beforeNavigate } from '$app/navigation';
  import { getModeName } from '$lib/api';
  import { appState } from '$lib/stores/app';

  onMount(() => {
    const modeEl = document.getElementById('app-mode');
    if (modeEl) {
      modeEl.textContent = getModeName();
    }
  });

  // Block client-side navigation while a scan/delete/fix/compress is in flight.
  // Leaving a page mid-operation would unmount it, discard its in-memory
  // progress, and orphan the still-running promise (its result would be written
  // to destroyed component state). The Sidebar greys its links to signal this;
  // this guard is the actual enforcement and also covers in-page links (e.g.
  // the Home quick actions) that the visual cue doesn't reach.
  beforeNavigate((navigation: { cancel: () => void }) => {
    if (get(appState).busy) {
      navigation.cancel();
    }
  });
</script>

<!-- h-screen + overflow-hidden: the window itself never scrolls; long content
     scrolls inside <main> (or inside page-level lists) instead -->
<div class="flex h-screen overflow-hidden bg-gray-100">
  <Sidebar />
  <div class="flex-1 flex flex-col min-w-0">
    <!-- Fixed path selector header -->
    <div class="bg-white border-b border-gray-200 px-8 py-4 z-10 shadow-sm shrink-0">
      <PathSelector />
    </div>

    <!-- Scrollable page content. This <main> is the single source of page
         padding and width: full-width, left-aligned, with p-8 gutters. Pages
         should NOT add their own max-w-* / mx-auto / outer padding so the
         layout stays consistent across routes. -->
    <main class="flex-1 min-h-0 p-8 overflow-y-auto overflow-x-hidden">
      <slot />
    </main>
  </div>
</div>
