/**
 * Web Storage utilities for persisting app state.
 *
 * Two scopes are exposed:
 * - `*FromStorage` / `storageKeys`  → localStorage: long-lived user preferences
 *   (scan paths, filters, compress settings) that should survive app restarts.
 * - `*FromSession` / `sessionKeys`  → sessionStorage: per-page scan results kept
 *   only for the lifetime of the window/tab. We deliberately use session (not
 *   local) scope here: scan results are time-sensitive — once the window closes
 *   the on-disk picture may have changed, so persisting them across restarts
 *   would risk showing stale data. Within a session they let the user navigate
 *   away from a half-finished workflow and return to it intact.
 */

const STORAGE_KEYS = {
  SCAN_PATHS: 'space-saver:scanPaths',
  FILTER_CONFIG: 'space-saver:filterConfig',
  COMPRESS_SETTINGS: 'space-saver:compressSettings',
} as const;

const SESSION_KEYS = {
  DUPLICATES_RESULT: 'space-saver:session:duplicates',
  SIMILAR_RESULT: 'space-saver:session:similar',
  BROKEN_RESULT: 'space-saver:session:broken',
  EMPTY_RESULT: 'space-saver:session:empty',
  COMPRESS_RESULT: 'space-saver:session:compress',
} as const;

function load<T>(storage: Storage, label: string, key: string, defaultValue: T): T {
  try {
    const item = storage.getItem(key);
    return item ? JSON.parse(item) : defaultValue;
  } catch (error) {
    console.error(`Error loading from ${label} (${key}):`, error);
    return defaultValue;
  }
}

function save<T>(storage: Storage, label: string, key: string, value: T): void {
  try {
    storage.setItem(key, JSON.stringify(value));
  } catch (error) {
    console.error(`Error saving to ${label} (${key}):`, error);
  }
}

function remove(storage: Storage, label: string, key: string): void {
  try {
    storage.removeItem(key);
  } catch (error) {
    console.error(`Error removing from ${label} (${key}):`, error);
  }
}

export function loadFromStorage<T>(key: string, defaultValue: T): T {
  if (typeof window === 'undefined') {
    return defaultValue;
  }
  return load(window.localStorage, 'localStorage', key, defaultValue);
}

export function saveToStorage<T>(key: string, value: T): void {
  if (typeof window === 'undefined') {
    return;
  }
  save(window.localStorage, 'localStorage', key, value);
}

export function removeFromStorage(key: string): void {
  if (typeof window === 'undefined') {
    return;
  }
  remove(window.localStorage, 'localStorage', key);
}

export function loadFromSession<T>(key: string, defaultValue: T): T {
  if (typeof window === 'undefined') {
    return defaultValue;
  }
  return load(window.sessionStorage, 'sessionStorage', key, defaultValue);
}

export function saveToSession<T>(key: string, value: T): void {
  if (typeof window === 'undefined') {
    return;
  }
  save(window.sessionStorage, 'sessionStorage', key, value);
}

export function removeFromSession(key: string): void {
  if (typeof window === 'undefined') {
    return;
  }
  remove(window.sessionStorage, 'sessionStorage', key);
}

export const storageKeys = STORAGE_KEYS;
export const sessionKeys = SESSION_KEYS;
