<script lang="ts">
  export let label: string;
  export let value: string | number;
  export let icon: string = '📊';
  export let color: 'blue' | 'green' | 'yellow' | 'red' | 'purple' | 'gray' = 'blue';
  
  const colorClasses = {
    blue: 'bg-blue-50 text-blue-600',
    green: 'bg-green-50 text-green-600',
    yellow: 'bg-yellow-50 text-yellow-600',
    red: 'bg-red-50 text-red-600',
    purple: 'bg-purple-50 text-purple-600',
    gray: 'bg-gray-50 text-gray-600'
  };
  
  $: colorClass = colorClasses[color] || colorClasses.blue;
</script>

<div class="bg-white rounded-lg shadow p-6">
  <div class="flex items-center justify-between">
    <div>
      <p class="text-sm text-gray-600 mb-1">{label}</p>
      <p class="text-2xl font-bold text-gray-900">{value}</p>
    </div>
    <div class="{colorClass} p-4 rounded-lg text-3xl">
      {icon}
    </div>
  </div>
</div>
