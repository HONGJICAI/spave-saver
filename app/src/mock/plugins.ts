import type { CompressionPlugin } from '$lib/api';

// Mirrors the three plugins registered in the backend's global plugin
// manager (crates/core/src/compress_plugins.rs). Shared by the
// getCompressionPlugins, setPluginQuality and scanCompressibleFiles mocks
// so plugin-name validation behaves like the backend.
export const mockPlugins: CompressionPlugin[] = [
  {
    name: 'Image ZIP to WebP ZIP',
    description: 'Converts images inside ZIP archives to WebP format',
    version: '1.0.0',
    quality: 85,
  },
  {
    name: 'WebP Converter',
    description: 'Converts PNG, JPEG, and other image formats to WebP',
    version: '1.0.0',
    quality: 85,
  },
  {
    name: 'Animated WebP Converter',
    description: 'Convert GIF to Animated WebP with lossy compression for better file size',
    version: '1.0.0',
    quality: 85,
  },
];

export function isKnownPlugin(name: string): boolean {
  return mockPlugins.some((p) => p.name === name);
}
