<script lang="ts">
  import { getStorageStats, scanDirectories, type StorageStats, type ScanResult } from '$lib/api';
  import StatCard from '$lib/components/StatCard.svelte';
  import FileList from '$lib/components/FileList.svelte';
  import { formatSize, percentage } from '$lib/utils/format';
  import { appState } from '$lib/stores/app';
  
  let loading = false;
  let error = '';
  let stats: StorageStats | null = null;
  let scanResults: ScanResult[] = [];
  let showFileList = false;
  
  // Aggregate scan results for file list
  $: filesResult = scanResults.length > 0 ? {
    file_count: scanResults.reduce((sum, r) => sum + r.file_count, 0),
    files: scanResults.flatMap(r => r.files)
  } : null;
  
  async function handleScan() {
    // Use scanPaths
    const paths = $appState.scanPaths;
    
    if (paths.length === 0) {
      error = 'Please enter at least one path to scan';
      return;
    }
    
    loading = true;
    error = '';
    stats = null;
    scanResults = [];
    
    try {
      // Fetch both stats and file list in parallel
      const [statsResult, filesData] = await Promise.all([
        getStorageStats(paths, $appState.filterConfig),
        scanDirectories(paths, $appState.filterConfig)
      ]);
      stats = statsResult;
      scanResults = filesData;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to get statistics';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>Statistics - Space-Saver</title>
</svelte:head>

<div class="max-w-7xl">
  <h1 class="text-3xl font-bold text-gray-900 mb-6">📊 Storage Statistics</h1>
  
  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <button
      onclick={handleScan}
      disabled={loading}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 w-full"
    >
      {loading ? '⏳ Analyzing...' : '📊 Analyze'}
    </button>
    
    {#if error}
      <div class="mt-4 p-4 bg-red-50 text-red-700 rounded-lg">
        ⚠️ {error}
      </div>
    {/if}
  </div>
  
  {#if stats}
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <StatCard
        label="Total Files"
        value={stats.total_files.toLocaleString()}
        icon="📄"
        color="blue"
      />
      <StatCard
        label="Total Size"
        value={formatSize(stats.total_size)}
        icon="💾"
        color="green"
      />
      <StatCard
        label="Empty Files"
        value={stats.empty_files}
        icon="📭"
        color="yellow"
      />
      <StatCard
        label="Path(s)"
        value={$appState.scanPaths.length > 0 ? `${$appState.scanPaths.length} path(s)` : 'Not set'}
        icon="📁"
        color="purple"
      />
    </div>
    
    <div class="bg-white rounded-lg shadow p-6">
      <h2 class="text-xl font-bold text-gray-900 mb-6">File Type Distribution</h2>
      
      <div class="space-y-4">
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">🖼️ Images</span>
            <span class="text-sm text-gray-600">
              {stats.images} files ({percentage(stats.images, stats.total_files)}%)
            </span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-3">
            <div 
              class="bg-blue-600 h-3 rounded-full"
              style="width: {percentage(stats.images, stats.total_files)}%"
            ></div>
          </div>
        </div>
        
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">🎬 Videos</span>
            <span class="text-sm text-gray-600">
              {stats.videos} files ({percentage(stats.videos, stats.total_files)}%)
            </span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-3">
            <div 
              class="bg-purple-600 h-3 rounded-full"
              style="width: {percentage(stats.videos, stats.total_files)}%"
            ></div>
          </div>
        </div>
        
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">📄 Documents</span>
            <span class="text-sm text-gray-600">
              {stats.documents} files ({percentage(stats.documents, stats.total_files)}%)
            </span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-3">
            <div 
              class="bg-green-600 h-3 rounded-full"
              style="width: {percentage(stats.documents, stats.total_files)}%"
            ></div>
          </div>
        </div>
        
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">🗜️ Archives</span>
            <span class="text-sm text-gray-600">
              {stats.archives} files ({percentage(stats.archives, stats.total_files)}%)
            </span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-3">
            <div 
              class="bg-yellow-600 h-3 rounded-full"
              style="width: {percentage(stats.archives, stats.total_files)}%"
            ></div>
          </div>
        </div>
        
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">📦 Others</span>
            <span class="text-sm text-gray-600">
              {stats.others} files ({percentage(stats.others, stats.total_files)}%)
            </span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-3">
            <div 
              class="bg-gray-600 h-3 rounded-full"
              style="width: {percentage(stats.others, stats.total_files)}%"
            ></div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- File List Section -->
    {#if filesResult && filesResult.files.length > 0}
      <div class="bg-white rounded-lg shadow p-6 mt-6">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-xl font-bold text-gray-900">
            📁 All Files ({filesResult.files.length.toLocaleString()})
          </h2>
          <button
            onclick={() => showFileList = !showFileList}
            class="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg transition-colors"
          >
            {showFileList ? '🔼 Hide Files' : '🔽 Show Files'}
          </button>
        </div>
        {#if showFileList}
          <FileList files={filesResult.files} />
        {:else}
          <p class="text-gray-500 text-sm">Click "Show Files" to view the complete file list.</p>
        {/if}
      </div>
    {/if}
  {/if}
</div>
