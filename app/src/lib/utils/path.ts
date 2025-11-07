/**
 * Path utilities for validating and comparing paths
 */

/**
 * Normalize path for comparison (handles different separators and trailing slashes)
 */
export function normalizePath(path: string): string {
  // Convert backslashes to forward slashes
  let normalized = path.replace(/\\/g, '/');
  // Remove trailing slash unless it's the root
  if (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.slice(0, -1);
  }
  // Convert to lowercase for case-insensitive comparison (Windows)
  return normalized.toLowerCase();
}

/**
 * Check if a path is a subpath of another path
 * @param path The path to check
 * @param parentPath The potential parent path
 * @returns true if path is a subpath of parentPath
 */
export function isSubpath(path: string, parentPath: string): boolean {
  const normalizedPath = normalizePath(path);
  const normalizedParent = normalizePath(parentPath);
  
  // Same path is not considered a subpath
  if (normalizedPath === normalizedParent) {
    return false;
  }
  
  // Check if path starts with parent path followed by a separator
  return normalizedPath.startsWith(normalizedParent + '/');
}

/**
 * Check if a path is a parent of another path
 * @param path The path to check
 * @param childPath The potential child path
 * @returns true if path is a parent of childPath
 */
export function isParentPath(path: string, childPath: string): boolean {
  return isSubpath(childPath, path);
}

/**
 * Find which existing paths would make a new path redundant (new path is subpath of existing)
 * @param newPath The path to check
 * @param existingPaths List of existing paths
 * @returns Array of paths that contain the new path
 */
export function findParentPaths(newPath: string, existingPaths: string[]): string[] {
  return existingPaths.filter(existing => isSubpath(newPath, existing));
}

/**
 * Find which existing paths would become redundant if a new path is added (existing paths are subpaths of new)
 * @param newPath The path to check
 * @param existingPaths List of existing paths
 * @returns Array of paths that are contained within the new path
 */
export function findChildPaths(newPath: string, existingPaths: string[]): string[] {
  return existingPaths.filter(existing => isSubpath(existing, newPath));
}

/**
 * Get validation result for a path against existing paths
 */
export interface PathValidationResult {
  isValid: boolean;
  isDuplicate: boolean;
  isSubpathOf: string[];  // Paths that contain this path
  hasSubpaths: string[];  // Paths contained by this path
  warnings: string[];
}

export function validatePath(path: string, existingPaths: string[]): PathValidationResult {
  const warnings: string[] = [];
  const normalizedPath = normalizePath(path);
  
  // Check for duplicate
  const isDuplicate = existingPaths.some(existing => normalizePath(existing) === normalizedPath);
  if (isDuplicate) {
    warnings.push('This path is already in the list');
  }
  
  // Check if it's a subpath of any existing path
  const isSubpathOf = findParentPaths(path, existingPaths);
  if (isSubpathOf.length > 0) {
    warnings.push(`This path is already covered by: ${isSubpathOf.join(', ')}`);
  }
  
  // Check if any existing paths are subpaths of this path
  const hasSubpaths = findChildPaths(path, existingPaths);
  if (hasSubpaths.length > 0) {
    warnings.push(`This path would make redundant: ${hasSubpaths.join(', ')}`);
  }
  
  const isValid = !isDuplicate && isSubpathOf.length === 0;
  
  return {
    isValid,
    isDuplicate,
    isSubpathOf,
    hasSubpaths,
    warnings
  };
}
