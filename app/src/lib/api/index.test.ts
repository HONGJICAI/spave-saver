import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  scanDirectory,
  findDuplicates,
  findSimilarImages,
  findEmptyItems,
  findBrokenFiles,
  fixFileExtensions,
  deleteFiles,
  getStorageStats,
  getCompressionPlugins,
  setPluginQuality,
  scanCompressibleFiles,
  compressFilesInPlace,
  getSkipCacheInfo,
  clearSkipCache,
  getConfig,
  setConfig,
  resetConfig,
  detectTools,
} from './index';
import { resetMockConfig, defaultConfig } from '../../mock/config';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('API Layer', () => {
  describe('Web Mode', () => {
    beforeEach(async () => {
      // Ensure we're in web mode
      vi.stubGlobal('__TAURI_INTERNALS__', undefined);
      // The mock skip cache is module state; reset it so tests stay independent
      await clearSkipCache();
      // The mock config persists to localStorage; reset it too
      resetMockConfig();
    });

    it('scanDirectory returns mock data in web mode', async () => {
      const result = await scanDirectory('/test/path');
      
      expect(result).toBeDefined();
      expect(result.files).toBeInstanceOf(Array);
      expect(result.total_size).toBeGreaterThan(0);
    });

    it('findDuplicates returns mock data in web mode', async () => {
      const result = await findDuplicates(['/test/path']);
      
      expect(result).toBeInstanceOf(Array);
      if (result.length > 0) {
        expect(result[0]).toHaveProperty('hash');
        expect(result[0]).toHaveProperty('files');
        expect(result[0]).toHaveProperty('total_size');
      }
    });

    it('findSimilarImages returns mock data in web mode', async () => {
      const result = await findSimilarImages(['/test/path'], 0.9);
      
      expect(result).toBeInstanceOf(Array);
      if (result.length > 0) {
        expect(result[0]).toHaveProperty('similarity_score');
        expect(result[0]).toHaveProperty('files');
      }
    });

    it('findEmptyItems returns empty files and folders in web mode', async () => {
      const result = await findEmptyItems(['/test/path']);

      expect(result.empty_files.length).toBeGreaterThan(0);
      expect(result.empty_folders.length).toBeGreaterThan(0);
      expect(result.empty_files.every(p => typeof p === 'string')).toBe(true);
      expect(result.empty_folders.every(p => typeof p === 'string')).toBe(true);
    });

    it('findEmptyItems mock includes items that demo every delete failure mode', async () => {
      const result = await findEmptyItems(['/test/path']);

      // "locked" items fail deletion with a permission error
      expect(result.empty_files.some(p => p.includes('locked'))).toBe(true);
      expect(result.empty_folders.some(p => p.includes('locked'))).toBe(true);
      // "usb-drive" items fail trash-mode deletion, succeed permanently
      expect(result.empty_folders.some(p => p.includes('usb-drive'))).toBe(true);
    });

    it('findEmptyItems merges results across multiple paths', async () => {
      const single = await findEmptyItems(['/a']);
      const merged = await findEmptyItems(['/a', '/b']);

      expect(merged.empty_files.length).toBe(single.empty_files.length * 2);
      expect(merged.empty_folders.length).toBe(single.empty_folders.length * 2);
    });

    it('findBrokenFiles returns mock data in web mode', async () => {
      const result = await findBrokenFiles(['/test/path']);

      expect(result).toBeInstanceOf(Array);
      expect(result.length).toBeGreaterThan(0);
      expect(result[0]).toHaveProperty('path');
      expect(result[0]).toHaveProperty('size');
      expect(result[0]).toHaveProperty('category');
      expect(result[0]).toHaveProperty('reason');
      expect(result.every(b => b.reason.length > 0)).toBe(true);
    });

    it('findBrokenFiles mock covers both broken categories', async () => {
      const result = await findBrokenFiles(['/test/path']);
      const categories = new Set(result.map(b => b.category));

      expect(categories.has('corrupted')).toBe(true);
      expect(categories.has('extension_mismatch')).toBe(true);
    });

    it('findBrokenFiles mismatches carry a suggested extension, corrupted do not', async () => {
      const result = await findBrokenFiles(['/test/path']);

      const mismatch = result.find(b => b.category === 'extension_mismatch');
      expect(mismatch?.suggested_extension).toBeTruthy();

      const corrupted = result.find(b => b.category === 'corrupted');
      expect(corrupted?.suggested_extension == null).toBe(true);
    });

    it('fixFileExtensions renames misnamed files in web mode', async () => {
      const results = await fixFileExtensions(['/photos/scan.jpg']);

      expect(results).toHaveLength(1);
      expect(results[0].success).toBe(true);
      expect(results[0].new_path).toBe('/photos/scan.pdf');
    });

    it('fixFileExtensions mock reports a permission failure for locked files', async () => {
      const results = await fixFileExtensions(['/locked/report.png']);

      expect(results[0].success).toBe(false);
      expect(results[0].error).toBeTruthy();
    });

    it('findBrokenFiles merges results across multiple paths', async () => {
      const single = await findBrokenFiles(['/a']);
      const merged = await findBrokenFiles(['/a', '/b']);

      expect(merged.length).toBe(single.length * 2);
    });

    it('getStorageStats returns mock data in web mode', async () => {
      const result = await getStorageStats(['/test/path']);
      
      expect(result).toBeDefined();
      expect(result).toHaveProperty('total_size');
      expect(result).toHaveProperty('total_files');
      expect(result).toHaveProperty('images');
    });

    it('deleteFiles reports per-file results in web mode', async () => {
      const results = await deleteFiles(['/file1.txt', '/locked/file2.txt']);

      expect(results).toHaveLength(2);
      expect(results[0]).toEqual({ path: '/file1.txt', success: true });
      expect(results[1].success).toBe(false);
      expect(results[1].error).toBeTruthy();
    });

    it('deleteFiles mock simulates a volume without a trash directory', async () => {
      // Trash mode fails for the USB-drive file...
      const trashed = await deleteFiles(['/usb-drive/video.mp4'], 'trash');
      expect(trashed[0].success).toBe(false);
      expect(trashed[0].error).toContain('no trash directory');

      // ...and retrying with permanent deletion succeeds
      const permanent = await deleteFiles(['/usb-drive/video.mp4'], 'permanent');
      expect(permanent[0].success).toBe(true);
    });

    it('getCompressionPlugins returns all three plugins with quality in web mode', async () => {
      const plugins = await getCompressionPlugins();

      expect(plugins.map(p => p.name)).toEqual([
        'Image ZIP to WebP ZIP',
        'WebP Converter',
        'Animated WebP Converter',
      ]);
      for (const plugin of plugins) {
        expect(plugin.description).toBeTruthy();
        expect(plugin.quality).toBe(85);
      }
    });

    it('setPluginQuality resolves in web mode', async () => {
      await expect(setPluginQuality('WebP Converter', 60)).resolves.toBeUndefined();
    });

    it('scanCompressibleFiles returns compressible and rejected lists in web mode', async () => {
      const result = await scanCompressibleFiles(['/test/path'], ['WebP Converter']);

      expect(result.compressible.length).toBeGreaterThan(0);
      expect(result.compressible[0]).toHaveProperty('path');
      expect(result.compressible[0]).toHaveProperty('estimated_savings');
      expect(result.compressible[0]).toHaveProperty('plugin_name');

      expect(result.rejected.length).toBeGreaterThan(0);
      expect(result.rejected[0].rejection_reasons[0]).toHaveProperty('plugin_name');
      expect(result.rejected[0].rejection_reasons[0]).toHaveProperty('reason');
    });

    it('compressFilesInPlace returns compressed status with backup in web mode', async () => {
      const results = await compressFilesInPlace(['/photos/a.png'], ['WebP Converter']);

      expect(results).toHaveLength(1);
      expect(results[0].status).toBe('compressed');
      expect(results[0].success).toBe(true);
      expect(results[0].backup_path).toBe('/photos/a.png.bak');
      expect(results[0].savings).toBeGreaterThan(0);
    });

    it('compressFilesInPlace mock covers all three result states', async () => {
      const results = await compressFilesInPlace(
        ['/photos/a.png', '/photos/already-tiny.png', '/photos/locked.png'],
        ['WebP Converter']
      );

      expect(results.map(r => r.status)).toEqual(['compressed', 'skipped', 'failed']);

      const skipped = results[1];
      expect(skipped.success).toBe(true);
      expect(skipped.reason).toContain('not smaller');
      expect(skipped.backup_path).toBeUndefined();

      const failed = results[2];
      expect(failed.success).toBe(false);
      expect(failed.error).toBeTruthy();
    });

    it('compressFilesInPlace omits backup_path when backups are disabled', async () => {
      const results = await compressFilesInPlace(['/photos/a.png'], ['WebP Converter'], false);

      expect(results[0].status).toBe('compressed');
      expect(results[0].backup_path).toBeUndefined();
    });

    it('skip cache info and clear resolve in web mode', async () => {
      const info = await getSkipCacheInfo();
      expect(info.entries).toBeGreaterThanOrEqual(0);
      await expect(clearSkipCache()).resolves.toBeGreaterThanOrEqual(0);
    });

    it('scanCompressibleFiles mock includes files that will skip and fail', async () => {
      const result = await scanCompressibleFiles(['/test/path'], ['WebP Converter']);
      const paths = result.compressible.map(f => f.path);

      expect(paths.some(p => p.includes('already-tiny'))).toBe(true);
      expect(paths.some(p => p.includes('locked'))).toBe(true);
    });

    it('setPluginQuality rejects unknown plugins with the backend error string', async () => {
      await expect(setPluginQuality('No Such Plugin', 50)).rejects.toBe(
        'Plugin not found: No Such Plugin'
      );
    });

    it('scanCompressibleFiles rejects unknown active plugins like the backend', async () => {
      await expect(scanCompressibleFiles(['/test/path'], ['No Such Plugin'])).rejects.toBe(
        'Active plugin not found: No Such Plugin'
      );
    });

    it('compressFilesInPlace reports missing files as File not found', async () => {
      const results = await compressFilesInPlace(['/photos/missing.png'], ['WebP Converter']);

      expect(results[0].status).toBe('failed');
      expect(results[0].success).toBe(false);
      expect(results[0].error).toBe('File not found');
    });

    it('findSimilarImages honors the threshold like the backend', async () => {
      const [all, some, none] = await Promise.all([
        findSimilarImages(['/test/path'], 0),
        findSimilarImages(['/test/path'], 0.93),
        findSimilarImages(['/test/path'], 1),
      ]);

      expect(all).toHaveLength(2);
      expect(some.map(g => g.similarity_score)).toEqual([0.95]);
      expect(none).toEqual([]);
    });

    it('paths containing "empty-dir" return empty results across scan APIs', async () => {
      const [scan, duplicates, similar, empty, broken, stats, compressible] = await Promise.all([
        scanDirectory('/data/empty-dir'),
        findDuplicates(['/data/empty-dir']),
        findSimilarImages(['/data/empty-dir'], 0.5),
        findEmptyItems(['/data/empty-dir']),
        findBrokenFiles(['/data/empty-dir']),
        getStorageStats(['/data/empty-dir']),
        scanCompressibleFiles(['/data/empty-dir'], ['WebP Converter']),
      ]);

      expect(scan.file_count).toBe(0);
      expect(scan.files).toEqual([]);
      expect(duplicates).toEqual([]);
      expect(similar).toEqual([]);
      expect(empty).toEqual({ empty_files: [], empty_folders: [] });
      expect(broken).toEqual([]);
      expect(stats.total_files).toBe(0);
      expect(stats.total_size).toBe(0);
      expect(compressible).toEqual({ compressible: [], rejected: [] });
    });

    it('skipped compressions are remembered and excluded from the next scan until cleared', async () => {
      // A file whose compression produced no size reduction is remembered...
      await compressFilesInPlace(['/path/to/already-tiny.png'], ['WebP Converter']);
      expect((await getSkipCacheInfo()).entries).toBe(1);

      // ...the next scan rejects it with the backend's cached-result reason...
      const scan = await scanCompressibleFiles(['/test/path'], ['WebP Converter']);
      expect(scan.compressible.some(f => f.path.includes('already-tiny'))).toBe(false);
      const cached = scan.rejected.find(f => f.path.includes('already-tiny'));
      expect(cached?.rejection_reasons[0].reason).toContain('cached result');

      // ...and clearing the cache makes it compressible again
      expect(await clearSkipCache()).toBe(1);
      expect((await getSkipCacheInfo()).entries).toBe(0);
      const rescan = await scanCompressibleFiles(['/test/path'], ['WebP Converter']);
      expect(rescan.compressible.some(f => f.path.includes('already-tiny'))).toBe(true);
    });

    it('getConfig returns the default configuration in web mode', async () => {
      const config = await getConfig();

      expect(config).toEqual(defaultConfig());
      expect(config.image_similarity_threshold).toBe(0.9);
      expect(config.default_delete_mode).toBe('trash');
      expect(config.default_compress_backup).toBe(true);
      expect(config.scan.exclude_patterns.length).toBeGreaterThan(0);
    });

    it('setConfig persists changes so the next getConfig returns them', async () => {
      const config = await getConfig();
      config.image_similarity_threshold = 0.7;
      config.default_delete_mode = 'permanent';
      config.default_compress_backup = false;

      const saved = await setConfig(config);
      expect(saved.image_similarity_threshold).toBe(0.7);

      const reloaded = await getConfig();
      expect(reloaded.image_similarity_threshold).toBe(0.7);
      expect(reloaded.default_delete_mode).toBe('permanent');
      expect(reloaded.default_compress_backup).toBe(false);
    });

    it('setConfig rejects out-of-range thresholds with the backend error string', async () => {
      const config = await getConfig();
      config.image_similarity_threshold = 5;

      await expect(setConfig(config)).rejects.toContain('between 0.0 and 1.0');
    });

    it('setConfig rejects an invalid delete mode like the backend', async () => {
      const config = await getConfig();
      // @ts-expect-error deliberately invalid to exercise the validation path
      config.default_delete_mode = 'shred';

      await expect(setConfig(config)).rejects.toContain("'trash' or 'permanent'");
    });

    it('setConfig rejects a max_concurrent_tasks below 1', async () => {
      const config = await getConfig();
      config.max_concurrent_tasks = 0;

      await expect(setConfig(config)).rejects.toContain('at least 1');
    });

    it('resetConfig restores defaults and persists them in web mode', async () => {
      // Change something and confirm it stuck...
      const config = await getConfig();
      config.image_similarity_threshold = 0.6;
      config.default_delete_mode = 'permanent';
      await setConfig(config);
      expect((await getConfig()).image_similarity_threshold).toBe(0.6);

      // ...then reset returns and persists the defaults
      const defaults = await resetConfig();
      expect(defaults).toEqual(defaultConfig());

      const reloaded = await getConfig();
      expect(reloaded.image_similarity_threshold).toBe(0.9);
      expect(reloaded.default_delete_mode).toBe('trash');
    });

    it('detectTools reports both available and missing tools in web mode', async () => {
      const tools = await detectTools();

      const names = tools.map(t => t.name);
      expect(names).toContain('ffmpeg');
      expect(names).toContain('cwebp');

      // The mock deliberately covers both states the backend can report
      expect(tools.some(t => t.available)).toBe(true);
      expect(tools.some(t => !t.available)).toBe(true);

      const ffmpeg = tools.find(t => t.name === 'ffmpeg');
      expect(ffmpeg?.available).toBe(true);
      expect(ffmpeg?.version).toBeTruthy();

      const cwebp = tools.find(t => t.name === 'cwebp');
      expect(cwebp?.available).toBe(false);
      expect(cwebp?.path == null).toBe(true);
    });
  });

  describe('Tauri Mode', () => {
    beforeEach(() => {
      // Mock Tauri environment
      vi.stubGlobal('__TAURI_INTERNALS__', {});
    });

    it('detects Tauri mode correctly', () => {
      expect('__TAURI_INTERNALS__' in window).toBe(true);
    });
  });
});
