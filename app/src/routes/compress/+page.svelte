<script lang="ts">
  import { onMount } from 'svelte';
  import { appState } from '$lib/stores/app';
  import { compressFiles } from '$lib/api';
  import { formatSize } from '$lib/utils/format';

  let files = $state<File[]>([]);
  let outputPath = $state('');
  let compressionLevel = $state<'fast' | 'balanced' | 'best'>('balanced');
  let compressing = $state(false);
  let progress = $state(0);
  let result: { originalSize: number; compressedSize: number; path: string } | null = $state(null);

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files) {
      files = Array.from(input.files);
      // Set default output path based on first file
      if (files.length > 0 && !outputPath) {
        const firstFile = files[0];
        const name = firstFile.name.replace(/\.[^.]+$/, '');
        outputPath = `${name}_compressed.zip`;
      }
    }
  }

  function removeFile(index: number) {
    files = files.filter((_, i) => i !== index);
  }

  function clearFiles() {
    files = [];
    result = null;
  }

  async function handleCompress() {
    if (files.length === 0) {
      $appState.error = 'Please select files to compress';
      return;
    }

    if (!outputPath) {
      $appState.error = 'Please specify an output path';
      return;
    }

    compressing = true;
    progress = 0;
    $appState.error = null;
    result = null;

    try {
      // Simulate progress
      const progressInterval = setInterval(() => {
        if (progress < 90) {
          progress += 10;
        }
      }, 200);

      const filePaths = files.map(f => f.name); // In real app, would be actual paths
      const compressResult = await compressFiles(filePaths, outputPath, compressionLevel);
      
      clearInterval(progressInterval);
      progress = 100;
      result = compressResult;
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : 'Failed to compress files';
    } finally {
      compressing = false;
    }
  }

  function getTotalSize(): number {
    return files.reduce((total, file) => total + file.size, 0);
  }

  function getCompressionRatio(): number {
    if (!result) return 0;
    return ((result.originalSize - result.compressedSize) / result.originalSize) * 100;
  }
</script>

<div class="p-6">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">File Compression</h1>
    <p class="text-gray-600">Compress files and folders to save disk space</p>
  </div>

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      {$appState.error}
    </div>
  {/if}

  {#if result}
    <div class="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4">
      <div class="flex items-center">
        <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
        </svg>
        <div>
          <p class="font-semibold">Compression Complete!</p>
          <p class="text-sm mt-1">
            Compressed {formatSize(result.originalSize)} to {formatSize(result.compressedSize)}
            ({getCompressionRatio().toFixed(1)}% reduction)
          </p>
          <p class="text-sm mt-1">Saved to: {result.path}</p>
        </div>
      </div>
    </div>
  {/if}

  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <h2 class="text-lg font-semibold mb-4">Select Files</h2>
    
    <div class="mb-4">
      <label class="block w-full">
        <div class="border-2 border-dashed border-gray-300 rounded-lg p-8 text-center hover:border-blue-500 cursor-pointer transition-colors">
          <svg class="w-12 h-12 text-gray-400 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
          </svg>
          <p class="text-gray-600 mb-1">Click to select files</p>
          <p class="text-sm text-gray-500">or drag and drop</p>
        </div>
        <input
          type="file"
          multiple
          onchange={handleFileSelect}
          class="hidden"
        />
      </label>
    </div>

    {#if files.length > 0}
      <div class="mb-4">
        <div class="flex justify-between items-center mb-2">
          <h3 class="font-medium">Selected Files ({files.length})</h3>
          <button
            onclick={clearFiles}
            class="text-sm text-red-600 hover:text-red-800"
          >
            Clear All
          </button>
        </div>
        <div class="border rounded-lg divide-y max-h-60 overflow-y-auto">
          {#each files as file, idx}
            <div class="flex items-center justify-between p-3 hover:bg-gray-50">
              <div class="flex items-center flex-1 min-w-0">
                <svg class="w-5 h-5 text-gray-400 mr-2 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                </svg>
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-medium text-gray-900 truncate">{file.name}</p>
                  <p class="text-xs text-gray-500">{formatSize(file.size)}</p>
                </div>
              </div>
              <button
                onclick={() => removeFile(idx)}
                class="ml-2 text-red-600 hover:text-red-800 flex-shrink-0"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
              </button>
            </div>
          {/each}
        </div>
        <p class="text-sm text-gray-600 mt-2">
          Total size: <span class="font-semibold">{formatSize(getTotalSize())}</span>
        </p>
      </div>
    {/if}
  </div>

  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <h2 class="text-lg font-semibold mb-4">Compression Settings</h2>
    
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-2">
        Output File
      </label>
      <input
        type="text"
        bind:value={outputPath}
        placeholder="compressed_archive.zip"
        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
      />
    </div>

    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-2">
        Compression Level
      </label>
      <div class="grid grid-cols-3 gap-3">
        <button
          onclick={() => compressionLevel = 'fast'}
          class="px-4 py-3 border-2 rounded-lg text-center transition-all {compressionLevel === 'fast' ? 'border-blue-500 bg-blue-50 text-blue-700' : 'border-gray-200 hover:border-gray-300'}"
        >
          <div class="font-semibold">Fast</div>
          <div class="text-xs text-gray-600 mt-1">Quick compression</div>
        </button>
        <button
          onclick={() => compressionLevel = 'balanced'}
          class="px-4 py-3 border-2 rounded-lg text-center transition-all {compressionLevel === 'balanced' ? 'border-blue-500 bg-blue-50 text-blue-700' : 'border-gray-200 hover:border-gray-300'}"
        >
          <div class="font-semibold">Balanced</div>
          <div class="text-xs text-gray-600 mt-1">Good speed & ratio</div>
        </button>
        <button
          onclick={() => compressionLevel = 'best'}
          class="px-4 py-3 border-2 rounded-lg text-center transition-all {compressionLevel === 'best' ? 'border-blue-500 bg-blue-50 text-blue-700' : 'border-gray-200 hover:border-gray-300'}"
        >
          <div class="font-semibold">Best</div>
          <div class="text-xs text-gray-600 mt-1">Maximum compression</div>
        </button>
      </div>
    </div>

    {#if compressing}
      <div class="mb-4">
        <div class="flex justify-between text-sm text-gray-600 mb-1">
          <span>Compressing...</span>
          <span>{progress}%</span>
        </div>
        <div class="w-full bg-gray-200 rounded-full h-2">
          <div 
            class="bg-blue-600 h-2 rounded-full transition-all duration-300"
            style="width: {progress}%"
          ></div>
        </div>
      </div>
    {/if}

    <button
      onclick={handleCompress}
      disabled={compressing || files.length === 0}
      class="w-full px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed font-semibold"
    >
      {compressing ? 'Compressing...' : 'Compress Files'}
    </button>
  </div>

  <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
    <div class="flex">
      <svg class="w-5 h-5 text-blue-600 mr-2 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
      </svg>
      <div class="text-sm text-blue-800">
        <p class="font-semibold mb-1">Compression Tips:</p>
        <ul class="list-disc list-inside space-y-1">
          <li>Already compressed files (ZIP, JPG, MP4) won't benefit much from compression</li>
          <li>"Fast" mode is best for large files where speed matters</li>
          <li>"Best" mode provides maximum compression but takes longer</li>
          <li>Text files, documents, and source code compress very well</li>
        </ul>
      </div>
    </div>
  </div>
</div>
