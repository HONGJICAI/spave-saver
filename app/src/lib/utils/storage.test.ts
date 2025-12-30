import { describe, it, expect, beforeEach, vi } from 'vitest';
import { loadFromStorage, saveToStorage, removeFromStorage, storageKeys } from './storage';

describe('storage utilities', () => {
  beforeEach(() => {
    // Clear localStorage before each test
    localStorage.clear();
    // Clear any mocks
    vi.clearAllMocks();
  });

  describe('saveToStorage', () => {
    it('should save simple values to localStorage', () => {
      saveToStorage('test-key', 'test-value');
      expect(localStorage.getItem('test-key')).toBe(JSON.stringify('test-value'));
    });

    it('should save objects to localStorage', () => {
      const testObject = { foo: 'bar', baz: 123 };
      saveToStorage('test-object', testObject);
      expect(localStorage.getItem('test-object')).toBe(JSON.stringify(testObject));
    });

    it('should save arrays to localStorage', () => {
      const testArray = [1, 2, 3, 'test'];
      saveToStorage('test-array', testArray);
      expect(localStorage.getItem('test-array')).toBe(JSON.stringify(testArray));
    });

    it('should handle errors gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const mockStorage = vi.spyOn(window.localStorage, 'setItem').mockImplementation(() => {
        throw new Error('Storage error');
      });

      saveToStorage('test-key', 'value');
      
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining('Error saving to localStorage'),
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
      mockStorage.mockRestore();
    });
  });

  describe('loadFromStorage', () => {
    it('should load saved values from localStorage', () => {
      localStorage.setItem('test-key', JSON.stringify('test-value'));
      expect(loadFromStorage('test-key', 'default')).toBe('test-value');
    });

    it('should load objects from localStorage', () => {
      const testObject = { foo: 'bar', baz: 123 };
      localStorage.setItem('test-object', JSON.stringify(testObject));
      expect(loadFromStorage('test-object', {})).toEqual(testObject);
    });

    it('should return default value when key does not exist', () => {
      expect(loadFromStorage('non-existent', 'default')).toBe('default');
    });

    it('should return default value on parse error', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      localStorage.setItem('invalid-json', 'not valid json');
      
      expect(loadFromStorage('invalid-json', 'default')).toBe('default');
      expect(consoleErrorSpy).toHaveBeenCalled();
      
      consoleErrorSpy.mockRestore();
    });

    it('should handle errors gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const mockStorage = vi.spyOn(window.localStorage, 'getItem').mockImplementation(() => {
        throw new Error('Storage error');
      });

      const result = loadFromStorage('test-key', 'default-value');
      
      expect(result).toBe('default-value');
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining('Error loading from localStorage'),
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
      mockStorage.mockRestore();
    });
  });

  describe('removeFromStorage', () => {
    it('should remove items from localStorage', () => {
      localStorage.setItem('test-key', 'test-value');
      removeFromStorage('test-key');
      expect(localStorage.getItem('test-key')).toBeNull();
    });

    it('should handle errors gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const mockStorage = vi.spyOn(window.localStorage, 'removeItem').mockImplementation(() => {
        throw new Error('Storage error');
      });

      removeFromStorage('test-key');
      
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining('Error removing from localStorage'),
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
      mockStorage.mockRestore();
    });
  });

  describe('storageKeys', () => {
    it('should have the correct key for scan paths', () => {
      expect(storageKeys.SCAN_PATHS).toBe('space-saver:scanPaths');
    });

    it('should have the correct key for filter config', () => {
      expect(storageKeys.FILTER_CONFIG).toBe('space-saver:filterConfig');
    });
  });
});
