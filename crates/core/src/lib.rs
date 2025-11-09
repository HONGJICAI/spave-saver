pub mod scanner;
pub mod hash;
pub mod image_sim;
pub mod video_sim;
pub mod compress;
pub mod filters;
pub mod compress_plugins;
pub mod plugins;

pub use scanner::{FileScanner, FileInfo};
pub use hash::{HashAlgorithm, FileHasher};
pub use image_sim::ImageSimilarity;
pub use video_sim::VideoSimilarity;
pub use compress::Compressor;
pub use filters::FileFilter;
pub use compress_plugins::{
    CompressionPlugin, 
    CompressionResult, 
    PluginManager, 
    PluginMetadata,
    global_plugin_manager,
    init_plugin_manager_with,
};
pub use plugins::{WebPConverterPlugin, ImageZipToWebpZipPlugin, AnimatedWebPConverterPlugin};
