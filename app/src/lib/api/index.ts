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
 * Compression plugin metadata
 */
export interface CompressionPlugin {
  name: string;
  description: string;
  version: string;
}

/**
 * Compression result for a single file
 */
export interface CompressionFileResult {
  success: boolean;
  original_size?: number;
  compressed_size?: number;
  output_path?: string;
  plugin_name?: string;
  files_processed?: number;
  error?: string;
  source?: string;
}

/**
 * Overall compression result
 */
export interface CompressionResult {
  results: CompressionFileResult[];
  total_original_size: number;
  total_compressed_size: number;
  compression_ratio: number;
}

/**
 * Suitable plugin info
 */
export interface SuitablePlugin {
  name: string;
  description: string;
  estimated_ratio?: number;
}

/**
 * Compressible file info from scan
 */
export interface CompressibleFile {
  path: string;
  original_size: number;
  estimated_compressed_size: number;
  estimated_savings: number;
  plugin_name: string;
  can_handle?: boolean;
  reason?: string;
}

/**
 * Rejection reason from a plugin
 */
export interface RejectionReason {
  plugin_name: string;
  reason: string;
}

/**
 * Rejected file info from scan
 */
export interface RejectedFile {
  path: string;
  size: number;
  extension: string;
  rejection_reasons: RejectionReason[];
}

/**
 * Scan result containing both compressible and rejected files
 */
export interface ScanCompressibleResult {
  compressible: CompressibleFile[];
  rejected: RejectedFile[];
}

/**
 * In-place compression result
 */
export interface InPlaceCompressionResult {
  success: boolean;
  path: string;
  backup_path?: string;
  original_size?: number;
  compressed_size?: number;
  savings?: number;
  plugin_name?: string;
  error?: string;
}

/**
 * Get available compression plugins
 */
export async function getCompressionPlugins(): Promise<CompressionPlugin[]> {
  if (isTauri) {
    return await invoke<CompressionPlugin[]>("get_compression_plugins");
  } else {
    // Mock plugins
    return [
      {
        name: "Image ZIP to WebP ZIP",
        description: "Converts images inside ZIP archives to WebP format",
        version: "1.0.0"
      },
      {
        name: "WebP Converter",
        description: "Converts PNG, JPEG, and other image formats to WebP",
        version: "1.0.0"
      }
    ];
  }
}

/**
 * Scan paths for compressible files
 */
export async function scanCompressibleFiles(
  paths: string[],
  activePlugins: string[],
  filter?: FilterConfig
): Promise<ScanCompressibleResult> {
  if (isTauri) {
    return await invoke<ScanCompressibleResult>("scan_compressible_files", {
      paths,
      activePlugins,
      filter
    });
  } else {
    // Mock scan results
    return {
      compressible: [
        {
          path: "/path/to/image.png",
          original_size: 1024000,
          estimated_compressed_size: 716800,
          estimated_savings: 307200,
          plugin_name: "WebP Converter"
        },
        {
          path: "/path/to/photos.zip",
          original_size: 5120000,
          estimated_compressed_size: 3686400,
          estimated_savings: 1433600,
          plugin_name: "Image ZIP to WebP ZIP"
        }
      ],
      rejected: [
        {
          path: "/path/to/document.pdf",
          size: 2048000,
          extension: "pdf",
          rejection_reasons: [
            {
              plugin_name: "WebP Converter",
              reason: "File extension not supported"
            }
          ]
        }
      ]
    };
  }
}

/**
 * Compress files in place (backup original, replace with compressed)
 */
export async function compressFilesInPlace(
  filePaths: string[],
  pluginOrders: string[]
): Promise<InPlaceCompressionResult[]> {
  if (isTauri) {
    return await invoke<InPlaceCompressionResult[]>("compress_files_in_place", {
      filePaths,
      pluginOrders
    });
  } else {
    // Mock in-place compression
    return filePaths.map(path => ({
      success: true,
      path,
      backup_path: `${path}.backup`,
      original_size: 1024000,
      compressed_size: 716800,
      savings: 307200,
      plugin_name: "WebP Converter"
    }));
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
