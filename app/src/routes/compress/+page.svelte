<script lang="ts">
  import { onMount } from "svelte";
  import { appState } from "$lib/stores/app";
  import {
    getCompressionPlugins,
    scanCompressibleFiles,
    compressFilesInPlace,
    type CompressionPlugin,
    type CompressibleFile,
    type RejectedFile,
    type InPlaceCompressionResult,
  } from "$lib/api";
  import { formatSize } from "$lib/utils/format";

  let availablePlugins = $state<CompressionPlugin[]>([]);
  let activePlugins = $state<Set<string>>(new Set());
  let scanning = $state(false);
  let compressibleFiles = $state<CompressibleFile[]>([]);
  let rejectedFiles = $state<RejectedFile[]>([]);
  let selectedFiles = $state<Set<string>>(new Set());
  let compressing = $state(false);
  let compressionResults = $state<InPlaceCompressionResult[]>([]);
  let showResults = $state(false);
  let showRejected = $state(false);
  
  // Worker pool configuration
  let poolSize = $state(2); // Number of concurrent workers
  let processedCount = $state(0);
  let totalToProcess = $state(0);
  
  // Step management
  type Step = 'scan' | 'confirm' | 'process';
  let currentStep = $state<Step>('scan');
  
  // Separate success and error results
  let successResults = $derived(compressionResults.filter(r => r.success));
  let errorResults = $derived(compressionResults.filter(r => !r.success));

  onMount(async () => {
    try {
      availablePlugins = await getCompressionPlugins();
      // Activate all plugins by default
      activePlugins = new Set(availablePlugins.map(p => p.name));
    } catch (err) {
      console.error("Failed to load plugins:", err);
      $appState.error = "Failed to load compression plugins";
    }
  });

  function togglePlugin(pluginName: string) {
    if (activePlugins.has(pluginName)) {
      activePlugins.delete(pluginName);
    } else {
      activePlugins.add(pluginName);
    }
    activePlugins = new Set(activePlugins);
  }

  function getActivePlugins(): string[] {
    return availablePlugins
      .filter(p => activePlugins.has(p.name))
      .map(p => p.name);
  }

  function movePluginUp(index: number) {
    if (index > 0) {
      const newPlugins = [...availablePlugins];
      [newPlugins[index - 1], newPlugins[index]] = [newPlugins[index], newPlugins[index - 1]];
      availablePlugins = newPlugins;
    }
  }

  function movePluginDown(index: number) {
    if (index < availablePlugins.length - 1) {
      const newPlugins = [...availablePlugins];
      [newPlugins[index], newPlugins[index + 1]] = [newPlugins[index + 1], newPlugins[index]];
      availablePlugins = newPlugins;
    }
  }

  async function handleScan() {
    if ($appState.scanPaths.length === 0) {
      $appState.error = "Please add paths to scan";
      return;
    }
    if (activePlugins.size === 0) {
      $appState.error = "Please activate at least one plugin";
      return;
    }
    scanning = true;
    $appState.error = null;
    compressibleFiles = [];
    rejectedFiles = [];
    selectedFiles.clear();
    try {
      const result = await scanCompressibleFiles(
        $appState.scanPaths, 
        getActivePlugins(),
        $appState.filterConfig
      );
      compressibleFiles = result.compressible;
      rejectedFiles = result.rejected;
      selectedFiles = new Set(result.compressible.map(f => f.path));
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : "Failed to scan files";
    } finally {
      scanning = false;
    }
  }

  function toggleFileSelection(path: string) {
    if (selectedFiles.has(path)) {
      selectedFiles.delete(path);
    } else {
      selectedFiles.add(path);
    }
    selectedFiles = new Set(selectedFiles);
  }

  function toggleAllFiles() {
    if (selectedFiles.size === compressibleFiles.length) {
      selectedFiles.clear();
    } else {
      selectedFiles = new Set(compressibleFiles.map(f => f.path));
    }
    selectedFiles = new Set(selectedFiles);
  }

  async function handleCompress() {
    if (selectedFiles.size === 0) {
      $appState.error = "Please select files to compress";
      return;
    }
    
    // Move to process step and start compression
    currentStep = 'process';
    compressing = true;
    $appState.error = null;
    compressionResults = [];
    processedCount = 0;
    
    try {
      const filesToCompress = Array.from(selectedFiles);
      totalToProcess = filesToCompress.length;
      const plugins = getActivePlugins();
      
      // Create a worker pool
      const pool: Promise<void>[] = [];
      let fileIndex = 0;
      
      // Worker function that processes one file at a time
      const worker = async () => {
        while (fileIndex < filesToCompress.length) {
          const currentIndex = fileIndex++;
          const filePath = filesToCompress[currentIndex];
          
          try {
            const results = await compressFilesInPlace([filePath], plugins);
            compressionResults = [...compressionResults, ...results];
          } catch (err) {
            console.error(`Failed to compress ${filePath}:`, err);
            compressionResults = [...compressionResults, {
              path: filePath,
              success: false,
              error: err instanceof Error ? err.message : "Unknown error"
            }];
          }
          
          processedCount++;
          
          // Small delay to allow UI to update
          await new Promise(resolve => setTimeout(resolve, 10));
        }
      };
      
      // Start workers (pool size = poolSize)
      for (let i = 0; i < poolSize; i++) {
        pool.push(worker());
      }
      
      // Wait for all workers to complete
      await Promise.all(pool);
      
      showResults = true;
      // Stay on process step to show results (don't move to another step)
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : "Failed to compress files";
    } finally {
      compressing = false;
    }
  }

  function getTotalOriginalSize(): number {
    return compressibleFiles.filter(f => selectedFiles.has(f.path)).reduce((sum, f) => sum + f.original_size, 0);
  }

  function getTotalEstimatedSavings(): number {
    return compressibleFiles.filter(f => selectedFiles.has(f.path)).reduce((sum, f) => sum + f.estimated_savings, 0);
  }

  function getSuccessCount(): number {
    return compressionResults.filter(r => r.success).length;
  }

  function getTotalActualSavings(): number {
    return compressionResults.filter(r => r.success).reduce((sum, r) => sum + (r.savings || 0), 0);
  }
  
  function goToScan() {
    currentStep = 'scan';
  }
  
  function goToConfirm() {
    if (compressibleFiles.length === 0) {
      $appState.error = "Please scan for files first";
      return;
    }
    currentStep = 'confirm';
  }
  
  function startNewScan() {
    currentStep = 'scan';
    showResults = false;
    compressionResults = [];
    compressibleFiles = [];
    rejectedFiles = [];
    selectedFiles.clear();
    processedCount = 0;
    totalToProcess = 0;
  }

