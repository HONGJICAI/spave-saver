<script lang="ts">
  import { scanDirectories, type ScanResult } from '$lib/api';
  import FileList from '$lib/components/FileList.svelte';
  import StatCard from '$lib/components/StatCard.svelte';
  import { formatSize } from '$lib/utils/format';
  import { appState } from '$lib/stores/app';
  
  let loading = false;
  let error = '';
  let results: ScanResult[] = [];
  
  // Aggregate results for display
  $: result = results.length > 0 ? {
    path: results.map(r => r.path).join(', '),
    file_count: results.reduce((sum, r) => sum + r.file_count, 0),
    total_size: results.reduce((sum, r) => sum + r.total_size, 0),
    files: results.flatMap(r => r.files)
  } : null;
  
  async function handleScan() {
    // Use scanPaths
    const paths = $appState.scanPaths;

    if (paths.length === 0) {
      error = 'Please enter at least one path to scan';
      return;
    }    loading = true;
    error = '';
    results = [];
    
    try {
      results = await scanDirectories(paths, $appState.filterConfig);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to scan directory';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>Scan - Space-Saver</title>
</svelte:head>

<div class="max-w-7xl">
  <h1 class="text-3xl font-bold text-gray-900 mb-6">🔍 Scan Directory</h1>
  
  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <button
      onclick={handleScan}
      disabled={loading}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed w-full"
    >
      {loading ? '⏳ Scanning...' : '🔍 Scan'}
    </button>
    
    {#if error}
      <div class="mt-4 p-4 bg-red-50 text-red-700 rounded-lg">
        ⚠️ {error}
      </div>
    {/if}
  </div>
  
  {#if result}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <StatCard
        label="Total Files"
        value={result.file_count.toLocaleString()}
        icon="📄"
        color="blue"
      />
      <StatCard
        label="Total Size"
        value={formatSize(result.total_size)}
        icon="💾"
        color="green"
      />
      <StatCard
        label="Scanned Path"
        value={result.path}
        icon="📁"
        color="purple"
      />
    </div>
    
    <div class="bg-white rounded-lg shadow p-6">
      <h2 class="text-xl font-bold text-gray-900 mb-4">
        Files Found ({result.files.length})
      </h2>
      <FileList files={result.files} />
    </div>
  {/if}
</div>
