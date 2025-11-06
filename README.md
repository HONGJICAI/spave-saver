# Space-Saver (硬盘节省大师)

A powerful disk space management tool built with Rust and Tauri, designed to help you find and clean up wasted space on your drives.

## 🏗️ Architecture

This project uses a **Rust Workspace** architecture with modular design for performance and scalability.

```
space-saver/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── core/               # Core algorithms (scanning, hashing, similarity)
│   ├── service/            # Service layer (task scheduling, API)
│   ├── db/                 # Database layer (SQLite, cache)
│   ├── utils/              # Common utilities
│   └── cli/                # Command-line interface
└── app/
    ├── src-tauri/          # Tauri backend
    └── src/                # Frontend (Svelte)
```

## 🎯 Features

- **File Scanning**: Fast multi-threaded file system traversal
- **Duplicate Detection**: Find duplicate files using BLAKE3 hashing
- **Image Similarity**: Detect similar images using perceptual hashing
- **Empty File Detection**: Find and clean up empty files
- **File Compression**: Compress files and directories (ZIP, GZIP)
- **Storage Statistics**: Analyze disk usage by file type
- **CLI & GUI**: Both command-line and graphical interfaces

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- Node.js 18+ (for the frontend)
- pnpm (for package management)

### Building the Project

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd space-saver
   ```

2. **Build the workspace:**
   ```bash
   cargo build --release
   ```

3. **Run the CLI:**
   ```bash
   cargo run --bin space-saver -- --help
   ```

4. **Run the Tauri app:**
   ```bash
   cd app
   pnpm install
   pnpm tauri dev
   ```

## 📖 CLI Usage

### Scan a directory
```bash
space-saver scan /path/to/directory --detailed
```

### Find duplicate files
```bash
space-saver duplicates /path/to/directory --min-size 1024
```

### Find similar images
```bash
space-saver similar /path/to/images --threshold 0.9
```

### Find empty files
```bash
space-saver empty /path/to/directory --delete
```

### Show storage statistics
```bash
space-saver stats /path/to/directory
```

### Show configuration
```bash
space-saver config
```

## 🔧 Configuration

Configuration is stored in TOML format. Default location:
- **Windows**: `%APPDATA%\spacesaver\Space-Saver\config.toml`
- **Linux**: `~/.config/spacesaver/Space-Saver/config.toml`
- **macOS**: `~/Library/Application Support/com.spacesaver.Space-Saver/config.toml`

Example configuration:
```toml
database_path = "/path/to/spacesaver.db"
cache_dir = "/path/to/cache"
log_level = "info"
max_concurrent_tasks = 4
hash_algorithm = "Blake3"
image_similarity_threshold = 0.9

[scan]
follow_links = false
max_depth = null
min_file_size = 0
exclude_patterns = ["*.tmp", "*.cache", ".git/*", "node_modules/*"]
```

## 🧪 Testing

Run all tests:
```bash
cargo test --workspace
```

Run tests for a specific crate:
```bash
cargo test -p space-saver-core
```

## 📦 Technology Stack

| Feature | Library |
|---------|---------|
| Async Runtime | Tokio |
| Database | Rusqlite, sled |
| Logging | tracing, tracing-subscriber |
| Hashing | blake3, sha2 |
| Image Processing | image |
| Compression | zip, flate2 |
| Parallelism | rayon, crossbeam |
| Serialization | serde, bincode |
| CLI | clap, indicatif, comfy-table |
| Desktop | Tauri |

## 🏛️ Module Dependencies

```
[Tauri/CLI] ──→ [service] ──→ [core]
         │               │
         │               └── [db]
         └──→ [utils]
```

## 📝 Development Roadmap

- [x] Core file scanning engine
- [x] Duplicate file detection
- [x] Image similarity detection
- [ ] Video similarity detection (requires ffmpeg integration)
- [x] File compression
- [x] CLI interface
- [x] Tauri integration
- [ ] Frontend UI development
- [ ] Database query optimization
- [ ] Progress streaming to frontend
- [ ] Batch file operations
- [ ] Custom filter rules
