import { writable, type Writable } from 'svelte/store';

export interface FilterConfig {
  minSize?: number;      // in bytes
  maxSize?: number;      // in bytes
  extensions?: string[]; // array of extensions
  filePattern?: string;  // pattern to match in filename
}

export interface AppState {
  scanPaths: string[]; // Multiple paths for scanning
  isScanning: boolean;
  error: string | null;
  mode: 'web' | 'tauri';
  filterConfig: FilterConfig;
}

function createAppStore() {
  const mode = '__TAURI_INTERNALS__' in window ? 'tauri' : 'web';
  const { subscribe, set, update } = writable<AppState>({
    scanPaths: [],
    isScanning: false,
    error: null,
    mode,
    filterConfig: {}
  });

  return {
    subscribe,
    set,
    update,
    setScanPaths: (paths: string[]) => update(state => ({ ...state, scanPaths: paths })),
    addScanPath: (path: string) => update(state => ({ 
      ...state, 
      scanPaths: state.scanPaths.includes(path) ? state.scanPaths : [...state.scanPaths, path]
    })),
    removeScanPath: (path: string) => update(state => ({ 
      ...state, 
      scanPaths: state.scanPaths.filter(p => p !== path)
    })),
    clearScanPaths: () => update(state => ({ ...state, scanPaths: [] })),
    setScanning: (isScanning: boolean) => update(state => ({ ...state, isScanning })),
    setError: (error: string | null) => update(state => ({ ...state, error })),
    setFilterConfig: (filterConfig: FilterConfig) => update(state => ({ ...state, filterConfig })),
    clearFilters: () => update(state => ({ ...state, filterConfig: {} })),
    reset: () => set({
      scanPaths: [],
      isScanning: false,
      error: null,
      mode,
      filterConfig: {}
    })
  };
}

export const appState = createAppStore();
