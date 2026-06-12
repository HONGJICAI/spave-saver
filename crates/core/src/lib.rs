pub mod compress;
pub mod compress_plugins;
pub mod filters;
pub mod hash;
pub mod image_sim;
pub mod plugins;
pub mod scanner;
pub mod skip_cache;
pub mod video_sim;

pub use compress::Compressor;
pub use compress_plugins::{
    global_plugin_manager, init_plugin_manager_with, CompressionOutcome, CompressionPlugin,
    CompressionResult, PluginManager, PluginMetadata,
};
pub use filters::FileFilter;
pub use hash::{FileHasher, HashAlgorithm};
pub use image_sim::ImageSimilarity;
pub use plugins::{AnimatedWebPConverterPlugin, ImageZipToWebpZipPlugin, WebPConverterPlugin};
pub use scanner::{FileInfo, FileScanner};
pub use skip_cache::{FileFingerprint, SkipCache};
pub use video_sim::VideoSimilarity;
