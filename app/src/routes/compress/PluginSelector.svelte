<script lang="ts">
  import type { CompressionPlugin } from "$lib/api";

  type Props = {
    plugins: CompressionPlugin[];
    activePlugins: Set<string>;
    onToggle: (pluginName: string) => void;
    onMoveUp: (index: number) => void;
    onMoveDown: (index: number) => void;
  };

  let { plugins, activePlugins, onToggle, onMoveUp, onMoveDown }: Props = $props();
</script>

<div class="bg-white rounded-lg shadow p-6">
  <h2 class="text-lg font-semibold mb-4">Compression Plugins</h2>
  <div class="space-y-3">
    {#each plugins as plugin, index}
      <div class="border rounded-lg p-3 {activePlugins.has(plugin.name) ? 'border-blue-500 bg-blue-50' : 'border-gray-200 bg-gray-50'}">
        <div class="flex items-start justify-between mb-2">
          <label class="flex items-start cursor-pointer flex-1">
            <input 
              type="checkbox" 
              checked={activePlugins.has(plugin.name)} 
              onchange={() => onToggle(plugin.name)} 
              class="mt-1 mr-2" 
            />
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
            <button 
              onclick={() => onMoveUp(index)} 
              disabled={index === 0} 
              class="p-1 {activePlugins.has(plugin.name) ? 'text-blue-600 hover:bg-blue-100' : 'text-gray-400 hover:bg-gray-100'} rounded disabled:opacity-30" 
              title="Move up" 
              aria-label="Move plugin up"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"></path>
              </svg>
            </button>
            <button 
              onclick={() => onMoveDown(index)} 
              disabled={index === plugins.length - 1} 
              class="p-1 {activePlugins.has(plugin.name) ? 'text-blue-600 hover:bg-blue-100' : 'text-gray-400 hover:bg-gray-100'} rounded disabled:opacity-30" 
              title="Move down" 
              aria-label="Move plugin down"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
              </svg>
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
