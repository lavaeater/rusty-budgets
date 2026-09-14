#[cfg(not(target_os = "android"))]
mod native;
#[cfg(not(target_os = "android"))]
pub use native::{FileDialog, SaveFileDialog, save_bytes_to_file};

#[cfg(target_os = "android")]
mod unsupported;
#[cfg(target_os = "android")]
pub use unsupported::{FileDialog, SaveFileDialog, save_bytes_to_file};

#[derive(Clone, Debug)]
pub struct FileData {
    pub name: String,
    pub contents: Vec<u8>,
}
