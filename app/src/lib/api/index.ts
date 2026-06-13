/**
 * Unified API Layer
 * Automatically detects Tauri or Web mode and routes to appropriate backend
 */

import { invoke } from "@tauri-apps/api/core";
import type { ScanResult, DuplicateGroup, SimilarGroup, StorageStats, FileInfo, EmptyScanResult, BrokenFile, BrokenCategory, FixExtensionResult } from "../types";
import type { FilterConfig } from "../stores/app";
import { mockScanResult } from "../../mock/scan";
import { mockFindDuplicates } from "../../mock/duplicates";
import { mockFindSimilar } from "../../mock/similar";
import { mockEmptyItems } from "../../mock/empty";
import { mockFindBroken, mockFixExtensions } from "../../mock/broken";
import { mockStorageStats } from "../../mock/stats";
import { mockPlugins, isKnownPlugin } from "../../mock/plugins";
import { mockSkipCache } from "../../mock/skipCache";

// Check if running in Tauri environment
const isTauri = "__TAURI_INTERNALS__" in window;

export { type ScanResult, type DuplicateGroup, type SimilarGroup, type StorageStats, type FileInfo, type FilterConfig, type EmptyScanResult, type BrokenFile, type BrokenCategory, type FixExtensionResult };

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
 * Find empty files (0 bytes) and empty folders (no files anywhere beneath
 * them, topmost-only) across multiple directories. The filter applies to
 * files only.
 */
export async function findEmptyItems(paths: string[], filter?: FilterConfig): Promise<EmptyScanResult> {
  if (isTauri) {
    return await invoke<EmptyScanResult>("empty_folder_check", { paths, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockEmptyItems(path)));
    return {
      empty_files: results.flatMap(r => r.empty_files),
      empty_folders: results.flatMap(r => r.empty_folders),
    };
  }
}

/**
 * Find broken (invalid or corrupted) files across multiple directories.
 * Reports only files that are provably unusable — corrupted/truncated content
 * or content that does not match its extension. Empty files are excluded.
 */
export async function findBrokenFiles(paths: string[], filter?: FilterConfig): Promise<BrokenFile[]> {
  if (isTauri) {
    return await invoke<BrokenFile[]>("broken_file_check", { paths, filter: filter || null });
  } else {
    const results = await Promise.all(paths.map(path => mockFindBroken(path)));
    return results.flat();
  }
}

/**
 * Fix misnamed files by renaming them to the extension matching their real
 * content (e.g. a PDF named .jpg becomes .pdf). The safe action for
 * extension-mismatch results — the file is valid, just named wrong.
 */
export async function fixFileExtensions(paths: string[]): Promise<FixExtensionResult[]> {
  if (isTauri) {
    return await invoke<FixExtensionResult[]>("fix_file_extensions", { paths });
  } else {
    return await mockFixExtensions(paths);
  }
}

/**
 * How files are removed: "trash" (system recycle bin, recoverable) or
 * "permanent" (unrecoverable). Defaults to trash.
 */
export type DeleteMode = "trash" | "permanent";

/**
 * Per-file outcome of a delete operation
 */
export interface DeleteResult {
  path: string;
  success: boolean;
  error?: string | null;
}

/**
 * Delete files, reporting a per-file outcome
 */
