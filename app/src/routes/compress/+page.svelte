<script lang="ts">
  import { onMount } from "svelte";
  import { appState } from "$lib/stores/app";
  import {
    getCompressionPlugins,
    setPluginQuality,
    scanCompressibleFiles,
    compressFilesInPlace,
    getSkipCacheInfo,
    clearSkipCache,
    getConfig,
    type CompressionPlugin,
    type CompressibleFile,
    type RejectedFile,
    type InPlaceCompressionResult,
  } from "$lib/api";
  import {
    loadFromStorage,
    saveToStorage,
    loadFromSession,
    saveToSession,
    storageKeys,
    sessionKeys,
  } from "$lib/utils/storage";
  import PluginSelector from "./PluginSelector.svelte";
  import StepIndicator from "./StepIndicator.svelte";
  import ScanStep from "./ScanStep.svelte";
  import ConfirmStep from "./ConfirmStep.svelte";
  import ProcessStep from "./ProcessStep.svelte";

  interface CompressSettings {
    order: string[];
    active: string[];
    poolSize: number;
    backup?: boolean;
  }

  // Session-cached workflow state so navigating away from a multi-step
  // compression flow and back restores the scan, selection, step, and results.
  // Plugin config (availablePlugins/activePlugins) is reloaded fresh in onMount,
  // so it is intentionally not part of this cache.
  //
  // Split into two session keys/effects on purpose: scan state (compressibleFiles/
  // rejectedFiles/selectedFiles) is written once per scan, while progress state
  // (compressed/skipped/failed results, counts) changes once per processed file.
  // Bundling them into one object would re-serialize the (potentially hundreds-of-thousands-
  // entry) scan results on every single file completion during compression —
  // that repeated full-array JSON.stringify was the main cause of the compress
  // page running out of memory on very large batches.
  type Step = 'scan' | 'confirm' | 'process';
  interface CompressScanCache {
    compressibleFiles: CompressibleFile[];
    rejectedFiles: RejectedFile[];
    selectedFiles: string[];
    currentStep: Step;
  }
  interface CompressProgressCache {
    compressedResults: InPlaceCompressionResult[];
    skippedResults: InPlaceCompressionResult[];
    // Unlike compressed/skipped, failures are kept in full (not capped): they
    // are usually rare and are the ones a user actually needs to inspect
    // individually. See recordResults() for how this stays cheap even at scale.
    failedResults: InPlaceCompressionResult[];
    compressedCount: number;
    skippedCount: number;
    totalActualSavings: number;
  }
  const cachedScan = loadFromSession<CompressScanCache | null>(sessionKeys.COMPRESS_RESULT, null);
  const cachedProgress = loadFromSession<CompressProgressCache | null>(sessionKeys.COMPRESS_PROGRESS, null);

  // Caps how many compressed/skipped results we keep around for the results
  // tables and the session-restore cache. compressedCount/skippedCount below
  // are running totals that are NOT capped, so stats stay accurate even for
  // hundreds of thousands of files while memory use for the kept list stays
  // bounded. Failed results are handled separately and are not capped.
  const MAX_KEPT_COMPRESSION_RESULTS = 500;

  let availablePlugins = $state<CompressionPlugin[]>([]);
  let activePlugins = $state<Set<string>>(new Set());
  let scanning = $state(false);
  let compressibleFiles = $state<CompressibleFile[]>(cachedScan?.compressibleFiles ?? []);
  let rejectedFiles = $state<RejectedFile[]>(cachedScan?.rejectedFiles ?? []);
  let selectedFiles = $state<Set<string>>(new Set(cachedScan?.selectedFiles ?? []));
  let compressing = $state(false);
  let compressedResults = $state<InPlaceCompressionResult[]>(cachedProgress?.compressedResults ?? []);
  let skippedResults = $state<InPlaceCompressionResult[]>(cachedProgress?.skippedResults ?? []);
  let failedResults = $state<InPlaceCompressionResult[]>(cachedProgress?.failedResults ?? []);
  let compressedCount = $state(cachedProgress?.compressedCount ?? 0);
  let skippedCount = $state(cachedProgress?.skippedCount ?? 0);
  let failedCount = $derived(failedResults.length);
  let totalActualSavings = $state(cachedProgress?.totalActualSavings ?? 0);

  // Worker pool configuration
  let poolSize = $state(2);
  let createBackup = $state(true);
  let processedCount = $state(0);
  let totalToProcess = $state(0);
  let currentlyProcessing = $state<string[]>([]);

  // Step management (Step type declared above with the session cache)
  let currentStep = $state<Step>(cachedScan?.currentStep ?? 'scan');

  $effect(() => {
    saveToSession<CompressScanCache>(sessionKeys.COMPRESS_RESULT, {
      compressibleFiles,
      rejectedFiles,
      selectedFiles: Array.from(selectedFiles),
      currentStep,
    });
  });

  // Progress is saved separately and throttled: during compression this
  // effect's dependencies change once per file, and doing a full
  // JSON.stringify + sessionStorage write on every single file would still
  // burn CPU/memory even though the payload itself is now bounded.
  let lastProgressSaveAt = 0;
  let progressSaveTimer: ReturnType<typeof setTimeout> | null = null;
  const PROGRESS_SAVE_INTERVAL_MS = 500;

  function scheduleProgressSave(snapshot: CompressProgressCache) {
    const now = Date.now();
    const elapsed = now - lastProgressSaveAt;
    if (progressSaveTimer) {
      clearTimeout(progressSaveTimer);
      progressSaveTimer = null;
    }
    if (elapsed >= PROGRESS_SAVE_INTERVAL_MS) {
      lastProgressSaveAt = now;
      saveToSession<CompressProgressCache>(sessionKeys.COMPRESS_PROGRESS, snapshot);
      return;
    }
    progressSaveTimer = setTimeout(() => {
      lastProgressSaveAt = Date.now();
      saveToSession<CompressProgressCache>(sessionKeys.COMPRESS_PROGRESS, snapshot);
    }, PROGRESS_SAVE_INTERVAL_MS - elapsed);
  }

  $effect(() => {
    scheduleProgressSave({
      compressedResults,
      skippedResults,
      failedResults,
      compressedCount,
      skippedCount,
      totalActualSavings,
    });
  });

  let totalResultsCount = $derived(compressedCount + skippedCount + failedCount);

  // Skip memory: files remembered as "compression produced no size reduction"
  let skipCacheEntries = $state(0);

  async function refreshSkipCacheInfo() {
    try {
      skipCacheEntries = (await getSkipCacheInfo()).entries;
    } catch (err) {
      console.error("Failed to load skip cache info:", err);
    }
  }

  async function handleClearSkipCache() {
    try {
      await clearSkipCache();
      await refreshSkipCacheInfo();
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : "Failed to clear skip memory";
    }
  }

  onMount(async () => {
    // The Settings page's "keep a backup" default seeds this when the user
    // hasn't made a per-compress choice yet.
    let backupDefault = true;
    try {
      backupDefault = (await getConfig()).default_compress_backup;
      createBackup = backupDefault;
    } catch {
      // Fall back to the in-component default
    }
    try {
      const plugins = await getCompressionPlugins();
      const saved = loadFromStorage<CompressSettings | null>(storageKeys.COMPRESS_SETTINGS, null);

      if (saved) {
        const rank = new Map(saved.order.map((name, i) => [name, i]));
        plugins.sort(
          (a, b) => (rank.get(a.name) ?? saved.order.length) - (rank.get(b.name) ?? saved.order.length)
        );
        activePlugins = new Set(plugins.filter(p => saved.active.includes(p.name)).map(p => p.name));
        poolSize = saved.poolSize ?? 2;
        createBackup = saved.backup ?? backupDefault;
        // Quality comes back from the backend already (seeded from config),
        // so it is no longer restored from localStorage here.
      } else {
        activePlugins = new Set(plugins.map(p => p.name));
      }
      availablePlugins = plugins;
    } catch (err) {
      console.error("Failed to load plugins:", err);
      $appState.error = "Failed to load compression plugins";
    }
    refreshSkipCacheInfo();
  });

  function persistSettings() {
    // Quality is persisted backend-side (config) via setPluginQuality; only
    // UI preferences (order, active set, pool size, backup) live here.
    saveToStorage<CompressSettings>(storageKeys.COMPRESS_SETTINGS, {
      order: availablePlugins.map(p => p.name),
      active: Array.from(activePlugins),
      poolSize,
      backup: createBackup,
    });
  }

  function togglePlugin(pluginName: string) {
    if (activePlugins.has(pluginName)) {
      activePlugins.delete(pluginName);
    } else {
      activePlugins.add(pluginName);
    }
    activePlugins = new Set(activePlugins);
    persistSettings();
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
      persistSettings();
    }
  }

  function movePluginDown(index: number) {
    if (index < availablePlugins.length - 1) {
      const newPlugins = [...availablePlugins];
      [newPlugins[index], newPlugins[index + 1]] = [newPlugins[index + 1], newPlugins[index]];
      availablePlugins = newPlugins;
      persistSettings();
    }
  }

  async function handleQualityChange(pluginName: string, quality: number) {
    availablePlugins = availablePlugins.map(p =>
      p.name === pluginName ? { ...p, quality } : p
    );
    persistSettings();
    try {
      await setPluginQuality(pluginName, quality);
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : "Failed to update plugin quality";
    }
  }

  function handlePoolSizeChange(size: number) {
    poolSize = size;
    persistSettings();
  }

  function handleCreateBackupChange(value: boolean) {
    createBackup = value;
    persistSettings();
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
    appState.setBusy(true);
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
      appState.setBusy(false);
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

  // Updates the running totals (unbounded, always accurate) and appends to the
  // capped display lists (bounded, for the results tables / session restore).
  // Splitting these avoids the O(n^2) "copy the whole array on every file"
  // pattern that made compressing hundreds of thousands of files OOM the UI.
  //
  // Failed results are the exception: they are kept in full since they're the
  // ones worth inspecting individually and are usually rare. `.push()` mutates
  // the $state array in place (Svelte 5 deeply proxies arrays), so appending
  // stays O(1) amortized instead of copying the whole array like `[...arr, x]`
  // would — that's what keeps this safe even if a run fails on most files.
  function recordResults(newResults: InPlaceCompressionResult[]) {
    for (const result of newResults) {
      if (result.status === 'compressed') {
        compressedCount++;
        totalActualSavings += result.savings || 0;
        if (compressedResults.length < MAX_KEPT_COMPRESSION_RESULTS) {
          compressedResults.push(result);
        }
      } else if (result.status === 'skipped') {
        skippedCount++;
        if (skippedResults.length < MAX_KEPT_COMPRESSION_RESULTS) {
          skippedResults.push(result);
        }
      } else if (result.status === 'failed') {
        failedResults.push(result);
      }
    }
  }

  async function handleCompress() {
    if (selectedFiles.size === 0) {
      $appState.error = "Please select files to compress";
      return;
    }

    currentStep = 'process';
    compressing = true;
    appState.setBusy(true);
    $appState.error = null;
    compressedResults = [];
    skippedResults = [];
    failedResults = [];
    compressedCount = 0;
    skippedCount = 0;
    totalActualSavings = 0;
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
            const results = await compressFilesInPlace([filePath], plugins, createBackup);
            recordResults(results);
          } catch (err) {
            console.error(`Failed to compress ${filePath}:`, err);
            recordResults([{
              path: filePath,
              status: 'failed',
              success: false,
              error: err instanceof Error ? err.message : "Unknown error"
            }]);
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
      appState.setBusy(false);
      currentlyProcessing = [];
      refreshSkipCacheInfo();
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
    compressedResults = [];
    skippedResults = [];
    failedResults = [];
    compressedCount = 0;
    skippedCount = 0;
    totalActualSavings = 0;
    compressibleFiles = [];
    rejectedFiles = [];
    selectedFiles.clear();
    processedCount = 0;
    totalToProcess = 0;
    currentlyProcessing = [];
  }
</script>

<div class="h-full flex flex-col w-full">
  <div class="mb-4 shrink-0">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Smart Compression Manager</h1>
    <p class="text-gray-600">Manage plugins, scan files, and compress with backups</p>
  </div>

  <div class="shrink-0">
    <StepIndicator
      {currentStep}
      hasScannedFiles={compressibleFiles.length > 0}
      hasResults={totalResultsCount > 0}
      isCompressing={compressing}
    />
  </div>

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4 shrink-0">
      {$appState.error}
    </div>
  {/if}

  {#if currentStep === 'scan'}
    <div class="flex-1 min-h-0 overflow-y-auto">
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div class="lg:col-span-1 min-w-0">
          <PluginSelector
            plugins={availablePlugins}
            {activePlugins}
            onToggle={togglePlugin}
            onMoveUp={movePluginUp}
            onMoveDown={movePluginDown}
            onQualityChange={handleQualityChange}
          />

          <!-- Skip memory: remembered no-size-reduction results -->
          <div class="mt-4 bg-white rounded-lg shadow p-4 flex items-center justify-between gap-2">
            <div class="min-w-0">
              <p class="text-sm font-medium text-gray-700">Skip memory</p>
              <p class="text-xs text-gray-600 mt-0.5">
                {skipCacheEntries} file{skipCacheEntries !== 1 ? 's' : ''} remembered as not compressible at current settings; scans exclude them until they change.
              </p>
            </div>
            <button
              onclick={handleClearSkipCache}
              disabled={skipCacheEntries === 0}
              class="px-3 py-1.5 text-xs font-medium text-gray-600 border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed whitespace-nowrap"
            >
              Clear
            </button>
          </div>
        </div>

        <div class="lg:col-span-2 min-w-0">
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
    </div>
  {/if}

  {#if currentStep === 'confirm'}
    <div class="flex-1 min-h-0 overflow-y-auto">
      <ConfirmStep
        {compressibleFiles}
        {selectedFiles}
        {poolSize}
        {createBackup}
        {compressing}
        {processedCount}
        {totalToProcess}
        {compressedCount}
        {skippedCount}
        {failedCount}
        onToggleFile={toggleFileSelection}
        onToggleAll={toggleAllFiles}
        onBack={() => currentStep = 'scan'}
        onCompress={handleCompress}
        onPoolSizeChange={handlePoolSizeChange}
        onCreateBackupChange={handleCreateBackupChange}
      />
    </div>
  {/if}

  {#if currentStep === 'process'}
    <div class="flex-1 min-h-0 overflow-y-auto">
      <ProcessStep
        {compressedResults}
        {skippedResults}
        {failedResults}
        {compressedCount}
        {skippedCount}
        {failedCount}
        {totalActualSavings}
        {totalResultsCount}
        {compressing}
        {processedCount}
        {totalToProcess}
        {poolSize}
        {currentlyProcessing}
        onStartNew={startNewScan}
      />
    </div>
  {/if}
</div>
