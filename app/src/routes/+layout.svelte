<script lang="ts">
  import '../style.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import PathSelector from '$lib/components/PathSelector.svelte';
  import { onMount } from 'svelte';
  import { getModeName } from '$lib/api';
  
  onMount(() => {
    const modeEl = document.getElementById('app-mode');
    if (modeEl) {
      modeEl.textContent = getModeName();
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

    <!-- Scrollable page content -->
    <main class="flex-1 min-h-0 p-8 overflow-y-auto overflow-x-hidden">
      <slot />
    </main>
  </div>
</div>