</script>

<div class="p-6 max-w-[1600px] mx-auto">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Smart Compression Manager</h1>
    <p class="text-gray-600">Manage plugins, scan files, and compress with backups</p>
  </div>

  <!-- Step Indicator -->
  <div class="mb-8">
    <div class="flex items-center justify-center">
      <!-- Step 1: Scan -->
      <div class="flex items-center">
        <div class={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${currentStep === 'scan' ? 'bg-blue-600 border-blue-600 text-white' : compressibleFiles.length > 0 ? 'bg-green-600 border-green-600 text-white' : 'bg-gray-200 border-gray-300 text-gray-600'}`}>
          {#if compressibleFiles.length > 0 && currentStep !== 'scan'}
            <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path></svg>
          {:else}
            <span class="font-semibold">1</span>
          {/if}
        </div>
        <div class="ml-3 text-left">
          <p class={`text-sm font-semibold ${currentStep === 'scan' ? 'text-blue-600' : 'text-gray-900'}`}>Plugin & Scan</p>
          <p class="text-xs text-gray-500">Select plugins and scan files</p>
        </div>
      </div>
      
      <!-- Connector -->
      <div class={`w-24 h-0.5 mx-4 ${compressibleFiles.length > 0 ? 'bg-green-600' : 'bg-gray-300'}`}></div>
      
      <!-- Step 2: Confirm -->
      <div class="flex items-center">
        <div class={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${currentStep === 'confirm' ? 'bg-blue-600 border-blue-600 text-white' : currentStep === 'process' ? 'bg-green-600 border-green-600 text-white' : compressibleFiles.length > 0 ? 'bg-white border-gray-300 text-gray-600' : 'bg-gray-200 border-gray-300 text-gray-400'}`}>
          {#if currentStep === 'process'}
            <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path></svg>
          {:else}
            <span class="font-semibold">2</span>
          {/if}
        </div>
        <div class="ml-3 text-left">
          <p class={`text-sm font-semibold ${currentStep === 'confirm' ? 'text-blue-600' : 'text-gray-900'}`}>Confirm</p>
          <p class="text-xs text-gray-500">Review file selection</p>
        </div>
      </div>
      
      <!-- Connector -->
      <div class={`w-24 h-0.5 mx-4 ${currentStep === 'process' ? 'bg-green-600' : 'bg-gray-300'}`}></div>
      
      <!-- Step 3: Process & Results -->
      <div class="flex items-center">
        <div class={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${currentStep === 'process' ? 'bg-blue-600 border-blue-600 text-white' : 'bg-gray-200 border-gray-300 text-gray-400'}`}>
          {#if compressionResults.length > 0 && !compressing}
            <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path></svg>
          {:else}
            <span class="font-semibold">3</span>
          {/if}
        </div>
        <div class="ml-3 text-left">
          <p class={`text-sm font-semibold ${currentStep === 'process' ? 'text-blue-600' : 'text-gray-900'}`}>Process & Results</p>
          <p class="text-xs text-gray-500">Live progress and results</p>
        </div>
      </div>
    </div>
  </div>

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">{$appState.error}</div>
  {/if}

  <!-- Step 1: Scan & Select -->
  {#if currentStep === 'scan'}
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <div class="lg:col-span-1">
      <div class="bg-white rounded-lg shadow p-6">
        <h2 class="text-lg font-semibold mb-4">Compression Plugins</h2>
        <div class="space-y-3">
          {#each availablePlugins as plugin, index}
            <div class="border rounded-lg p-3 {activePlugins.has(plugin.name) ? 'border-blue-500 bg-blue-50' : 'border-gray-200 bg-gray-50'}">
              <div class="flex items-start justify-between mb-2">
                <label class="flex items-start cursor-pointer flex-1">
                  <input type="checkbox" checked={activePlugins.has(plugin.name)} onchange={() => togglePlugin(plugin.name)} class="mt-1 mr-2" />
                  <div class="flex-1 min-w-0">
                    <p class="font-medium text-sm truncate">{plugin.name}</p>
                    <p class="text-xs text-gray-600 mt-1">{plugin.description}</p>
                    <p class="text-xs text-gray-500 mt-1">v{plugin.version}</p>
                  </div>
                </label>
              </div>
              <div class="flex items-center justify-between mt-2 pt-2 border-t {activePlugins.has(plugin.name) ? 'border-blue-200' : 'border-gray-200'}">
                <span class="text-xs {activePlugins.has(plugin.name) ? 'text-blue-700' : 'text-gray-500'} font-medium">
                  Order: #{index + 1}
                  {#if !activePlugins.has(plugin.name)}
                    <span class="text-gray-400">(disabled)</span>
                  {/if}
                </span>
                <div class="flex gap-1">
                  <button onclick={() => movePluginUp(index)} disabled={index === 0} class="p-1 {activePlugins.has(plugin.name) ? 'text-blue-600 hover:bg-blue-100' : 'text-gray-400 hover:bg-gray-100'} rounded disabled:opacity-30" title="Move up" aria-label="Move plugin up">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"></path></svg>
                  </button>
                  <button onclick={() => movePluginDown(index)} disabled={index === availablePlugins.length - 1} class="p-1 {activePlugins.has(plugin.name) ? 'text-blue-600 hover:bg-blue-100' : 'text-gray-400 hover:bg-gray-100'} rounded disabled:opacity-30" title="Move down" aria-label="Move plugin down">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
        <div class="mt-4 p-3 bg-blue-50 border border-blue-200 rounded text-xs text-blue-800">
          <p class="font-semibold mb-1">Plugin Order</p>
          <p>Plugins are checked in the order shown. Reorder to customize behavior.</p>
        </div>
      </div>
    </div>

    <div class="lg:col-span-2">
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
              {#if $appState.scanPaths.length === 0}
                Configure scan paths first, then click the button below to scan for files
              {:else}
                Click the button below to scan {$appState.scanPaths.length} path{$appState.scanPaths.length > 1 ? 's' : ''} for compressible files
              {/if}
            </p>
            <button 
              onclick={handleScan} 
              disabled={scanning || $appState.scanPaths.length === 0} 
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
              <button onclick={handleScan} disabled={scanning} class="px-3 py-2 text-green-600 border border-green-600 rounded hover:bg-green-50 disabled:opacity-50 text-sm font-medium">
                {scanning ? 'Scanning...' : 'Rescan'}
              </button>
              <button onclick={goToConfirm} disabled={selectedFiles.size === 0} class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-sm font-medium">
                Next: Review ({selectedFiles.size} selected) →
              </button>
            </div>
          </div>
        
          <div class="mb-4 p-3 bg-blue-50 border border-blue-200 rounded">
            <div class="flex items-center justify-between">
              <div class="grid grid-cols-2 gap-4 text-sm flex-1">
                <div><span class="text-gray-600">Total Size:</span><span class="font-semibold ml-2">{formatSize(getTotalOriginalSize())}</span></div>
                <div><span class="text-gray-600">Estimated Savings:</span><span class="font-semibold ml-2 text-green-700">{formatSize(getTotalEstimatedSavings())}</span></div>
              </div>
              <button onclick={toggleAllFiles} class="ml-4 text-xs text-blue-600 hover:text-blue-800 font-medium whitespace-nowrap">
                {selectedFiles.size === compressibleFiles.length ? 'Deselect All' : 'Select All'}
              </button>
            </div>
          </div>
          <div class="border rounded-lg divide-y max-h-[500px] overflow-y-auto">
            {#each compressibleFiles as file}
              <div class="p-3 hover:bg-gray-50">
                <button onclick={() => toggleFileSelection(file.path)} class="flex items-start cursor-pointer w-full text-left">
                  <input type="checkbox" checked={selectedFiles.has(file.path)} class="mt-1 mr-3 flex-shrink-0 pointer-events-none" aria-label={`Select ${file.path}`} />
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-gray-900 truncate">{file.path}</p>
                    <div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-600">
                      <span>Original: {formatSize(file.original_size)}</span>
                      <span class="text-blue-600">→ {formatSize(file.estimated_compressed_size)}</span>
                      <span class="text-green-600 font-medium">Savings: {formatSize(file.estimated_savings)} ({(file.estimated_savings / file.original_size * 100).toFixed(1)}%)</span>
                      <span class="text-purple-600">Plugin: {file.plugin_name}</span>
                    </div>
                  </div>
                </button>
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
    </div>
  </div>
  {/if}

  <!-- Step 2: Confirm & Compress -->
  {#if currentStep === 'confirm'}
  <div class="bg-white rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-lg font-semibold">Review & Compress</h2>
      <button onclick={goToScan} class="text-sm text-blue-600 hover:text-blue-800 font-medium">← Back to Scan</button>
    </div>
    
    <div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
      <h3 class="text-sm font-semibold text-gray-800 mb-3">Summary</h3>
      <div class="grid grid-cols-2 gap-4 text-sm">
        <div><span class="text-gray-600">Selected Files:</span><span class="font-semibold ml-2">{selectedFiles.size} of {compressibleFiles.length}</span></div>
        <div><span class="text-gray-600">Worker Pool Size:</span><span class="font-semibold ml-2">{poolSize} concurrent worker{poolSize > 1 ? 's' : ''}</span></div>
        <div><span class="text-gray-600">Total Size:</span><span class="font-semibold ml-2">{formatSize(getTotalOriginalSize())}</span></div>
        <div><span class="text-gray-600">Estimated Savings:</span><span class="font-semibold ml-2 text-green-700">{formatSize(getTotalEstimatedSavings())}</span></div>
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
          bind:value={poolSize}
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
          <p>• {compressionResults.filter(r => r.success).length} successful</p>
          <p>• {compressionResults.filter(r => !r.success).length} failed</p>
        </div>
      </div>
    {/if}

    <!-- File List -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-semibold text-gray-700">Selected Files ({selectedFiles.size})</h3>
        <button onclick={toggleAllFiles} class="text-xs text-blue-600 hover:text-blue-800 font-medium">
          {selectedFiles.size === compressibleFiles.length ? 'Deselect All' : 'Select All'}
        </button>
      </div>
      <div class="border rounded-lg divide-y max-h-[400px] overflow-y-auto">
        {#each compressibleFiles as file}
          <div class="p-3 hover:bg-gray-50">
            <button onclick={() => toggleFileSelection(file.path)} class="flex items-start cursor-pointer w-full text-left">
              <input type="checkbox" checked={selectedFiles.has(file.path)} class="mt-1 mr-3 flex-shrink-0 pointer-events-none" aria-label={`Select ${file.path}`} />
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-gray-900 truncate">{file.path}</p>
                <div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-600">
                  <span>Original: {formatSize(file.original_size)}</span>
                  <span class="text-blue-600">→ {formatSize(file.estimated_compressed_size)}</span>
                  <span class="text-green-600 font-medium">Savings: {formatSize(file.estimated_savings)} ({(file.estimated_savings / file.original_size * 100).toFixed(1)}%)</span>
                  <span class="text-purple-600">Plugin: {file.plugin_name}</span>
                </div>
              </div>
            </button>
          </div>
        {/each}
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex items-center justify-end gap-3">
      <button onclick={goToScan} disabled={compressing} class="px-4 py-2 bg-gray-100 text-gray-700 rounded hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium">
        Cancel
      </button>
      <button onclick={handleCompress} disabled={compressing || selectedFiles.size === 0} class="px-6 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed text-sm font-medium flex items-center gap-2">
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
  {/if}

  <!-- Step 3: Process & Results -->
  {#if currentStep === 'process'}
  <div class="bg-white rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-lg font-semibold">
        {compressing ? 'Processing Files...' : 'Compression Complete'}
      </h2>
      {#if !compressing}
        <button onclick={startNewScan} class="text-sm text-blue-600 hover:text-blue-800 font-medium">
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
      </div>
    {/if}

    <!-- Summary Statistics (shown after completion) -->
    {#if !compressing && compressionResults.length > 0}
      <div class="grid grid-cols-3 gap-4 mb-6">
        <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
          <p class="text-xs text-gray-600 mb-1">Total Processed</p>
          <p class="text-2xl font-bold text-blue-600">{compressionResults.length}</p>
        </div>
        <div class="bg-green-50 border border-green-200 rounded-lg p-4">
          <p class="text-xs text-gray-600 mb-1">Successful</p>
          <p class="text-2xl font-bold text-green-600">{successResults.length}</p>
          <p class="text-xs text-green-700 mt-1">Saved: {formatSize(getTotalActualSavings())}</p>
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
            <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path></svg>
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
                      <p class="font-medium text-gray-900 truncate" title={result.path}>{result.path.split(/[\\/]/).pop()}</p>
                      <p class="text-xs text-gray-600 truncate" title={result.path}>{result.path.split(/[\\/]/).slice(0, -1).join('/')}</p>
                      <p class="text-xs text-purple-600 mt-1">Plugin: {result.plugin_name}</p>
                    </td>
                    <td class="px-3 py-2 text-right whitespace-nowrap">
                      <p class="font-semibold text-green-700">{formatSize(result.savings || 0)}</p>
                      <p class="text-xs text-gray-600">{((result.savings || 0) / (result.original_size || 1) * 100).toFixed(1)}%</p>
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
            <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path></svg>
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
                      <p class="font-medium text-gray-900 truncate" title={result.path}>{result.path.split(/[\\/]/).pop()}</p>
                      <p class="text-xs text-gray-600 truncate" title={result.path}>{result.path.split(/[\\/]/).slice(0, -1).join('/')}</p>
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
        <button onclick={startNewScan} class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm font-medium">
          Start New Scan
        </button>
      </div>
    {/if}
  </div>
  {/if}

</div>
