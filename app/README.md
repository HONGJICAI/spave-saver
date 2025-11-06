# Space-Saver Frontend

A dual-mode SvelteKit frontend for the Space-Saver disk management application. Works both as a standalone web app (with mock data) and as a Tauri desktop application (with Rust backend).

## Features

- 🔍 **Scan Directory** - Analyze folder contents and display files
- 🔄 **Find Duplicates** - Detect duplicate files by hash
- 🖼️ **Similar Images** - Find visually similar images
- 📄 **Empty Files** - Locate and remove empty files
- 🗜️ **Compress Files** - Compress files and folders
- 📊 **Statistics** - View storage analysis and file type distribution

## Dual Mode Support

### Web Mode (Mock Data)
```bash
pnpm dev:web
# or
pnpm build:web
```

Uses mock data providers for all backend operations. Perfect for:
- Frontend development without Rust backend
- Testing UI components
- Demos and prototypes

### Tauri Mode (Real Backend)
```bash
pnpm tauri dev
# or
pnpm tauri build
```

Connects to the Rust backend via Tauri IPC. Requires:
- Rust toolchain installed
- Tauri CLI configured
- Backend crates compiled

## Development

### Install Dependencies
```bash
pnpm install
```

### Run Development Server
```bash
# Web mode with mocks
pnpm dev:web

# Tauri mode with backend
pnpm tauri dev
```

### Run Tests
```bash
# Run tests
pnpm test

# Run tests in watch mode
pnpm test:watch

# Run tests with UI
pnpm test:ui

# Generate coverage report
pnpm test:coverage
```

## Project Structure

```
app/
├── src/
│   ├── lib/
│   │   ├── api/           # Unified API layer
│   │   ├── components/    # Svelte components
│   │   ├── stores/        # Svelte stores
│   │   └── utils/         # Utility functions
│   ├── mock/              # Mock data providers
│   ├── routes/            # SvelteKit routes/pages
│   └── test/              # Test setup and helpers
├── src-tauri/             # Tauri backend (Rust)
├── static/                # Static assets
└── tests/                 # E2E tests
```

## API Layer

The API layer automatically detects the runtime environment:

```typescript
// Detects Tauri vs Web mode
const isTauri = "__TAURI_INTERNALS__" in window;

// Routes calls appropriately
if (isTauri) {
  // Use Tauri IPC
  return await invoke('scan_directory', { path });
} else {
  // Use mock data
  return await mockScanDirectory(path);
}
```

## Technologies

- **SvelteKit** 2.9.0 - Framework
- **Svelte** 5.0 - UI library
- **TypeScript** ~5.6.2 - Type safety
- **Vite** 6.0.3 - Build tool
- **Vitest** 2.1.0 - Testing framework
- **Tailwind CSS** 4.1.13 - Styling
- **Flowbite Svelte** 0.46.0 - UI components
- **Tauri** 2.x - Desktop integration

## Build

### Web Build
```bash
pnpm build:web
```

Creates a static web application in `build/` directory.

### Tauri Build
```bash
pnpm tauri build
```

Creates platform-specific installers in `src-tauri/target/release/bundle/`.

## Environment Variables

- `VITE_MODE` - Set to 'web' or 'tauri' to force mode
- `TAURI_DEV_HOST` - Override dev server host in Tauri mode
