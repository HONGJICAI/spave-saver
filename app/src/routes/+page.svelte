<script lang="ts">
  import { appState } from '$lib/stores/app';
  
  interface QuickAction {
    name: string;
    description: string;
    path: string;
    icon: string;
    color: string;
  }
  
  const quickActions: QuickAction[] = [
    {
      name: 'Statistics',
      description: 'Analyze storage usage and file distribution',
      path: '/stats',
      icon: '📊',
      color: 'bg-blue-500 hover:bg-blue-600'
    },
    {
      name: 'Find Duplicates',
      description: 'Detect and remove duplicate files',
      path: '/duplicates',
      icon: '📋',
      color: 'bg-purple-500 hover:bg-purple-600'
    },
    {
      name: 'Similar Images',
      description: 'Find visually similar images',
      path: '/similar',
      icon: '🖼️',
      color: 'bg-green-500 hover:bg-green-600'
    },
    {
      name: 'Empty Files',
      description: 'Find and clean up empty files',
      path: '/empty',
      icon: '📄',
      color: 'bg-yellow-500 hover:bg-yellow-600'
    },
    {
      name: 'Compress',
      description: 'Compress files to save space',
      path: '/compress',
      icon: '🗜️',
      color: 'bg-red-500 hover:bg-red-600'
    }
  ];
  
  $: hasPathsConfigured = $appState.scanPaths.length > 0;
</script>

<svelte:head>
  <title>Home - Space-Saver</title>
</svelte:head>

<div class="max-w-5xl mx-auto">
  <!-- Hero Section -->
  <div class="text-center mb-12">
    <div class="text-6xl mb-4">💾</div>
    <h1 class="text-4xl font-bold text-gray-900 mb-4">Welcome to Space-Saver</h1>
    <p class="text-xl text-gray-600 max-w-2xl mx-auto">
      Your intelligent disk space manager. Analyze, optimize, and reclaim storage space with powerful tools.
    </p>
  </div>
  
  <!-- Status Card -->
  <div class="bg-white rounded-xl shadow-lg p-6 mb-8">
    <div class="flex items-center gap-4">
      <div class="text-4xl">
        {hasPathsConfigured ? '✅' : '📁'}
      </div>
      <div class="flex-1">
        <h2 class="text-lg font-semibold text-gray-900">
          {hasPathsConfigured ? 'Ready to Scan' : 'Get Started'}
        </h2>
        <p class="text-gray-600">
          {#if hasPathsConfigured}
            You have <span class="font-medium text-blue-600">{$appState.scanPaths.length}</span> path(s) configured. Choose an action below to begin.
          {:else}
            Add a scan path above to start analyzing your storage.
          {/if}
        </p>
      </div>
      {#if hasPathsConfigured}
        <a
          href="/stats"
          class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
        >
          📊 Analyze Now
        </a>
      {/if}
    </div>
  </div>
  
  <!-- Quick Actions Grid -->
  <h2 class="text-2xl font-bold text-gray-900 mb-6">Quick Actions</h2>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
    {#each quickActions as action}
      <a
        href={action.path}
        class="group bg-white rounded-xl shadow-md hover:shadow-lg transition-all p-6 border-2 border-transparent hover:border-gray-200"
      >
        <div class="flex items-start gap-4">
          <div class="{action.color} text-white p-3 rounded-xl text-2xl transition-colors">
            {action.icon}
          </div>
          <div class="flex-1">
            <h3 class="font-semibold text-gray-900 group-hover:text-blue-600 transition-colors">
              {action.name}
            </h3>
            <p class="text-sm text-gray-600 mt-1">
              {action.description}
            </p>
          </div>
        </div>
      </a>
    {/each}
  </div>
  
  <!-- Features Section -->
  <div class="bg-gradient-to-br from-gray-50 to-gray-100 rounded-xl p-8">
    <h2 class="text-2xl font-bold text-gray-900 mb-6 text-center">Features</h2>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <div class="text-center">
        <div class="text-3xl mb-3">⚡</div>
        <h3 class="font-semibold text-gray-900 mb-2">Fast Scanning</h3>
        <p class="text-sm text-gray-600">Blazing fast file analysis powered by Rust</p>
      </div>
      <div class="text-center">
        <div class="text-3xl mb-3">🔒</div>
        <h3 class="font-semibold text-gray-900 mb-2">Safe Operations</h3>
        <p class="text-sm text-gray-600">Preview changes before making any modifications</p>
      </div>
      <div class="text-center">
        <div class="text-3xl mb-3">📈</div>
        <h3 class="font-semibold text-gray-900 mb-2">Detailed Reports</h3>
        <p class="text-sm text-gray-600">Comprehensive statistics and visualizations</p>
      </div>
    </div>
  </div>
</div>
