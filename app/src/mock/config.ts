import type { AppConfig } from '$lib/types';
import { loadFromStorage, saveToStorage } from '$lib/utils/storage';

const CONFIG_KEY = 'space-saver:config';

/**
 * Mirrors crates/utils/src/config.rs Config::default(). In web mode this is
 * the real source of truth (persisted to localStorage), so the settings page
 * round-trips exactly like the Tauri backend reading/writing config.toml:
 * save a change, reload the page, and the value sticks.
 */
export function defaultConfig(): AppConfig {
  return {
    database_path: '/home/demo/.local/share/Space-Saver/spacesaver.db',
    cache_dir: '/home/demo/.local/share/Space-Saver/cache',
    log_level: 'info',
    max_concurrent_tasks: 4,
    hash_algorithm: 'Blake3',
    image_similarity_threshold: 0.9,
    default_delete_mode: 'trash',
    default_compress_backup: true,
    scan: {
      follow_links: false,
      max_depth: null,
      min_file_size: 0,
      exclude_patterns: ['*.tmp', '*.cache', '.git/*', 'node_modules/*'],
    },
  };
}

export function getMockConfig(): AppConfig {
  return loadFromStorage<AppConfig>(CONFIG_KEY, defaultConfig());
}

export function setMockConfig(config: AppConfig): AppConfig {
  saveToStorage(CONFIG_KEY, config);
  return config;
}

/** Reset persisted config back to defaults (used to keep tests independent). */
export function resetMockConfig(): void {
  saveToStorage(CONFIG_KEY, defaultConfig());
}
