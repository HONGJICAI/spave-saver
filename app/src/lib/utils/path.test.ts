import { describe, it, expect } from 'vitest';
import { 
  normalizePath, 
  isSubpath, 
  isParentPath, 
  findParentPaths, 
  findChildPaths,
  validatePath 
} from './path';

describe('normalizePath', () => {
  it('should convert backslashes to forward slashes', () => {
    expect(normalizePath('C:\\Users\\test')).toBe('c:/users/test');
  });

  it('should remove trailing slash', () => {
    expect(normalizePath('/home/user/')).toBe('/home/user');
    expect(normalizePath('C:\\Users\\test\\')).toBe('c:/users/test');
  });

  it('should keep root slash', () => {
    expect(normalizePath('/')).toBe('/');
    expect(normalizePath('C:\\')).toBe('c:/');
  });

  it('should convert to lowercase', () => {
    expect(normalizePath('/Home/User/Documents')).toBe('/home/user/documents');
  });
});

describe('isSubpath', () => {
  it('should return true for subpaths', () => {
    expect(isSubpath('/home/user/documents', '/home/user')).toBe(true);
    expect(isSubpath('C:\\Users\\John\\Photos', 'C:\\Users\\John')).toBe(true);
  });

  it('should return false for non-subpaths', () => {
    expect(isSubpath('/home/user', '/home/user2')).toBe(false);
    expect(isSubpath('/home/other', '/home/user')).toBe(false);
  });

  it('should return false for same paths', () => {
    expect(isSubpath('/home/user', '/home/user')).toBe(false);
    expect(isSubpath('C:\\Users\\John', 'C:\\Users\\John')).toBe(false);
  });

  it('should return false for parent paths', () => {
    expect(isSubpath('/home', '/home/user')).toBe(false);
  });

  it('should handle paths with different separators', () => {
    expect(isSubpath('C:\\Users\\John\\Documents', 'C:/Users/John')).toBe(true);
  });
});

describe('isParentPath', () => {
  it('should return true for parent paths', () => {
    expect(isParentPath('/home/user', '/home/user/documents')).toBe(true);
    expect(isParentPath('C:\\Users\\John', 'C:\\Users\\John\\Photos')).toBe(true);
  });

  it('should return false for non-parent paths', () => {
    expect(isParentPath('/home/user/documents', '/home/user')).toBe(false);
    expect(isParentPath('/home/user2', '/home/user')).toBe(false);
  });
});

describe('findParentPaths', () => {
  it('should find all parent paths', () => {
    const existing = ['/home', '/home/user', '/var', '/opt'];
    const result = findParentPaths('/home/user/documents', existing);
    expect(result).toEqual(['/home', '/home/user']);
  });

  it('should return empty array when no parent paths exist', () => {
    const existing = ['/var', '/opt', '/etc'];
    const result = findParentPaths('/home/user/documents', existing);
    expect(result).toEqual([]);
  });
});

describe('findChildPaths', () => {
  it('should find all child paths', () => {
    const existing = ['/home/user/documents', '/home/user/photos', '/var', '/home/other'];
    const result = findChildPaths('/home/user', existing);
    expect(result).toEqual(['/home/user/documents', '/home/user/photos']);
  });

  it('should return empty array when no child paths exist', () => {
    const existing = ['/var', '/opt', '/etc'];
    const result = findChildPaths('/home/user', existing);
    expect(result).toEqual([]);
  });
});

describe('validatePath', () => {
  it('should validate a new unique path', () => {
    const existing = ['/home/user', '/var'];
    const result = validatePath('/opt/data', existing);
    expect(result.isValid).toBe(true);
    expect(result.isDuplicate).toBe(false);
    expect(result.isSubpathOf).toEqual([]);
    expect(result.hasSubpaths).toEqual([]);
    expect(result.warnings).toEqual([]);
  });

  it('should detect duplicate paths', () => {
    const existing = ['/home/user', '/var'];
    const result = validatePath('/home/user', existing);
    expect(result.isValid).toBe(false);
    expect(result.isDuplicate).toBe(true);
    expect(result.warnings).toContain('This path is already in the list');
  });

  it('should detect subpaths of existing paths', () => {
    const existing = ['/home/user', '/var'];
    const result = validatePath('/home/user/documents', existing);
    expect(result.isValid).toBe(false);
    expect(result.isSubpathOf).toEqual(['/home/user']);
    expect(result.warnings.some(w => w.includes('already covered by'))).toBe(true);
  });

  it('should detect paths that would make existing paths redundant', () => {
    const existing = ['/home/user/documents', '/home/user/photos'];
    const result = validatePath('/home/user', existing);
    expect(result.isValid).toBe(true);
    expect(result.hasSubpaths).toEqual(['/home/user/documents', '/home/user/photos']);
    expect(result.warnings.some(w => w.includes('would make redundant'))).toBe(true);
  });

  it('should handle case-insensitive comparison on Windows paths', () => {
    const existing = ['C:\\Users\\John'];
    const result = validatePath('c:\\users\\john', existing);
    expect(result.isDuplicate).toBe(true);
  });

  it('should handle mixed path separators', () => {
    const existing = ['C:\\Users\\John'];
    const result = validatePath('C:/Users/John/Documents', existing);
    expect(result.isValid).toBe(false);
    expect(result.isSubpathOf).toContain('C:\\Users\\John');
  });
});
