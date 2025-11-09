<script lang="ts">
  import type { InPlaceCompressionResult } from "$lib/api";
  import { formatSize } from "$lib/utils/format";

  type Props = {
    results: InPlaceCompressionResult[];
    compressing: boolean;
    processedCount: number;
    totalToProcess: number;
    poolSize: number;
    currentlyProcessing: string[];
    onStartNew: () => void;
  };

  let { 
    results, 
    compressing, 
    processedCount, 
    totalToProcess, 
    poolSize,
    currentlyProcessing,
    onStartNew 
  }: Props = $props();

  let successResults = $derived(results.filter(r => r.success));
  let errorResults = $derived(results.filter(r => !r.success));
  
  let totalActualSavings = $derived(
    successResults.reduce((sum, r) => sum + (r.savings || 0), 0)
  );
</script>

<div class="bg-white rounded-lg shadow p-6">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-lg font-semibold">
      {compressing ? 'Processing Files...' : 'Compression Complete'}
    </h2>
    {#if !compressing}
      <button 
        onclick={onStartNew} 
        class="text-sm text-blue-600 hover:text-blue-800 font-medium"
      >
        Start New Scan
      </button>
    {/if}
  </div>
  
  <!-- Progress Bar (shown while compressing) -->
  {#if compressing}
    <div class="mb-6 p-4 bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-lg">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm font-semibold text-gray-800">
          Processing with {poolSize} worker{poolSize > 1 ? 's' : ''}
        </span>
        <span class="text-sm font-medium text-blue-600">
          {processedCount} / {totalToProcess} files
        </span>
      </div>
      <div class="w-full bg-gray-200 rounded-full h-3 overflow-hidden">
        <div 
          class="bg-gradient-to-r from-blue-500 to-indigo-600 h-3 rounded-full transition-all duration-300 ease-out"
          style="width: {totalToProcess > 0 ? (processedCount / totalToProcess * 100).toFixed(1) : 0}%"
        ></div>
      </div>
      <div class="mt-2 text-xs text-gray-600 flex gap-4">
        <p>• {successResults.length} successful</p>
        <p>• {errorResults.length} failed</p>
      </div>

      <!-- Currently Processing Files -->
      {#if currentlyProcessing.length > 0}
        <div class="mt-4 pt-4 border-t border-blue-200">
          <div class="flex items-center gap-2 mb-2">
            <svg class="animate-spin h-4 w-4 text-blue-600" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <span class="text-sm font-semibold text-gray-800">
              Currently Processing ({currentlyProcessing.length}):
            </span>
          </div>
          <div class="space-y-1 max-h-32 overflow-y-auto">
            {#each currentlyProcessing as filePath}
              <div class="flex items-center gap-2 text-xs text-gray-700 bg-white/50 rounded px-2 py-1">
                <svg class="w-3 h-3 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                </svg>
                <span class="truncate" title={filePath}>{filePath}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Summary Statistics (shown after completion) -->
  {#if !compressing && results.length > 0}
    <div class="grid grid-cols-3 gap-4 mb-6">
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <p class="text-xs text-gray-600 mb-1">Total Processed</p>
        <p class="text-2xl font-bold text-blue-600">{results.length}</p>
      </div>
      <div class="bg-green-50 border border-green-200 rounded-lg p-4">
        <p class="text-xs text-gray-600 mb-1">Successful</p>
        <p class="text-2xl font-bold text-green-600">{successResults.length}</p>
        <p class="text-xs text-green-700 mt-1">Saved: {formatSize(totalActualSavings)}</p>
      </div>
      <div class="bg-red-50 border border-red-200 rounded-lg p-4">
        <p class="text-xs text-gray-600 mb-1">Failed</p>
        <p class="text-2xl font-bold text-red-600">{errorResults.length}</p>
      </div>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Success Table -->
    <div class="border rounded-lg overflow-hidden">
      <div class="bg-green-50 border-b border-green-200 px-4 py-3">
        <h3 class="font-semibold text-green-800 flex items-center gap-2">
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
          </svg>
          Successful ({successResults.length})
        </h3>
      </div>
      <div class="max-h-96 overflow-y-auto">
        {#if successResults.length === 0}
          <div class="p-8 text-center text-gray-500 text-sm">
            No successful compressions yet
          </div>
        {:else}
          <table class="w-full text-sm">
            <thead class="bg-gray-50 sticky top-0">
              <tr class="text-left text-xs text-gray-600 border-b">
                <th class="px-3 py-2 font-medium">File</th>
                <th class="px-3 py-2 font-medium text-right">Savings</th>
              </tr>
            </thead>
            <tbody class="divide-y">
              {#each successResults as result}
                <tr class="hover:bg-green-50">
                  <td class="px-3 py-2">
                    <p class="font-medium text-gray-900 truncate" title={result.path}>
                      {result.path.split(/[\\/]/).pop()}
                    </p>
                    <p class="text-xs text-gray-600 truncate" title={result.path}>
                      {result.path.split(/[\\/]/).slice(0, -1).join('/')}
                    </p>
                    <p class="text-xs text-purple-600 mt-1">Plugin: {result.plugin_name}</p>
                  </td>
                  <td class="px-3 py-2 text-right whitespace-nowrap">
                    <p class="font-semibold text-green-700">{formatSize(result.savings || 0)}</p>
                    <p class="text-xs text-gray-600">
                      {((result.savings || 0) / (result.original_size || 1) * 100).toFixed(1)}%
                    </p>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>

    <!-- Error Table -->
    <div class="border rounded-lg overflow-hidden">
      <div class="bg-red-50 border-b border-red-200 px-4 py-3">
        <h3 class="font-semibold text-red-800 flex items-center gap-2">
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
          </svg>
          Failed ({errorResults.length})
        </h3>
      </div>
      <div class="max-h-96 overflow-y-auto">
        {#if errorResults.length === 0}
          <div class="p-8 text-center text-gray-500 text-sm">
            No errors - all compressions successful!
          </div>
        {:else}
          <table class="w-full text-sm">
            <thead class="bg-gray-50 sticky top-0">
              <tr class="text-left text-xs text-gray-600 border-b">
                <th class="px-3 py-2 font-medium">File</th>
                <th class="px-3 py-2 font-medium">Error</th>
              </tr>
            </thead>
            <tbody class="divide-y">
              {#each errorResults as result}
                <tr class="hover:bg-red-50">
                  <td class="px-3 py-2">
                    <p class="font-medium text-gray-900 truncate" title={result.path}>
                      {result.path.split(/[\\/]/).pop()}
                    </p>
                    <p class="text-xs text-gray-600 truncate" title={result.path}>
                      {result.path.split(/[\\/]/).slice(0, -1).join('/')}
                    </p>
                  </td>
                  <td class="px-3 py-2">
                    <p class="text-xs text-red-600">{result.error}</p>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>
  </div>

  <!-- Action Buttons -->
  {#if !compressing}
    <div class="flex items-center justify-end gap-3 mt-6">
      <button 
        onclick={onStartNew} 
        class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium"
      >
        Start New Scan
      </button>
    </div>
  {/if}
</div>
