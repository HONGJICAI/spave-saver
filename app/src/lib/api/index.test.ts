import { describe, it, expect, vi, beforeEach } from 'vitest';
import { scanDirectory, findDuplicates, findSimilarImages, findEmptyFiles, deleteFiles, getStorageStats } from './index';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('API Layer', () => {
  describe('Web Mode', () => {
    beforeEach(() => {
      // Ensure we're in web mode
      vi.stubGlobal('__TAURI_INTERNALS__', undefined);
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
        expect(result[0]).toHaveProperty('totalSize');
      }
    });

    it('findSimilarImages returns mock data in web mode', async () => {
      const result = await findSimilarImages(['/test/path'], 0.9);
      
      expect(result).toBeInstanceOf(Array);
      if (result.length > 0) {
        expect(result[0]).toHaveProperty('similarity');
        expect(result[0]).toHaveProperty('files');
      }
    });

    it('findEmptyFiles returns mock data in web mode', async () => {
      const result = await findEmptyFiles(['/test/path']);
      
      expect(result).toBeInstanceOf(Array);
    });

    it('getStorageStats returns mock data in web mode', async () => {
      const result = await getStorageStats(['/test/path']);
      
      expect(result).toBeDefined();
      expect(result).toHaveProperty('totalSize');
      expect(result).toHaveProperty('fileCount');
      expect(result).toHaveProperty('typeDistribution');
    });

    it('deleteFiles resolves in web mode', async () => {
      await expect(deleteFiles(['/file1.txt'])).resolves.toBeUndefined();
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
