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
  import PluginSelector from "./PluginSelector.svelte";
  import StepIndicator from "./StepIndicator.svelte";
  import ScanStep from "./ScanStep.svelte";
  import ConfirmStep from "./ConfirmStep.svelte";
  import ProcessStep from "./ProcessStep.svelte";

  let availablePlugins = $state<CompressionPlugin[]>([]);
  let activePlugins = $state<Set<string>>(new Set());
  let scanning = $state(false);
  let compressibleFiles = $state<CompressibleFile[]>([]);
  let rejectedFiles = $state<RejectedFile[]>([]);
  let selectedFiles = $state<Set<string>>(new Set());
  let compressing = $state(false);
  let compressionResults = $state<InPlaceCompressionResult[]>([]);
  
  // Worker pool configuration
  let poolSize = $state(2);
  let processedCount = $state(0);
  let totalToProcess = $state(0);
  let currentlyProcessing = $state<string[]>([]);
  
  // Step management
  type Step = 'scan' | 'confirm' | 'process';
  let currentStep = $state<Step>('scan');

  onMount(async () => {
    try {
      availablePlugins = await getCompressionPlugins();
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
    
    currentStep = 'process';
    compressing = true;
    $appState.error = null;
    compressionResults = [];
    processedCount = 0;
    currentlyProcessing = [];
    
    try {
      const filesToCompress = Array.from(selectedFiles);
      totalToProcess = filesToCompress.length;
      const plugins = getActivePlugins();
      
      const pool: Promise<void>[] = [];
      let fileIndex = 0;
      
      const worker = async () => {
        while (fileIndex < filesToCompress.length) {
          const currentIndex = fileIndex++;
          const filePath = filesToCompress[currentIndex];
          
          // Add to currently processing
          currentlyProcessing = [...currentlyProcessing, filePath];
          
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
          
          // Remove from currently processing
          currentlyProcessing = currentlyProcessing.filter(p => p !== filePath);
          processedCount++;
          await new Promise(resolve => setTimeout(resolve, 10));
        }
      };
      
      for (let i = 0; i < poolSize; i++) {
        pool.push(worker());
      }
      
      await Promise.all(pool);
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : "Failed to compress files";
    } finally {
      compressing = false;
      currentlyProcessing = [];
    }
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
    compressionResults = [];
    compressibleFiles = [];
    rejectedFiles = [];
    selectedFiles.clear();
    processedCount = 0;
    totalToProcess = 0;
    currentlyProcessing = [];
  }
</script>

<div class="p-6 max-w-[1600px] mx-auto">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Smart Compression Manager</h1>
    <p class="text-gray-600">Manage plugins, scan files, and compress with backups</p>
  </div>

  <StepIndicator 
    {currentStep}
    hasScannedFiles={compressibleFiles.length > 0}
    hasResults={compressionResults.length > 0}
    isCompressing={compressing}
  />

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      {$appState.error}
    </div>
  {/if}

  {#if currentStep === 'scan'}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-1">
        <PluginSelector 
          plugins={availablePlugins}
          {activePlugins}
          onToggle={togglePlugin}
          onMoveUp={movePluginUp}
          onMoveDown={movePluginDown}
        />
      </div>

      <div class="lg:col-span-2">
        <ScanStep 
          {compressibleFiles}
          {rejectedFiles}
          {selectedFiles}
          {scanning}
          scanPaths={$appState.scanPaths}
          onScan={handleScan}
          onToggleFile={toggleFileSelection}
          onToggleAll={toggleAllFiles}
          onNext={goToConfirm}
        />
      </div>
    </div>
  {/if}

  {#if currentStep === 'confirm'}
    <ConfirmStep 
      {compressibleFiles}
      {selectedFiles}
      {poolSize}
      {compressing}
      {processedCount}
      {totalToProcess}
      successCount={compressionResults.filter(r => r.success).length}
      errorCount={compressionResults.filter(r => !r.success).length}
      onToggleFile={toggleFileSelection}
      onToggleAll={toggleAllFiles}
      onBack={() => currentStep = 'scan'}
      onCompress={handleCompress}
      onPoolSizeChange={(size) => poolSize = size}
    />
  {/if}

  {#if currentStep === 'process'}
    <ProcessStep 
      results={compressionResults}
      {compressing}
      {processedCount}
      {totalToProcess}
      {poolSize}
      {currentlyProcessing}
      onStartNew={startNewScan}
    />
  {/if}
</div>
