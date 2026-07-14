use encoding_rs::GBK;

use crate::{
    Error, MAGIC_DATABASE, MAGIC_ENCRYPTED_DATABASE, Result, crypt_in_place, password_verifier,
};

pub const VERSION_1: u32 = 0x0001_0000;
pub const DATABASE_HEADER_LEN: usize = 112;
pub const FIELD_DESCRIPTOR_LEN: usize = 72;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseHeader {
    pub version: u32,
    pub timestamp_bits: u64,
    pub record_count: u32,
    pub max_record_id: u32,
    pub record_size: u32,
    pub password_verifier: [u8; 32],
    pub reserved: [u8; 52],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: [u8; 20],
    pub field_type: i32,
    pub offset: u32,
    pub size: u32,
    pub reserved: [u8; 40],
}

impl FieldDescriptor {
    pub fn name_gbk(&self) -> String {
        decode_gbk(c_string(&self.name))
    }

    pub fn storage_size(&self) -> Result<usize> {
        match self.field_type {
            1 | 7 => Ok(1),
            2 => Ok(2),
            3 | 5 | 9 | 11 | 12 => Ok(4),
            4 | 6 | 8 => Ok(8),
            10 if self.size > 0 => Ok(self.size as usize),
            10 => Err(Error::InvalidLayout("text field size is zero")),
            _ => Err(Error::InvalidLayout("unsupported field type")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Database {
    pub header: DatabaseHeader,
    pub fields: Vec<FieldDescriptor>,
    records: Vec<Vec<u8>>,
}

impl Database {
    pub fn open(mut bytes: Vec<u8>, password: Option<&[u8]>) -> Result<Self> {
        let magic = read_array::<4>(&bytes, 0)?;
        match magic {
            MAGIC_ENCRYPTED_DATABASE => {
                let password = password.ok_or(Error::EmptyPassword)?;
                crypt_in_place(&mut bytes, password)?;
                let stored = read_array::<32>(&bytes, 28)?;
                if stored != password_verifier(password) {
                    return Err(Error::InvalidPassword);
                }
            }
            MAGIC_DATABASE => {}
            _ => return Err(Error::InvalidMagic(magic)),
        }
        Self::from_decrypted_bytes(&bytes)
    }

    pub fn from_decrypted_bytes(bytes: &[u8]) -> Result<Self> {
        let magic = read_array::<4>(bytes, 0)?;
        if magic != MAGIC_DATABASE && magic != MAGIC_ENCRYPTED_DATABASE {
            return Err(Error::InvalidMagic(magic));
        }

        let version = read_u32(bytes, 4)?;
        if version != VERSION_1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let record_count = read_u32(bytes, 16)?;
        let record_size = read_u32(bytes, 24)?;
        if record_size == 0 {
            return Err(Error::InvalidLayout("record size is zero"));
        }

        let field_count = read_u32(bytes, DATABASE_HEADER_LEN)? as usize;
        let descriptor_bytes = field_count
            .checked_mul(FIELD_DESCRIPTOR_LEN)
            .ok_or(Error::InvalidLayout("field descriptor size overflow"))?;
        let data_offset = DATABASE_HEADER_LEN
            .checked_add(4)
            .and_then(|value| value.checked_add(descriptor_bytes))
            .ok_or(Error::InvalidLayout("record offset overflow"))?;
        if data_offset > bytes.len() {
            return Err(Error::Truncated {
                expected: data_offset,
                actual: bytes.len(),
            });
        }

        let record_size = record_size as usize;
        let data = &bytes[data_offset..];
        if data.len() % record_size != 0 {
            return Err(Error::InvalidLayout(
                "record data is not record-size aligned",
            ));
        }
        let actual_record_count = data.len() / record_size;
        if actual_record_count != record_count as usize {
            return Err(Error::InvalidLayout(
                "header record count does not match file size",
            ));
        }

        let mut fields = Vec::with_capacity(field_count);
        for index in 0..field_count {
            let offset = DATABASE_HEADER_LEN + 4 + index * FIELD_DESCRIPTOR_LEN;
            let descriptor = &bytes[offset..offset + FIELD_DESCRIPTOR_LEN];
            let field = FieldDescriptor {
                name: descriptor[..20].try_into().expect("fixed descriptor slice"),
                field_type: i32::from_le_bytes(
                    descriptor[20..24]
                        .try_into()
                        .expect("fixed descriptor slice"),
                ),
                offset: u32::from_le_bytes(
                    descriptor[24..28]
                        .try_into()
                        .expect("fixed descriptor slice"),
                ),
                size: u32::from_le_bytes(
                    descriptor[28..32]
                        .try_into()
                        .expect("fixed descriptor slice"),
                ),
                reserved: descriptor[32..72]
                    .try_into()
                    .expect("fixed descriptor slice"),
            };
            let end = (field.offset as usize)
                .checked_add(field.storage_size()?)
                .ok_or(Error::InvalidLayout("field range overflow"))?;
            if end > record_size {
                return Err(Error::InvalidLayout("field extends beyond its record"));
            }
            fields.push(field);
        }

        let records = data.chunks_exact(record_size).map(Vec::from).collect();
        Ok(Self {
            header: DatabaseHeader {
                version,
                timestamp_bits: read_u64(bytes, 8)?,
                record_count,
                max_record_id: read_u32(bytes, 20)?,
                record_size: record_size as u32,
                password_verifier: read_array::<32>(bytes, 28)?,
                reserved: read_array::<52>(bytes, 60)?,
            },
            fields,
            records,
        })
    }

    pub fn records(&self) -> &[Vec<u8>] {
        &self.records
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|field| field.name_gbk() == name)
    }

    pub fn record_id(&self, record_index: usize) -> Result<u32> {
        let record = self.record(record_index)?;
        read_u32(record, 0)
    }

    pub fn field_bytes(&self, record_index: usize, field_index: usize) -> Result<&[u8]> {
        let record = self.record(record_index)?;
        let field = self.field(field_index)?;
        let start = field.offset as usize;
        Ok(&record[start..start + field.storage_size()?])
    }

    pub fn field_text_gbk(&self, record_index: usize, field_index: usize) -> Result<String> {
        Ok(decode_gbk(c_string(
            self.field_bytes(record_index, field_index)?,
        )))
    }

    pub fn set_field_bytes(
        &mut self,
        record_index: usize,
        field_index: usize,
        value: &[u8],
    ) -> Result<()> {
        let field = self.field(field_index)?.clone();
        let storage_size = field.storage_size()?;
        if value.len() > storage_size {
            return Err(Error::FieldTooLong {
                required: value.len(),
                available: storage_size,
            });
        }
        let record = self.record_mut(record_index)?;
        let start = field.offset as usize;
        let target = &mut record[start..start + storage_size];
        target.fill(0);
        target[..value.len()].copy_from_slice(value);
        Ok(())
    }

    pub fn set_field_text_gbk(
        &mut self,
        record_index: usize,
        field_index: usize,
        value: &str,
    ) -> Result<()> {
        let (encoded, _, had_errors) = GBK.encode(value);
        if had_errors {
            return Err(Error::UnrepresentableText);
        }
        self.set_field_bytes(record_index, field_index, &encoded)
    }

    pub fn push_empty_record(&mut self) -> Result<usize> {
        self.header.max_record_id = self
            .header
            .max_record_id
            .checked_add(1)
            .ok_or(Error::InvalidLayout("record id overflow"))?;
        let mut record = vec![0; self.header.record_size as usize];
        record[..4].copy_from_slice(&self.header.max_record_id.to_le_bytes());
        self.records.push(record);
        self.header.record_count = self.records.len() as u32;
        Ok(self.records.len() - 1)
    }

    pub fn remove_record(&mut self, record_index: usize) -> Result<Vec<u8>> {
        if record_index >= self.records.len() {
            return Err(Error::IndexOutOfBounds {
                index: record_index,
                len: self.records.len(),
            });
        }
        let record = self.records.remove(record_index);
        self.header.record_count = self.records.len() as u32;
        Ok(record)
    }

    pub fn to_bytes(&self, password: Option<&[u8]>) -> Result<Vec<u8>> {
        if self.records.len() > u32::MAX as usize || self.fields.len() > u32::MAX as usize {
            return Err(Error::InvalidLayout("entry count exceeds format limits"));
        }
        let mut bytes = Vec::with_capacity(
            DATABASE_HEADER_LEN
                + 4
                + self.fields.len() * FIELD_DESCRIPTOR_LEN
                + self.records.len() * self.header.record_size as usize,
        );
        bytes.extend_from_slice(if password.is_some() {
            &MAGIC_ENCRYPTED_DATABASE
        } else {
            &MAGIC_DATABASE
        });
        bytes.extend_from_slice(&self.header.version.to_le_bytes());
        bytes.extend_from_slice(&self.header.timestamp_bits.to_le_bytes());
        bytes.extend_from_slice(&(self.records.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.header.max_record_id.to_le_bytes());
        bytes.extend_from_slice(&self.header.record_size.to_le_bytes());
        bytes.extend_from_slice(
            &password
                .map(password_verifier)
                .unwrap_or(self.header.password_verifier),
        );
        bytes.extend_from_slice(&self.header.reserved);
        bytes.extend_from_slice(&(self.fields.len() as u32).to_le_bytes());
        for field in &self.fields {
            bytes.extend_from_slice(&field.name);
            bytes.extend_from_slice(&field.field_type.to_le_bytes());
            bytes.extend_from_slice(&field.offset.to_le_bytes());
            bytes.extend_from_slice(&field.size.to_le_bytes());
            bytes.extend_from_slice(&field.reserved);
        }
        for record in &self.records {
            if record.len() != self.header.record_size as usize {
                return Err(Error::InvalidLayout("record has an unexpected size"));
            }
            bytes.extend_from_slice(record);
        }
        if let Some(password) = password {
            crypt_in_place(&mut bytes, password)?;
        }
        Ok(bytes)
    }

    fn record(&self, index: usize) -> Result<&[u8]> {
        self.records
            .get(index)
            .map(Vec::as_slice)
            .ok_or(Error::IndexOutOfBounds {
                index,
                len: self.records.len(),
            })
    }

    fn record_mut(&mut self, index: usize) -> Result<&mut [u8]> {
        let len = self.records.len();
        self.records
            .get_mut(index)
            .map(Vec::as_mut_slice)
            .ok_or(Error::IndexOutOfBounds { index, len })
    }

    fn field(&self, index: usize) -> Result<&FieldDescriptor> {
        self.fields.get(index).ok_or(Error::IndexOutOfBounds {
            index,
            len: self.fields.len(),
        })
    }
}

pub(crate) fn c_string(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}

pub(crate) fn decode_gbk(bytes: &[u8]) -> String {
    let (decoded, _, _) = GBK.decode(bytes);
    decoded.into_owned()
}

pub(crate) fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N]> {
    bytes
        .get(offset..offset + N)
        .ok_or(Error::Truncated {
            expected: offset + N,
            actual: bytes.len(),
        })?
        .try_into()
        .map_err(|_| Error::InvalidLayout("fixed-size read failed"))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(read_array(bytes, offset)?))
}
