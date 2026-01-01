<script lang="ts">
  import type { CompressibleFile, RejectedFile } from "$lib/api";
  import { formatSize } from "$lib/utils/format";
  import FileListItem from "./FileListItem.svelte";

  type Props = {
    compressibleFiles: CompressibleFile[];
    rejectedFiles: RejectedFile[];
    selectedFiles: Set<string>;
    scanning: boolean;
    scanPaths: string[];
    onScan: () => void;
    onToggleFile: (path: string) => void;
    onToggleAll: () => void;
    onNext: () => void;
  };

  let { 
    compressibleFiles, 
    rejectedFiles, 
    selectedFiles, 
    scanning, 
    scanPaths,
    onScan,
    onToggleFile,
    onToggleAll,
    onNext 
  }: Props = $props();

  let showRejected = $state(false);

  // Group files by plugin
  let filesByPlugin = $derived.by(() => {
    const grouped = new Map<string, CompressibleFile[]>();
    for (const file of compressibleFiles) {
      const plugin = file.plugin_name || 'Unknown';
      if (!grouped.has(plugin)) {
        grouped.set(plugin, []);
      }
      grouped.get(plugin)!.push(file);
    }
    return Array.from(grouped.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  });

  let totalOriginalSize = $derived(
    compressibleFiles
      .filter(f => selectedFiles.has(f.path))
      .reduce((sum, f) => sum + f.original_size, 0)
  );

  let totalEstimatedSavings = $derived(
    compressibleFiles
      .filter(f => selectedFiles.has(f.path))
      .reduce((sum, f) => sum + f.estimated_savings, 0)
  );
</script>

<div class="bg-white rounded-lg shadow p-6">
  {#if compressibleFiles.length === 0}
    <!-- Initial Scan Prompt -->
    <div class="text-center py-16">
      <svg class="w-20 h-20 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
      </svg>
      <h3 class="text-xl font-semibold text-gray-900 mb-2">Ready to Find Compressible Files</h3>
      <p class="text-gray-600 mb-6 max-w-md mx-auto">
        {#if scanPaths.length === 0}
          Configure scan paths first, then click the button below to scan for files
        {:else}
          Click the button below to scan {scanPaths.length} path{scanPaths.length > 1 ? 's' : ''} for compressible files
        {/if}
      </p>
      <button 
        onclick={onScan} 
        disabled={scanning || scanPaths.length === 0} 
        class="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-base font-medium inline-flex items-center gap-2"
      >
        {#if scanning}
          <svg class="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          Scanning...
        {:else}
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
          Scan for Files
        {/if}
      </button>
    </div>
  {:else}
    <!-- Scanned Files List -->
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-lg font-semibold">Compressible Files ({compressibleFiles.length})</h2>
      <div class="flex items-center gap-3">
        <button 
          onclick={onScan} 
          disabled={scanning} 
          class="px-3 py-2 text-green-600 border border-green-600 rounded hover:bg-green-50 disabled:opacity-50 text-sm font-medium"
        >
          {scanning ? 'Scanning...' : 'Rescan'}
        </button>
        <button 
          onclick={onNext} 
          disabled={selectedFiles.size === 0} 
          class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-sm font-medium"
        >
          Next: Review ({selectedFiles.size} selected) →
        </button>
      </div>
    </div>
  
    <div class="mb-4 p-3 bg-blue-50 border border-blue-200 rounded">
      <div class="flex items-center justify-between">
        <div class="grid grid-cols-2 gap-4 text-sm flex-1">
          <div>
            <span class="text-gray-600">Total Size:</span>
            <span class="font-semibold ml-2">{formatSize(totalOriginalSize)}</span>
          </div>
          <div>
            <span class="text-gray-600">Estimated Savings:</span>
            <span class="font-semibold ml-2 text-green-700">{formatSize(totalEstimatedSavings)}</span>
          </div>
        </div>
        <button 
          onclick={onToggleAll} 
          class="ml-4 text-xs text-blue-600 hover:text-blue-800 font-medium whitespace-nowrap"
        >
          {selectedFiles.size === compressibleFiles.length ? 'Deselect All' : 'Select All'}
        </button>
      </div>
    </div>

    <div class="border rounded-lg divide-y max-h-[500px] overflow-y-auto">
      {#each filesByPlugin as [pluginName, files]}
        <div class="bg-gray-50">
          <!-- Plugin Header -->
          <div class="px-4 py-3 border-b bg-gradient-to-r from-blue-50 to-indigo-50">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
                </svg>
                <h3 class="font-semibold text-gray-900">{pluginName}</h3>
                <span class="text-sm text-gray-600">({files.length} file{files.length !== 1 ? 's' : ''})</span>
              </div>
              <div class="text-sm text-gray-600">
                <span class="font-medium">Potential Savings:</span>
                <span class="ml-1 text-green-700 font-semibold">
                  {formatSize(files.reduce((sum, f) => sum + f.estimated_savings, 0))}
                </span>
              </div>
            </div>
          </div>
          
          <!-- Files in this plugin -->
          <div class="divide-y">
            {#each files as file}
              <FileListItem 
                {file} 
                isSelected={selectedFiles.has(file.path)} 
                onToggle={onToggleFile} 
              />
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <!-- Rejected Files Section -->
    {#if rejectedFiles.length > 0}
      <div class="mt-6">
        <button 
          onclick={() => showRejected = !showRejected}
          class="w-full flex items-center justify-between p-3 bg-gray-50 border border-gray-200 rounded-lg hover:bg-gray-100 transition-colors"
        >
          <div class="flex items-center gap-2">
            <svg class="w-5 h-5 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
            </svg>
            <span class="font-semibold text-gray-900">Files Not Handled ({rejectedFiles.length})</span>
          </div>
          <svg 
            class="w-5 h-5 text-gray-500 transition-transform {showRejected ? 'rotate-180' : ''}" 
            fill="none" 
            stroke="currentColor" 
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
          </svg>
        </button>

        {#if showRejected}
          <div class="mt-2 border rounded-lg divide-y max-h-[400px] overflow-y-auto">
            {#each rejectedFiles as file}
              <div class="p-3 bg-white">
                <div class="flex items-start gap-2">
                  <svg class="w-4 h-4 text-orange-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                  </svg>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-gray-900 truncate">{file.path}</p>
                    <div class="mt-1 text-xs text-gray-600">
                      <span>Size: {formatSize(file.size)}</span>
                      <span class="ml-3">Extension: .{file.extension || 'none'}</span>
                    </div>
                    <div class="mt-2 space-y-1">
                      {#each file.rejection_reasons as rejection}
                        <div class="flex items-start gap-2 text-xs">
                          <span class="font-medium text-purple-600">{rejection.plugin_name}:</span>
                          <span class="text-gray-600">{rejection.reason}</span>
                        </div>
                      {/each}
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>
