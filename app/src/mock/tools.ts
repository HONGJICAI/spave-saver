import type { ToolStatus } from '$lib/types';

/**
 * Web-mode external tool detection. Returns a deliberate mix so the settings
 * page demonstrates both states the backend can report:
 *   - ffmpeg / ffprobe "available" (these gate the planned video / animated
 *     compression — detection is what unlocks those features)
 *   - cwebp "not found" (the built-in Rust WebP encoder is used regardless,
 *     so a missing cwebp is informational, not an error)
 * Mirrors crates/service/src/tools.rs detect_tools(): name, availability,
 * resolved path and a best-effort version line.
 */
export function mockDetectTools(): Promise<ToolStatus[]> {
  return new Promise((resolve) =>
    setTimeout(
      () =>
        resolve([
          {
            name: 'ffmpeg',
            available: true,
            path: '/usr/bin/ffmpeg',
            version: 'ffmpeg version 6.1.1',
            purpose: 'Video and animated-image compression',
          },
          {
            name: 'ffprobe',
            available: true,
            path: '/usr/bin/ffprobe',
            version: 'ffprobe version 6.1.1',
            purpose: 'Inspecting video/audio streams for compression',
          },
          {
            name: 'cwebp',
            available: false,
            path: null,
            version: null,
            purpose: 'Standalone WebP encoding (the built-in encoder is used by default)',
          },
        ]),
      150
    )
  );
}
