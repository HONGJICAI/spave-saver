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

  let compressedResults = $derived(results.filter(r => r.status === 'compressed'));
  let skippedResults = $derived(results.filter(r => r.status === 'skipped'));
  let failedResults = $derived(results.filter(r => r.status === 'failed'));

  let totalActualSavings = $derived(
    compressedResults.reduce((sum, r) => sum + (r.savings || 0), 0)
  );

  let showAllCompressed = $state(false);
  let showAllFailed = $state(false);
  let showSkipped = $state(false);

  let displayedCompressedResults = $derived(
    showAllCompressed ? compressedResults : compressedResults.slice(0, 10)
  );
  let displayedFailedResults = $derived(
    showAllFailed ? failedResults : failedResults.slice(0, 10)
  );

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function dirName(path: string): string {
    return path.split(/[\\/]/).slice(0, -1).join('/');
  }
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
        <p>• {compressedResults.length} compressed</p>
        <p>• {skippedResults.length} skipped</p>
        <p>• {failedResults.length} failed</p>
      </div>

      <!-- Currently Processing Files (always visible with fixed height) -->
      <div class="mt-4 pt-4 border-t border-blue-200">
        <div class="flex items-center gap-2 mb-2">
          {#if currentlyProcessing.length > 0}
            <svg class="animate-spin h-4 w-4 text-blue-600" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          {:else}
            <svg class="h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" opacity="0.25"></circle>
            </svg>
          {/if}
          <span class="text-sm font-semibold text-gray-800">
            Currently Processing ({currentlyProcessing.length}):
          </span>
        </div>
        <div class="space-y-1 h-32 overflow-y-auto bg-white/30 rounded px-2 py-2">
          {#if currentlyProcessing.length === 0}
            <div class="flex items-center justify-center h-full text-xs text-gray-500 italic">
              Waiting for next file...
            </div>
          {:else}
            {#each currentlyProcessing as filePath}
              <div class="flex items-center gap-2 text-xs text-gray-700 bg-white/50 rounded px-2 py-1">
                <svg class="w-3 h-3 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                </svg>
                <span class="truncate" title={filePath}>{filePath}</span>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Summary Statistics (shown after completion) -->
  {#if !compressing && results.length > 0}
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 min-w-0">
        <p class="text-xs text-gray-600 mb-1">Total Processed</p>
        <p class="text-2xl font-bold text-blue-600">{results.length}</p>
      </div>
      <div class="bg-green-50 border border-green-200 rounded-lg p-4 min-w-0">
        <p class="text-xs text-gray-600 mb-1">Compressed</p>
        <p class="text-2xl font-bold text-green-600">{compressedResults.length}</p>
        <p class="text-xs text-green-700 mt-1">Saved: {formatSize(totalActualSavings)}</p>
      </div>
      <div class="bg-amber-50 border border-amber-200 rounded-lg p-4 min-w-0">
        <p class="text-xs text-gray-600 mb-1">Skipped</p>
        <p class="text-2xl font-bold text-amber-600">{skippedResults.length}</p>
        <p class="text-xs text-amber-700 mt-1">Already optimal, kept as-is</p>
      </div>
      <div class="bg-red-50 border border-red-200 rounded-lg p-4 min-w-0">
        <p class="text-xs text-gray-600 mb-1">Failed</p>
        <p class="text-2xl font-bold text-red-600">{failedResults.length}</p>
      </div>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Compressed Table -->
    <div class="border rounded-lg overflow-hidden min-w-0">
      <div class="bg-green-50 border-b border-green-200 px-4 py-3">
        <h3 class="font-semibold text-green-800 flex items-center gap-2">
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
          </svg>
          Compressed ({compressedResults.length})
        </h3>
      </div>
      <div class="max-h-[35vh] overflow-y-auto">
        {#if compressedResults.length === 0}
          <div class="p-8 text-center text-gray-500 text-sm">
            No compressed files yet
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
              {#each displayedCompressedResults as result}
                <tr class="hover:bg-green-50">
                  <td class="px-3 py-2 max-w-0 w-full">
                    <p class="font-medium text-gray-900 truncate" title={result.path}>
                      {fileName(result.path)}
                    </p>
                    <p class="text-xs text-gray-600 truncate" title={result.path}>
                      {dirName(result.path)}
                    </p>
                    <p class="text-xs text-purple-600 mt-1">Plugin: {result.plugin_name}</p>
                    {#if result.backup_path}
                      <p class="text-xs text-gray-500 truncate" title={result.backup_path}>
                        Backup: {fileName(result.backup_path)}
                      </p>
                    {/if}
                  </td>
                  <td class="px-3 py-2 text-right whitespace-nowrap align-top">
                    <p class="font-semibold text-green-700">{formatSize(result.savings || 0)}</p>
                    <p class="text-xs text-gray-600">
                      {((result.savings || 0) / (result.original_size || 1) * 100).toFixed(1)}%
                    </p>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if compressedResults.length > 10}
            <div class="p-2 bg-gray-50 border-t border-gray-200 text-center">
              <button
                onclick={() => showAllCompressed = !showAllCompressed}
                class="text-xs text-blue-600 hover:text-blue-800 font-medium"
              >
                {showAllCompressed ? `Show Less (10 of ${compressedResults.length})` : `Show All (${compressedResults.length})`}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Failed Table -->
    <div class="border rounded-lg overflow-hidden min-w-0">
      <div class="bg-red-50 border-b border-red-200 px-4 py-3">
        <h3 class="font-semibold text-red-800 flex items-center gap-2">
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
          </svg>
          Failed ({failedResults.length})
        </h3>
      </div>
      <div class="max-h-[35vh] overflow-y-auto">
        {#if failedResults.length === 0}
          <div class="p-8 text-center text-gray-500 text-sm">
            No errors
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
              {#each displayedFailedResults as result}
                <tr class="hover:bg-red-50">
                  <td class="px-3 py-2">
                    <p class="font-medium text-gray-900 truncate" title={result.path}>
                      {fileName(result.path)}
                    </p>
                    <p class="text-xs text-gray-600 truncate" title={result.path}>
                      {dirName(result.path)}
                    </p>
                  </td>
                  <td class="px-3 py-2">
                    <p class="text-xs text-red-600">{result.error}</p>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if failedResults.length > 10}
            <div class="p-2 bg-gray-50 border-t border-gray-200 text-center">
              <button
                onclick={() => showAllFailed = !showAllFailed}
                class="text-xs text-blue-600 hover:text-blue-800 font-medium"
              >
                {showAllFailed ? `Show Less (10 of ${failedResults.length})` : `Show All (${failedResults.length})`}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>

  <!-- Skipped Files (output was not smaller; originals kept untouched) -->
  {#if skippedResults.length > 0}
    <div class="mt-6">
      <button
        onclick={() => showSkipped = !showSkipped}
        class="w-full flex items-center justify-between p-3 bg-amber-50 border border-amber-200 rounded-lg hover:bg-amber-100 transition-colors"
      >
        <div class="flex items-center gap-2">
          <svg class="w-5 h-5 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <span class="font-semibold text-gray-900">Skipped ({skippedResults.length}) — already optimal, originals kept</span>
        </div>
        <svg
          class="w-5 h-5 text-gray-500 transition-transform {showSkipped ? 'rotate-180' : ''}"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
        </svg>
      </button>

      {#if showSkipped}
        <div class="mt-2 border rounded-lg divide-y max-h-[30vh] overflow-y-auto">
          {#each skippedResults as result}
            <div class="p-3 bg-white">
              <p class="text-sm font-medium text-gray-900 truncate" title={result.path}>{result.path}</p>
              <div class="mt-1 text-xs text-gray-600 flex flex-wrap gap-x-3">
                <span class="text-purple-600">{result.plugin_name}</span>
                <span>{result.reason}</span>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

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
