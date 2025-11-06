/**
 * Unified API Layer
 * Automatically detects Tauri or Web mode and routes to appropriate backend
 */

import { invoke } from "@tauri-apps/api/core";
import type { ScanResult, DuplicateGroup, SimilarGroup, StorageStats, FileInfo } from "../types";
import type { FilterConfig } from "../stores/app";
import { mockScanResult } from "../../mock/scan";
import { mockFindDuplicates } from "../../mock/duplicates";
import { mockFindSimilar } from "../../mock/similar";
import { mockEmptyFiles } from "../../mock/empty";
import { mockStorageStats } from "../../mock/stats";

// Check if running in Tauri environment
const isTauri = "__TAURI_INTERNALS__" in window;

export { type ScanResult, type DuplicateGroup, type SimilarGroup, type StorageStats, type FileInfo, type FilterConfig };

/**
 * Scan multiple directories for files
 */
export async function scanDirectories(paths: string[], filter?: FilterConfig): Promise<ScanResult[]> {
  if (isTauri) {
    return await invoke<ScanResult[]>("scan", { paths, filter: filter || null });
  } else {
    return await Promise.all(paths.map(path => mockScanResult(path)));
  }
}

/**
 * Scan a single directory for files (convenience method)
 */
export async function scanDirectory(path: string, filter?: FilterConfig): Promise<ScanResult> {
  const results = await scanDirectories([path], filter);
  return results[0];
}

/**
 * Find duplicate files across multiple directories
 */
export async function findDuplicates(paths: string[], filter?: FilterConfig): Promise<DuplicateGroup[]> {
  if (isTauri) {
    return await invoke<DuplicateGroup[]>("duplicate_file_check", { paths, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockFindDuplicates(path)));
    return results.flat();
  }
}

/**
 * Find similar images across multiple directories
 */
export async function findSimilarImages(
  paths: string[],
  threshold: number = 0.9,
  filter?: FilterConfig
): Promise<SimilarGroup[]> {
  if (isTauri) {
    return await invoke<SimilarGroup[]>("similar_file_check", { paths, threshold, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockFindSimilar(path, threshold)));
    return results.flat();
  }
}

/**
 * Find empty files across multiple directories
 */
export async function findEmptyFiles(paths: string[], filter?: FilterConfig): Promise<string[]> {
  if (isTauri) {
    return await invoke<string[]>("empty_folder_check", { paths, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockEmptyFiles(path)));
    return results.flat();
  }
}

/**
 * Delete files
 */
export async function deleteFiles(paths: string[]): Promise<number> {
  if (isTauri) {
    return await invoke<number>("delete_files", { paths });
  } else {
    // Mock deletion
    return new Promise((resolve) => {
      setTimeout(() => resolve(paths.length), 500);
    });
  }
}

/**
 * Get storage statistics across multiple directories
 */
export async function getStorageStats(paths: string[], filter?: FilterConfig): Promise<StorageStats> {
  if (isTauri) {
    return await invoke<StorageStats>("get_storage_stats", { paths, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockStorageStats(path)));
    // Aggregate stats from all paths
    return results.reduce((acc, stats) => ({
      total_files: acc.total_files + stats.total_files,
      total_size: acc.total_size + stats.total_size,
      images: acc.images + stats.images,
      videos: acc.videos + stats.videos,
      documents: acc.documents + stats.documents,
      archives: acc.archives + stats.archives,
      others: acc.others + stats.others,
      empty_files: acc.empty_files + stats.empty_files,
    }), {
      total_files: 0,
      total_size: 0,
      images: 0,
      videos: 0,
      documents: 0,
      archives: 0,
      others: 0,
      empty_files: 0,
    });
  }
}

/**
 * Compress files
 */
export async function compressFiles(
  paths: string[],
  outputPath: string
): Promise<number> {
  if (isTauri) {
    return await invoke<number>("compress_files", { paths, outputPath });
  } else {
    // Mock compression
    return new Promise((resolve) => {
      setTimeout(() => resolve(1048576), 800); // 1MB compressed
    });
  }
}

/**
 * Check if running in Tauri mode
 */
export function isTauriMode(): boolean {
  return isTauri;
}

/**
 * Get mode name
 */
export function getModeName(): string {
  return isTauri ? "Tauri Desktop" : "Web Mode (Mock Data)";
}
