mod crypto;
mod database;
mod error;
mod table;

pub use crypto::{crypt_in_place, password_verifier};
pub use database::{
    DATABASE_HEADER_LEN, Database, DatabaseHeader, FIELD_DESCRIPTOR_LEN, FieldDescriptor, VERSION_1,
};
pub use error::{Error, Result};
pub use table::{
    TABLE_PAGE_HEADER_LEN, TABLE_PAGE_LEN, TABLE_PAGE_PAYLOAD_LEN, Table, TableHeader, TablePage,
};

pub const MAGIC_ENCRYPTED_DATABASE: [u8; 4] = *b"WCDB";
pub const MAGIC_DATABASE: [u8; 4] = *b"WEDB";
pub const MAGIC_ENCRYPTED_TABLE: [u8; 4] = *b"WEDT";
pub const MAGIC_TABLE: [u8; 4] = *b"WEDT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Database,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileHeader {
    pub kind: FileKind,
    pub encrypted: bool,
}

impl FileHeader {
    pub fn detect(bytes: &[u8]) -> Result<Self> {
        let magic: [u8; 4] = bytes
            .get(..4)
            .ok_or(Error::Truncated {
                expected: 4,
                actual: bytes.len(),
            })?
            .try_into()
            .expect("slice length checked");

        match magic {
            MAGIC_ENCRYPTED_DATABASE => Ok(Self {
                kind: FileKind::Database,
                encrypted: true,
            }),
            MAGIC_DATABASE => Ok(Self {
                kind: FileKind::Database,
                encrypted: false,
            }),
            MAGIC_ENCRYPTED_TABLE => Ok(Self {
                kind: FileKind::Table,
                encrypted: bytes.get(4..8) != Some(&VERSION_1.to_le_bytes()),
            }),
            _ => Err(Error::InvalidMagic(magic)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_encrypted_database() {
        assert_eq!(
            FileHeader::detect(b"WCDBpayload").unwrap(),
            FileHeader {
                kind: FileKind::Database,
                encrypted: true,
            }
        );
    }

    #[test]
    fn rejects_short_input() {
        assert!(matches!(
            FileHeader::detect(b"WC"),
            Err(Error::Truncated {
                expected: 4,
                actual: 2
            })
        ));
    }
}
