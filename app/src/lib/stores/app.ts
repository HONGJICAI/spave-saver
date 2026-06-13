import { writable, type Writable } from 'svelte/store';
import { loadFromStorage, saveToStorage, storageKeys } from '$lib/utils/storage';
import { validatePath, type PathValidationResult } from '$lib/utils/path';

export interface FilterConfig {
  minSize?: number;       // in bytes
  maxSize?: number;       // in bytes
  extensions?: string[];  // array of extensions
  filePattern?: string;   // pattern to match in filename
  excludePaths?: string[]; // paths to exclude (files at or beneath are dropped)
}

export interface AppState {
  scanPaths: string[]; // Multiple paths for scanning
  // True while any long-running operation (scan, delete, fix, compress) is in
  // flight. The layout uses this to block navigation so a stray sidebar click
  // can't unmount a page mid-operation and lose its progress.
  busy: boolean;
  error: string | null;
  mode: 'web' | 'tauri';
  filterConfig: FilterConfig;
}

function createAppStore() {
  const mode = '__TAURI_INTERNALS__' in window ? 'tauri' : 'web';
  
  // Load persisted state from localStorage
  const persistedPaths = loadFromStorage<string[]>(storageKeys.SCAN_PATHS, []);
  const persistedFilters = loadFromStorage<FilterConfig>(storageKeys.FILTER_CONFIG, {});
  
  const { subscribe, set, update } = writable<AppState>({
    scanPaths: persistedPaths,
    busy: false,
    error: null,
    mode,
    filterConfig: persistedFilters
  });

  return {
    subscribe,
    set,
    update,
    setScanPaths: (paths: string[]) => update(state => {
      const newState = { ...state, scanPaths: paths };
      saveToStorage(storageKeys.SCAN_PATHS, paths);
      return newState;
    }),
    addScanPath: (path: string) => update(state => {
      for (const pathItem of state.scanPaths) {
        if (pathItem === path) {
          return state; // Duplicate
        }
      }
      const newPaths = [...state.scanPaths, path];
      saveToStorage(storageKeys.SCAN_PATHS, newPaths);
      return { ...state, scanPaths: newPaths };
    }),
    removeScanPath: (path: string) => update(state => {
      const newPaths = state.scanPaths.filter(p => p !== path);
      saveToStorage(storageKeys.SCAN_PATHS, newPaths);
      return { ...state, scanPaths: newPaths };
    }),
    clearScanPaths: () => update(state => {
      saveToStorage(storageKeys.SCAN_PATHS, []);
      return { ...state, scanPaths: [] };
    }),
    setBusy: (busy: boolean) => update(state => ({ ...state, busy })),
    setError: (error: string | null) => update(state => ({ ...state, error })),
    setFilterConfig: (filterConfig: FilterConfig) => update(state => {
      saveToStorage(storageKeys.FILTER_CONFIG, filterConfig);
      return { ...state, filterConfig };
    }),
    clearFilters: () => update(state => {
      const emptyFilter = {};
      saveToStorage(storageKeys.FILTER_CONFIG, emptyFilter);
      return { ...state, filterConfig: emptyFilter };
    }),
    validatePath: (path: string, existingPaths?: string[]): PathValidationResult => {
      let currentPaths: string[] = [];
      subscribe(state => { currentPaths = state.scanPaths; })();
      return validatePath(path, existingPaths || currentPaths);
    },
    reset: () => {
      saveToStorage(storageKeys.SCAN_PATHS, []);
      saveToStorage(storageKeys.FILTER_CONFIG, {});
      set({
        scanPaths: [],
        busy: false,
        error: null,
        mode,
        filterConfig: {}
      });
    }
  };
}

export const appState = createAppStore();
