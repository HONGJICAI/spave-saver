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

<div class="flex min-h-screen bg-gray-100">
  <Sidebar />
  <div class="flex-1 flex flex-col">
      <!-- Fixed path selector header -->
      <div class="bg-white border-b border-gray-200 px-8 py-4 sticky top-0 z-10 shadow-sm">
        <PathSelector />
      </div>
    
    <!-- Scrollable page content -->
    <main class="flex-1 p-8 overflow-auto">
      <slot />
    </main>
  </div>
</div>
