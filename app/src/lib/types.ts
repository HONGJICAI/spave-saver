/**
 * Shared type definitions for the Space Saver application
 */

/**
 * File information
 */
export interface FileInfo {
  path: string;
  size: number;
  modified: number;
  file_type: string;
  hash?: string;
}

/**
 * Scan result
 */
export interface ScanResult {
  path: string;
  file_count: number;
  total_size: number;
  files: FileInfo[];
}

/**
 * Duplicate file group
 */
export interface DuplicateGroup {
  hash: string;
  files: FileInfo[];
  count: number;
  total_size: number;
  wasted_space: number;
}

/**
 * Similar image group
 */
export interface SimilarGroup {
  files: FileInfo[];
  similarity_score: number;
}

/**
 * Empty files and folders found in a scan. Files are 0 bytes; folders
 * contain no files anywhere beneath them and are reported topmost-only.
 */
export interface EmptyScanResult {
  empty_files: string[];
  empty_folders: string[];
}

/**
 * Storage statistics
 */
export interface StorageStats {
  total_files: number;
  total_size: number;
  images: number;
  videos: number;
  documents: number;
  archives: number;
  others: number;
  empty_files: number;
}
