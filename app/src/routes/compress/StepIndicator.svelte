<script lang="ts">
  type Step = 'scan' | 'confirm' | 'process';
  
  type Props = {
    currentStep: Step;
    hasScannedFiles: boolean;
    hasResults: boolean;
    isCompressing: boolean;
  };

  let { currentStep, hasScannedFiles, hasResults, isCompressing }: Props = $props();
</script>

<div class="mb-8">
  <div class="flex items-center justify-center">
    <!-- Step 1: Scan -->
    <div class="flex items-center">
      <div class={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${currentStep === 'scan' ? 'bg-blue-600 border-blue-600 text-white' : hasScannedFiles ? 'bg-green-600 border-green-600 text-white' : 'bg-gray-200 border-gray-300 text-gray-600'}`}>
        {#if hasScannedFiles && currentStep !== 'scan'}
          <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path>
          </svg>
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
    <div class={`w-24 h-0.5 mx-4 ${hasScannedFiles ? 'bg-green-600' : 'bg-gray-300'}`}></div>
    
    <!-- Step 2: Confirm -->
    <div class="flex items-center">
      <div class={`flex items-center justify-center w-10 h-10 rounded-full border-2 ${currentStep === 'confirm' ? 'bg-blue-600 border-blue-600 text-white' : currentStep === 'process' ? 'bg-green-600 border-green-600 text-white' : hasScannedFiles ? 'bg-white border-gray-300 text-gray-600' : 'bg-gray-200 border-gray-300 text-gray-400'}`}>
        {#if currentStep === 'process'}
          <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path>
          </svg>
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
        {#if hasResults && !isCompressing}
          <svg class="w-6 h-6" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"></path>
          </svg>
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
