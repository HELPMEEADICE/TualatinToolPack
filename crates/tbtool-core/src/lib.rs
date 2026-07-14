mod catalog;
mod hardware;
mod ini;
mod launcher;
mod skin;
mod text;

pub use hardware::{
    HardwareCategory, HardwareDevice, HardwareProperty, HardwareSection, HardwareSnapshot,
    collect_hardware_snapshot,
};
pub use ini::{IniDocument, IniEntry, IniSection};
pub use launcher::{
    ExecutableArchitecture, HostArchitecture, LaunchMethod, LaunchPlan, LaunchedTool, ToolLauncher,
    detect_executable_architecture,
};
pub use skin::{ImageFormat, SkinManifest, SkinPackage};
pub use text::{TextEncoding, decode_text, encode_text};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("input is not valid UTF-8 or GBK text")]
    InvalidText,

    #[error("text cannot be represented in {0:?}")]
    UnrepresentableText(TextEncoding),

    #[error("invalid INI line {line}: {reason}")]
    InvalidIni { line: usize, reason: &'static str },

    #[error("skin archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("skin archive entry is not a safe relative path: {0}")]
    UnsafeArchivePath(String),

    #[error("skin archive is missing {0}")]
    MissingSkinAsset(&'static str),

    #[error("skin asset is too large")]
    AssetTooLarge,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("EDB/EDT error: {0}")]
    Database(#[from] tbtool_edb::Error),

    #[error("tool database is missing field {0}")]
    MissingToolField(&'static str),

    #[error("unsafe or unsupported packaged tool path: {0}")]
    InvalidToolPath(String),

    #[error("built-in action cannot be launched as a process: {0}")]
    BuiltInAction(String),

    #[error("tool path does not exist: {0}")]
    ToolNotFound(String),

    #[error("resolved tool path escapes the package root: {0}")]
    ToolOutsideRoot(String),

    #[error("hardware detection failed: {0}")]
    HardwareDetection(String),

    #[error("this Windows host ({host}) cannot run a {executable} tool: {path}")]
    UnsupportedToolArchitecture {
        host: HostArchitecture,
        executable: ExecutableArchitecture,
        path: String,
    },
}
pub use catalog::{ToolCatalog, ToolCategory, ToolEntry, ToolTarget, normalize_packaged_path};
