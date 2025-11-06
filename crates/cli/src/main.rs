use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use comfy_table::{Table, presets::UTF8_FULL};

use space_saver_core::{FileScanner, scanner::DefaultFileScanner, FileHasher, FileFilter};
use space_saver_service::{ServiceApi, FileOperations};
use space_saver_utils::{init_logger, format_size, format_duration, Config};

/// Space Saver - Disk space management utility
#[derive(Parser)]
#[command(name = "space-saver")]
#[command(about = "A powerful disk space management tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory for files
    Scan {
        /// Directory to scan
        path: PathBuf,

        /// Show detailed output
        #[arg(short, long)]
        detailed: bool,
    },

    /// Find duplicate files
    Duplicates {
        /// Directory to scan
        path: PathBuf,

        /// Minimum file size to consider (in bytes)
        #[arg(short, long, default_value = "0")]
        min_size: u64,
    },

    /// Find similar images
    Similar {
        /// Directory to scan
        path: PathBuf,

        /// Similarity threshold (0.0 to 1.0)
        #[arg(short, long, default_value = "0.9")]
        threshold: f32,
    },

    /// Find empty files
    Empty {
        /// Directory to scan
        path: PathBuf,

        /// Delete empty files
        #[arg(short, long)]
        delete: bool,
    },

    /// Show storage statistics
    Stats {
        /// Directory to analyze
        path: PathBuf,
    },

    /// Show configuration
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger
    if cli.verbose {
        init_logger();
    }

    match cli.command {
        Commands::Scan { path, detailed } => {
            scan_command(path, detailed).await?;
        }
        Commands::Duplicates { path, min_size } => {
            duplicates_command(path, min_size).await?;
        }
        Commands::Similar { path, threshold } => {
            similar_command(path, threshold).await?;
        }
        Commands::Empty { path, delete } => {
            empty_command(path, delete).await?;
        }
        Commands::Stats { path } => {
            stats_command(path).await?;
        }
        Commands::Config => {
            config_command().await?;
        }
    }

    Ok(())
}

async fn scan_command(path: PathBuf, detailed: bool) -> Result<()> {
    println!("Scanning: {}", path.display());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message("Scanning files...");

    let scanner = DefaultFileScanner::new();
    let start = std::time::Instant::now();
    let files = scanner.scan(&path)?;
    let duration = start.elapsed();

    pb.finish_with_message("Scan completed");

    let total_size: u64 = files.iter().map(|f| f.size).sum();

    println!("\n📊 Scan Results:");
    println!("  Files found: {}", files.len());
    println!("  Total size: {}", format_size(total_size));
    println!("  Duration: {}", format_duration(duration));

    if detailed && !files.is_empty() {
        println!("\n📁 Top 10 largest files:");
        let mut sorted_files = files;
        sorted_files.sort_by(|a, b| b.size.cmp(&a.size));

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Size", "Path"]);

        for file in sorted_files.iter().take(10) {
            table.add_row(vec![
                format_size(file.size),
                file.path.display().to_string(),
            ]);
        }

        println!("{table}");
    }

    Ok(())
}

async fn duplicates_command(path: PathBuf, min_size: u64) -> Result<()> {
    println!("Finding duplicates in: {}", path.display());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message("Scanning and hashing files...");

    let api = ServiceApi::new();
    let duplicates = api.find_duplicates(path, None).await?;

    pb.finish_with_message("Analysis completed");

    if duplicates.is_empty() {
        println!("\n✅ No duplicate files found!");
        return Ok(());
    }

    let filtered: Vec<_> = duplicates
        .into_iter()
        .filter(|d| d.files[0].size >= min_size)
        .collect();

    let total_wasted: u64 = filtered.iter().map(|d| d.wasted_space).sum();

    println!("\n📊 Duplicate Files:");
    println!("  Groups found: {}", filtered.len());
    println!("  Wasted space: {}", format_size(total_wasted));

    for (idx, group) in filtered.iter().take(10).enumerate() {
        println!("\n  Group {} (Hash: {}...)", idx + 1, &group.hash[..8]);
        println!("    Files: {}", group.count);
        println!("    Size each: {}", format_size(group.files[0].size));
        println!("    Wasted: {}", format_size(group.wasted_space));
        
        for file in &group.files {
            println!("      - {}", file.path.display());
        }
    }

    Ok(())
}

async fn similar_command(path: PathBuf, threshold: f32) -> Result<()> {
    println!("Finding similar images in: {}", path.display());
    println!("Threshold: {:.2}", threshold);
    
    let pb = ProgressBar::new_spinner();
    pb.set_message("Analyzing images...");

    let api = ServiceApi::new();
    let similar = api.find_similar_images(path, threshold, None).await?;

    pb.finish_with_message("Analysis completed");

    if similar.is_empty() {
        println!("\n✅ No similar images found!");
        return Ok(());
    }

    println!("\n📊 Similar Images:");
    println!("  Groups found: {}", similar.len());

    for (idx, group) in similar.iter().take(10).enumerate() {
        println!("\n  Group {} (Similarity: {:.2}%)", idx + 1, group.similarity_score * 100.0);
        for file in &group.files {
            println!("    - {}", file.path.display());
        }
    }

    Ok(())
}

async fn empty_command(path: PathBuf, delete: bool) -> Result<()> {
    println!("Finding empty files in: {}", path.display());
    
    let scanner = DefaultFileScanner::new();
    let files = scanner.scan(&path)?;
    let filter = FileFilter::empty_files();
    let empty_files = filter.filter_files(files);

    if empty_files.is_empty() {
        println!("\n✅ No empty files found!");
        return Ok(());
    }

    println!("\n📊 Empty Files:");
    println!("  Count: {}", empty_files.len());

    if delete {
        let ops = FileOperations::new();
        let paths: Vec<_> = empty_files.iter().map(|f| f.path.clone()).collect();
        let deleted = ops.delete_files(&paths)?;
        println!("  Deleted: {}", deleted);
    } else {
        for file in empty_files.iter().take(20) {
            println!("  - {}", file.path.display());
        }
        if empty_files.len() > 20 {
            println!("  ... and {} more", empty_files.len() - 20);
        }
        println!("\nUse --delete flag to remove these files.");
    }

    Ok(())
}

async fn stats_command(path: PathBuf) -> Result<()> {
    println!("Analyzing: {}", path.display());
    
    let pb = ProgressBar::new_spinner();
    pb.set_message("Analyzing storage...");

    let api = ServiceApi::new();
    let stats = api.get_storage_stats(path, None).await?;

    pb.finish_with_message("Analysis completed");

    println!("\n📊 Storage Statistics:");
    println!("  Total files: {}", stats.total_files);
    println!("  Total size: {}", format_size(stats.total_size));
    println!("\n📁 By Type:");
    println!("  Images: {}", stats.images);
    println!("  Videos: {}", stats.videos);
    println!("  Documents: {}", stats.documents);
    println!("  Archives: {}", stats.archives);
    println!("  Others: {}", stats.others);
    println!("\n⚠️  Empty files: {}", stats.empty_files);

    Ok(())
}

async fn config_command() -> Result<()> {
    let config = Config::load_or_default();
    
    println!("📝 Configuration:");
    println!("{}", toml::to_string_pretty(&config)?);
    println!("\nConfig file: {}", Config::default_path().display());

    Ok(())
}
