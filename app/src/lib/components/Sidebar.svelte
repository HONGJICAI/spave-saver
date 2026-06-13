<script lang="ts">
  import { onMount } from 'svelte';
  // @ts-ignore
  import { base } from '$app/paths';
  // @ts-ignore
  import { page } from '$app/stores';
  import { appState } from '$lib/stores/app';

  interface NavItem {
    name: string;
    path: string;
    icon: string;
  }
  
  const navItems: NavItem[] = [
    { name: 'Home', path: '/', icon: '🏠' },
    { name: 'Statistics', path: '/stats', icon: '📊' },
    { name: 'Duplicates', path: '/duplicates', icon: '📋' },
    { name: 'Similar', path: '/similar', icon: '🖼️' },
    { name: 'Empty', path: '/empty', icon: '📄' },
    { name: 'Broken', path: '/broken', icon: '🚫' },
    { name: 'Compress', path: '/compress', icon: '🗜️' },
    { name: 'Settings', path: '/settings', icon: '⚙️' }
  ];

  let mode = 'Loading...';

  onMount(() => {
    // @ts-ignore
    if (window.__TAURI_INTERNALS__) {
      mode = 'Desktop Mode';
    } else {
      mode = 'Web Mode';
    }
  });

  // Helper to check if a path is active, handling base path
  function isActive(currentPath: string, itemPath: string): boolean {
    // Normalize paths by removing trailing slashes for comparison
    const normalize = (p: string) => p.endsWith('/') && p.length > 1 ? p.slice(0, -1) : p;
    
    const targetPath = `${base}${itemPath}`;
    return normalize(currentPath) === normalize(targetPath);
  }
</script>

<aside class="w-64 bg-gray-800 text-white flex flex-col">
  <div class="p-4 border-b border-gray-700">
    <h1 class="text-2xl font-bold">💾 Space-Saver</h1>
    <p class="text-sm text-gray-400 mt-1">Disk Space Manager</p>
  </div>
  
  <!-- While an operation is in flight, navigation is blocked (see the
       beforeNavigate guard in +layout.svelte). We grey the links and swallow
       clicks so the disabled state is visible, not just silently inert. -->
  <nav class="flex-1 p-4" aria-busy={$appState.busy}>
    <ul class="space-y-2">
      {#each navItems as item}
        <li>
          <a
            href="{base}{item.path}"
            aria-disabled={$appState.busy}
            tabindex={$appState.busy ? -1 : undefined}
            onclick={(e) => { if ($appState.busy) e.preventDefault(); }}
            class="flex items-center gap-3 px-4 py-3 rounded-lg transition-colors {
              isActive($page.url.pathname, item.path)
                ? 'bg-blue-600 text-white'
                : 'hover:bg-gray-700'
            } {$appState.busy ? 'opacity-40 cursor-not-allowed pointer-events-none' : ''}"
          >
            <span class="text-xl">{item.icon}</span>
            <span>{item.name}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>
  {#if $appState.busy}
    <p class="px-4 -mt-2 mb-2 text-xs text-gray-400">Navigation paused while an operation is running…</p>
  {/if}
  
  <div class="p-4 border-t border-gray-700 text-sm text-gray-400">
    <p>Mode: <span class="text-green-400" id="app-mode">{mode}</span></p>
    <p class="mt-1">Version 0.1.0</p>
  </div>
</aside>

<style>
  aside {
    height: 100%;
    flex-shrink: 0;
  }
</style>