export async function deleteFiles(
  paths: string[],
  mode: DeleteMode = "trash"
): Promise<DeleteResult[]> {
  if (isTauri) {
    return await invoke<DeleteResult[]>("delete_files", { paths, mode });
  } else {
    // Mock deletion, demoing the failure modes:
    // - "locked" files always fail (permission denied)
    // - "usb-drive" files fail in trash mode only (no trash directory on
    //   that volume), succeeding when retried as permanent deletion
    return new Promise((resolve) => {
      setTimeout(
        () =>
          resolve(
            paths.map((path) => {
              if (path.includes("locked")) {
                return { path, success: false, error: "Permission denied (os error 13)" };
              }
              if (path.includes("usb-drive") && mode === "trash") {
                return {
                  path,
                  success: false,
                  error:
                    "Cannot move to trash: the volume has no trash directory. Retry with permanent deletion.",
                };
              }
              return { path, success: true };
            })
          ),
        300
      );
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
  /** Quality setting (0-100), or null if the plugin has no quality knob */
  quality?: number | null;
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
 * Status of an in-place compression:
 * - compressed: original renamed to backup, smaller file written
 * - skipped: output was not smaller, original kept untouched
 * - failed: an error occurred, original kept untouched
 */
export type CompressionStatus = "compressed" | "skipped" | "failed";

/**
 * In-place compression result
 */
export interface InPlaceCompressionResult {
  status: CompressionStatus;
  success: boolean;
  path: string;
  backup_path?: string;
  original_size?: number;
  compressed_size?: number;
  savings?: number;
  plugin_name?: string;
  reason?: string;
  error?: string;
}

/**
 * Get available compression plugins
 */
export async function getCompressionPlugins(): Promise<CompressionPlugin[]> {
  if (isTauri) {
    return await invoke<CompressionPlugin[]>("get_compression_plugins");
  } else {
    // Copies, so UI-side mutation cannot leak into the shared mock list
    return mockPlugins.map(p => ({ ...p }));
  }
}

/**
 * Set the quality (0-100) of a compression plugin
 */
export async function setPluginQuality(pluginName: string, quality: number): Promise<void> {
  if (isTauri) {
    await invoke("set_plugin_quality", { pluginName, quality });
  } else {
    // Mirrors the backend: unknown plugin names fail (quality itself is
    // clamped backend-side, never an error). Like a real invoke() failure,
    // the rejection value is the backend's plain error string.
    if (!isKnownPlugin(pluginName)) {
      return Promise.reject(`Plugin not found: ${pluginName}`);
    }
    // Success is a no-op: mock plugins keep their displayed value in the UI
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
    // Mirrors the backend: unknown active plugin names abort the scan with
    // the same plain-string error a real invoke() would reject with
    for (const name of activePlugins) {
      if (!isKnownPlugin(name)) {
        return Promise.reject(`Active plugin not found: ${name}`);
      }
    }

    // Paths containing "empty-dir" contribute nothing, like the backend
    // scanning an empty or nonexistent directory
    if (!paths.some(p => !p.includes("empty-dir"))) {
      return { compressible: [], rejected: [] };
    }

    // Mock scan results. "already-tiny" and "locked" are picked up by the
    // compressFilesInPlace mock to demo the skipped/failed states in web mode.
    const compressible: CompressibleFile[] = [
      {
        path: "/path/to/image.png",
        original_size: 1024000,
        estimated_compressed_size: 716800,
        estimated_savings: 307200,
        plugin_name: "WebP Converter"
      },
      {
        path: "/path/to/wallpaper.png",
        original_size: 3145728,
        estimated_compressed_size: 2202010,
        estimated_savings: 943718,
        plugin_name: "WebP Converter"
      },
      {
        path: "/path/to/photos.zip",
        original_size: 5120000,
        estimated_compressed_size: 3686400,
        estimated_savings: 1433600,
        plugin_name: "Image ZIP to WebP ZIP"
      },
      {
        path: "/path/to/already-tiny.png",
        original_size: 98304,
        estimated_compressed_size: 72744,
        estimated_savings: 25560,
        plugin_name: "WebP Converter"
      },
      {
        path: "/path/to/locked.png",
        original_size: 512000,
        estimated_compressed_size: 358400,
        estimated_savings: 153600,
        plugin_name: "WebP Converter"
      }
    ];
    const rejected: RejectedFile[] = [
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
    ];

    // Files remembered as "no size reduction" (recorded by the
    // compressFilesInPlace mock when a file skips) are excluded from
    // compressible and surfaced as rejections, like the backend skip cache
    const remaining: CompressibleFile[] = [];
    for (const file of compressible) {
      if (mockSkipCache.has(file.path)) {
        rejected.push({
          path: file.path,
          size: file.original_size,
          extension: file.path.split(".").pop() ?? "",
          rejection_reasons: [
            {
              plugin_name: file.plugin_name,
              reason:
                "Previously produced no size reduction at quality 85 (cached result; file unchanged)"
            }
          ]
        });
      } else {
        remaining.push(file);
      }
    }
    return { compressible: remaining, rejected };
  }
}

/**
 * Compress files in place. With createBackup the original is kept as
 * <name>.bak; without it the original is deleted once compression fully
 * succeeds (failures and skips never touch it).
 */
export async function compressFilesInPlace(
  filePaths: string[],
  pluginOrders: string[],
  createBackup: boolean = true
): Promise<InPlaceCompressionResult[]> {
  if (isTauri) {
    return await invoke<InPlaceCompressionResult[]>("compress_files_in_place", {
      filePaths,
      pluginOrders,
      createBackup
    });
  } else {
    // Mock in-place compression. Status is derived from the file name so the
    // three-state UI (compressed / skipped / failed) can be previewed in web
    // mode: "already-tiny" files skip (and are remembered by the mock skip
    // cache, like the backend), "locked" files fail with a permission error,
    // "missing" files fail with "File not found", the rest compress.
    await new Promise(resolve => setTimeout(resolve, 200));
    return filePaths.map(path => {
      if (path.includes("already-tiny")) {
        mockSkipCache.record(path);
        return {
          status: "skipped" as const,
          success: true,
          path,
          plugin_name: "WebP Converter",
          reason: "Compressed output (102400 bytes) is not smaller than the original (98304 bytes); original kept"
        };
      }
      if (path.includes("missing")) {
        return {
          status: "failed" as const,
          success: false,
          path,
          error: "File not found"
        };
      }
      if (path.includes("locked")) {
        return {
          status: "failed" as const,
          success: false,
          path,
          error: "Failed to back up original file: Permission denied (os error 13)"
        };
      }
      return {
        status: "compressed" as const,
        success: true,
        path,
        ...(createBackup ? { backup_path: `${path}.bak` } : {}),
        original_size: 1024000,
        compressed_size: 716800,
        savings: 307200,
        plugin_name: "WebP Converter"
      };
    });
  }
}

/**
 * Skip-cache info: how many "no size reduction" results are remembered
 */
export interface SkipCacheInfo {
  entries: number;
}

/**
 * Get the number of remembered no-size-reduction results
 */
export async function getSkipCacheInfo(): Promise<SkipCacheInfo> {
  if (isTauri) {
    return await invoke<SkipCacheInfo>("get_skip_cache_info");
  } else {
    return { entries: mockSkipCache.size() };
  }
}

/**
 * Forget all remembered no-size-reduction results; returns how many were removed
 */
export async function clearSkipCache(): Promise<number> {
  if (isTauri) {
    return await invoke<number>("clear_skip_cache");
  } else {
    return mockSkipCache.clear();
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
