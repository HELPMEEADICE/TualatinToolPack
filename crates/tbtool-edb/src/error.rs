use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("file is truncated: expected at least {expected} bytes, got {actual}")]
    Truncated { expected: usize, actual: usize },

    #[error("unsupported EDB/EDT magic: {0:02X?}")]
    InvalidMagic([u8; 4]),

    #[error("encrypted EDB/EDT requires a non-empty password")]
    EmptyPassword,

    #[error("password does not match the encrypted file")]
    InvalidPassword,

    #[error("unsupported format version 0x{0:08X}")]
    UnsupportedVersion(u32),

    #[error("invalid EDB/EDT layout: {0}")]
    InvalidLayout(&'static str),

    #[error("index {index} is outside the available {len} entries")]
    IndexOutOfBounds { index: usize, len: usize },

    #[error("field value requires {required} bytes but only {available} are available")]
    FieldTooLong { required: usize, available: usize },

    #[error("text cannot be represented in GBK")]
    UnrepresentableText,
}
