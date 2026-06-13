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
 * Why a file is considered broken:
 * - "corrupted": content cannot be parsed as its declared format
 *   (truncated/garbage), e.g. a JPEG whose data is cut off
 * - "extension_mismatch": content does not match the extension, e.g. a
 *   .jpg whose bytes are actually a PDF (may be valid, just misnamed)
 */
export type BrokenCategory = "corrupted" | "extension_mismatch";

/**
 * A file found to be invalid or corrupted
 */
export interface BrokenFile {
  path: string;
  size: number;
  category: BrokenCategory;
  /** Human-readable explanation, worded close to the backend's error */
  reason: string;
  /**
   * For an extension mismatch, the extension matching the real content
   * (e.g. "pdf"), so the file can be renamed instead of deleted. Null for
   * corruption.
   */
  suggested_extension?: string | null;
}

/**
 * Per-file outcome of fixing a file's extension (renaming to match content)
 */
export interface FixExtensionResult {
  /** The original path that was asked to be fixed */
  path: string;
  success: boolean;
  /** The new path after renaming, when successful */
  new_path?: string | null;
  error?: string | null;
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
