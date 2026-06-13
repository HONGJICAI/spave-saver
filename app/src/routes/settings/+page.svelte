<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getConfig,
    setConfig,
    detectTools,
    getCompressionPlugins,
    setPluginQuality,
    getSkipCacheInfo,
    clearSkipCache,
    type AppConfig,
    type ToolStatus,
  } from '$lib/api';
  import type { CompressionPlugin } from '$lib/api';
  import { formatSize } from '$lib/utils/format';

  let config = $state<AppConfig | null>(null);
  let tools = $state<ToolStatus[]>([]);
  let plugins = $state<CompressionPlugin[]>([]);
  let skipCacheEntries = $state(0);

  let loading = $state(true);
  let detectingTools = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let saved = $state(false);

  onMount(async () => {
    try {
      const [cfg, info] = await Promise.all([getConfig(), getSkipCacheInfo()]);
      config = cfg;
      skipCacheEntries = info.entries;
      plugins = await getCompressionPlugins();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
    // Tool detection can be slow (spawns processes); load it separately so the
    // rest of the page renders immediately.
    void runDetectTools();
  });

  async function runDetectTools() {
    detectingTools = true;
    try {
      tools = await detectTools();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      detectingTools = false;
    }
  }

  async function handleSave() {
    if (!config) return;
    saving = true;
    saved = false;
    error = null;
    try {
      // setConfig rejects with the backend's plain error string on bad input
      config = await setConfig(config);
      saved = true;
      setTimeout(() => (saved = false), 2500);
    } catch (err) {
      error = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }

  async function handlePluginQuality(plugin: CompressionPlugin) {
    error = null;
    try {
      await setPluginQuality(plugin.name, plugin.quality ?? 85);
    } catch (err) {
      error = typeof err === 'string' ? err : err instanceof Error ? err.message : String(err);
    }
  }

  async function handleClearSkipCache() {
    try {
      await clearSkipCache();
      skipCacheEntries = (await getSkipCacheInfo()).entries;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<div class="p-6 max-w-3xl">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Settings</h1>
    <p class="text-gray-600">Configure scan defaults, behaviour and inspect the environment</p>
  </div>

  {#if error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      {error}
    </div>
  {/if}

  {#if loading}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">Loading settings…</div>
  {:else if config}
    <!-- Environment / external tools -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h2 class="text-xl font-semibold text-gray-900">Environment</h2>
          <p class="text-sm text-gray-600">Optional command-line tools detected on your PATH</p>
        </div>
        <button
          onclick={runDetectTools}
          disabled={detectingTools}
          class="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
        >
          {detectingTools ? 'Detecting…' : 'Re-detect'}
        </button>
      </div>

      <div class="space-y-3">
        {#each tools as tool}
          <div class="flex items-start justify-between border border-gray-100 rounded-lg p-3">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-mono font-medium text-gray-900">{tool.name}</span>
                {#if tool.available}
                  <span class="px-2 py-0.5 text-xs bg-green-100 text-green-800 rounded">Available</span>
                {:else}
                  <span class="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded">Not found</span>
                {/if}
              </div>
              <p class="text-sm text-gray-600 mt-1">{tool.purpose}</p>
              {#if tool.version}
                <p class="text-xs text-gray-500 mt-1 truncate" title={tool.version}>{tool.version}</p>
              {/if}
              {#if tool.path}
                <p class="text-xs text-gray-400 font-mono truncate" title={tool.path}>{tool.path}</p>
              {/if}
            </div>
          </div>
        {/each}
        {#if tools.length === 0 && !detectingTools}
          <p class="text-sm text-gray-500">No tools probed.</p>
        {/if}
      </div>
    </section>

    <!-- Scan defaults -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-4">Scan defaults</h2>

      <label class="block text-sm font-medium text-gray-700 mb-2" for="threshold">
        Image similarity threshold: {(config.image_similarity_threshold * 100).toFixed(0)}%
      </label>
      <input
        id="threshold"
        type="range"
        min="0.5"
        max="1.0"
        step="0.05"
        bind:value={config.image_similarity_threshold}
        class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
      />
      <p class="text-xs text-gray-500 mt-1">
        Default sensitivity for the Similar Images finder. Lower = more matches.
      </p>
    </section>

    <!-- Behaviour -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-4">Behaviour</h2>

      <fieldset class="mb-4">
        <legend class="text-sm font-medium text-gray-700 mb-2">Default delete mode</legend>
        <label class="flex items-start gap-2 mb-2">
          <input type="radio" bind:group={config.default_delete_mode} value="trash" class="mt-1" />
          <span>
            <span class="font-medium">Move to Trash</span>
            <span class="block text-xs text-gray-500">Recoverable from the system recycle bin</span>
          </span>
        </label>
        <label class="flex items-start gap-2">
          <input type="radio" bind:group={config.default_delete_mode} value="permanent" class="mt-1" />
          <span>
            <span class="font-medium">Delete permanently</span>
            <span class="block text-xs text-gray-500">Removed from disk immediately, cannot be undone</span>
          </span>
        </label>
      </fieldset>

      <label class="flex items-center gap-2">
        <input type="checkbox" bind:checked={config.default_compress_backup} />
        <span>
          <span class="font-medium">Keep a backup when compressing</span>
          <span class="block text-xs text-gray-500">Keeps the original as <code>&lt;name&gt;.bak</code> by default</span>
        </span>
      </label>
    </section>

    <!-- Compression plugins -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-4">Compression quality</h2>
      <div class="space-y-4">
        {#each plugins as plugin}
          <div class="border border-gray-100 rounded-lg p-3">
            <p class="font-medium text-gray-900">{plugin.name}</p>
            <p class="text-xs text-gray-500 mb-2">{plugin.description}</p>
            <div class="flex items-center gap-3">
              <input
                type="range"
                min="1"
                max="100"
                step="1"
                bind:value={plugin.quality}
                class="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
              />
              <span class="w-10 text-sm text-gray-700 text-right">{plugin.quality}</span>
              <button
                onclick={() => handlePluginQuality(plugin)}
                class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                Apply
              </button>
            </div>
          </div>
        {/each}
      </div>
    </section>

    <!-- Maintenance -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-4">Maintenance</h2>
      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium text-gray-900">Compression skip cache</p>
          <p class="text-sm text-gray-500">
            {skipCacheEntries} file{skipCacheEntries === 1 ? '' : 's'} remembered as "no size reduction"
          </p>
        </div>
        <button
          onclick={handleClearSkipCache}
          disabled={skipCacheEntries === 0}
          class="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
        >
          Clear cache
        </button>
      </div>
    </section>

    <!-- Read-only environment info (applied at startup) -->
    <section class="bg-white rounded-lg shadow p-6 mb-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-1">System</h2>
      <p class="text-sm text-gray-500 mb-4">Applied at startup — restart to take effect</p>
      <dl class="text-sm divide-y divide-gray-100">
        <div class="flex justify-between py-2 gap-4">
          <dt class="text-gray-600">Database</dt>
          <dd class="font-mono text-gray-900 truncate" title={config.database_path}>{config.database_path}</dd>
        </div>
        <div class="flex justify-between py-2 gap-4">
          <dt class="text-gray-600">Cache directory</dt>
          <dd class="font-mono text-gray-900 truncate" title={config.cache_dir}>{config.cache_dir}</dd>
        </div>
        <div class="flex justify-between py-2 gap-4">
          <dt class="text-gray-600">Log level</dt>
          <dd class="font-mono text-gray-900">{config.log_level}</dd>
        </div>
        <div class="flex justify-between py-2 gap-4">
          <dt class="text-gray-600">Max concurrent tasks</dt>
          <dd class="font-mono text-gray-900">{config.max_concurrent_tasks}</dd>
        </div>
        <div class="flex justify-between py-2 gap-4">
          <dt class="text-gray-600">Hash algorithm</dt>
          <dd class="font-mono text-gray-900">{config.hash_algorithm}</dd>
        </div>
      </dl>
    </section>

    <!-- Save bar -->
    <div class="flex items-center gap-3">
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300"
      >
        {saving ? 'Saving…' : 'Save settings'}
      </button>
      {#if saved}
        <span class="text-green-600 text-sm">Saved ✓</span>
      {/if}
    </div>
  {/if}
</div>
