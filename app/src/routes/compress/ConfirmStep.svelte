<script lang="ts">
  import type { CompressibleFile } from "$lib/api";
  import { formatSize } from "$lib/utils/format";
  import FileListItem from "./FileListItem.svelte";

  type Props = {
    compressibleFiles: CompressibleFile[];
    selectedFiles: Set<string>;
    poolSize: number;
    compressing: boolean;
    processedCount: number;
    totalToProcess: number;
    successCount: number;
    errorCount: number;
    onToggleFile: (path: string) => void;
    onToggleAll: () => void;
    onBack: () => void;
    onCompress: () => void;
    onPoolSizeChange: (size: number) => void;
  };

  let { 
    compressibleFiles,
    selectedFiles,
    poolSize,
    compressing,
    processedCount,
    totalToProcess,
    successCount,
    errorCount,
    onToggleFile,
    onToggleAll,
    onBack,
    onCompress,
    onPoolSizeChange
  }: Props = $props();

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
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-lg font-semibold">Review & Compress</h2>
    <button 
      onclick={onBack} 
      class="text-sm text-blue-600 hover:text-blue-800 font-medium"
    >
      ← Back to Scan
    </button>
  </div>
  
  <div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
    <h3 class="text-sm font-semibold text-gray-800 mb-3">Summary</h3>
    <div class="grid grid-cols-2 gap-4 text-sm">
      <div>
        <span class="text-gray-600">Selected Files:</span>
        <span class="font-semibold ml-2">{selectedFiles.size} of {compressibleFiles.length}</span>
      </div>
      <div>
        <span class="text-gray-600">Worker Pool Size:</span>
        <span class="font-semibold ml-2">{poolSize} concurrent worker{poolSize > 1 ? 's' : ''}</span>
      </div>
      <div>
        <span class="text-gray-600">Total Size:</span>
        <span class="font-semibold ml-2">{formatSize(totalOriginalSize)}</span>
      </div>
      <div>
        <span class="text-gray-600">Estimated Savings:</span>
        <span class="font-semibold ml-2 text-green-700">{formatSize(totalEstimatedSavings)}</span>
      </div>
    </div>
  </div>

  <!-- Worker Pool Configuration -->
  <div class="mb-6 p-4 bg-white border border-gray-200 rounded">
    <div class="block text-sm font-medium text-gray-700 mb-2">
      Worker Pool Size
    </div>
    <div class="flex items-center gap-3">
      <input 
        type="range" 
        min="1" 
        max="20" 
        value={poolSize}
        oninput={(e) => onPoolSizeChange(parseInt((e.target as HTMLInputElement).value))}
        class="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        disabled={compressing}
        aria-label="Worker pool size"
      />
      <span class="text-sm font-semibold text-gray-700 w-12 text-right">{poolSize}</span>
    </div>
    <p class="text-xs text-gray-600 mt-2">
      Process {poolSize} file{poolSize > 1 ? 's' : ''} concurrently. Higher values = faster but more CPU/memory usage.
    </p>
  </div>

  <!-- Progress Indicator -->
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
      <div class="mt-2 text-xs text-gray-600">
        <p>• {successCount} successful</p>
        <p>• {errorCount} failed</p>
      </div>
    </div>
  {/if}

  <!-- File List -->
  <div class="mb-6">
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-sm font-semibold text-gray-700">Selected Files ({selectedFiles.size})</h3>
      <button 
        onclick={onToggleAll} 
        class="text-xs text-blue-600 hover:text-blue-800 font-medium"
      >
        {selectedFiles.size === compressibleFiles.length ? 'Deselect All' : 'Select All'}
      </button>
    </div>
    <div class="border rounded-lg divide-y max-h-[400px] overflow-y-auto">
      {#each compressibleFiles as file}
        <FileListItem 
          {file} 
          isSelected={selectedFiles.has(file.path)} 
          onToggle={onToggleFile} 
        />
      {/each}
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="flex items-center justify-end gap-3">
    <button 
      onclick={onBack} 
      disabled={compressing} 
      class="px-4 py-2 bg-gray-100 text-gray-700 rounded hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
    >
      Cancel
    </button>
    <button 
      onclick={onCompress} 
      disabled={compressing || selectedFiles.size === 0} 
      class="px-6 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-sm font-medium flex items-center gap-2"
    >
      {#if compressing}
        <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        Processing {processedCount}/{totalToProcess}
      {:else}
        Compress {selectedFiles.size} File{selectedFiles.size !== 1 ? 's' : ''}
      {/if}
    </button>
  </div>
</div>
