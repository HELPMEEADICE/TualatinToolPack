use std::collections::HashSet;

use crate::{
    Error, MAGIC_ENCRYPTED_TABLE, Result, crypt_in_place,
    database::{VERSION_1, read_array, read_u32},
};

pub const TABLE_PAGE_LEN: usize = 512;
pub const TABLE_PAGE_HEADER_LEN: usize = 12;
pub const TABLE_PAGE_PAYLOAD_LEN: usize = TABLE_PAGE_LEN - TABLE_PAGE_HEADER_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableHeader {
    pub version: u32,
    pub timestamp_bits: u64,
    pub page_count: u32,
    pub free_page_count: u32,
    pub first_free_page: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablePage {
    pub previous: u32,
    pub next: u32,
    pub used: u32,
    pub payload: [u8; TABLE_PAGE_PAYLOAD_LEN],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub header: TableHeader,
    pub pages: Vec<TablePage>,
    header_padding: [u8; TABLE_PAGE_LEN - 32],
}

impl Table {
    pub fn open(mut bytes: Vec<u8>, password: Option<&[u8]>) -> Result<Self> {
        let magic = read_array::<4>(&bytes, 0)?;
        if magic != MAGIC_ENCRYPTED_TABLE {
            return Err(Error::InvalidMagic(magic));
        }
        if read_u32(&bytes, 4)? != VERSION_1 {
            let password = password.ok_or(Error::EmptyPassword)?;
            crypt_in_place(&mut bytes, password)?;
        }
        Self::from_decrypted_bytes(&bytes)
    }

    pub fn from_decrypted_bytes(bytes: &[u8]) -> Result<Self> {
        let magic = read_array::<4>(bytes, 0)?;
        if magic != MAGIC_ENCRYPTED_TABLE {
            return Err(Error::InvalidMagic(magic));
        }
        let version = read_u32(bytes, 4)?;
        if version != VERSION_1 {
            return Err(Error::UnsupportedVersion(version));
        }
        if bytes.len() < TABLE_PAGE_LEN {
            return Err(Error::Truncated {
                expected: TABLE_PAGE_LEN,
                actual: bytes.len(),
            });
        }
        if bytes.len() % TABLE_PAGE_LEN != 0 {
            return Err(Error::InvalidLayout("table file is not page aligned"));
        }
        let page_count = read_u32(bytes, 16)? as usize;
        if page_count != bytes.len() / TABLE_PAGE_LEN {
            return Err(Error::InvalidLayout(
                "header page count does not match file size",
            ));
        }

        let mut pages = Vec::with_capacity(page_count.saturating_sub(1));
        for index in 1..page_count {
            let offset = index * TABLE_PAGE_LEN;
            let used = read_u32(bytes, offset + 8)?;
            if used as usize > TABLE_PAGE_PAYLOAD_LEN {
                return Err(Error::InvalidLayout(
                    "table page payload length exceeds 500 bytes",
                ));
            }
            let previous = read_u32(bytes, offset)?;
            let next = read_u32(bytes, offset + 4)?;
            if previous as usize >= page_count || next as usize >= page_count {
                return Err(Error::InvalidLayout("table page link is outside the file"));
            }
            pages.push(TablePage {
                previous,
                next,
                used,
                payload: read_array(bytes, offset + TABLE_PAGE_HEADER_LEN)?,
            });
        }

        Ok(Self {
            header: TableHeader {
                version,
                timestamp_bits: u64::from_le_bytes(read_array(bytes, 8)?),
                page_count: page_count as u32,
                free_page_count: read_u32(bytes, 20)?,
                first_free_page: read_u32(bytes, 24)?,
                reserved: read_u32(bytes, 28)?,
            },
            pages,
            header_padding: read_array(bytes, 32)?,
        })
    }

    pub fn page(&self, page_index: u32) -> Result<&TablePage> {
        if page_index == 0 {
            return Err(Error::InvalidLayout("page zero is the table header"));
        }
        self.pages
            .get(page_index as usize - 1)
            .ok_or(Error::IndexOutOfBounds {
                index: page_index as usize,
                len: self.pages.len() + 1,
            })
    }

    pub fn read_chain(&self, first_page: u32) -> Result<Vec<u8>> {
        if first_page == 0 {
            return Ok(Vec::new());
        }
        let mut output = Vec::new();
        let mut visited = HashSet::new();
        let mut current = first_page;
        let mut expected_previous = 0;
        while current != 0 {
            if !visited.insert(current) {
                return Err(Error::InvalidLayout("cycle in table page chain"));
            }
            let page = self.page(current)?;
            if page.previous != expected_previous {
                return Err(Error::InvalidLayout(
                    "broken previous link in table page chain",
                ));
            }
            output.extend_from_slice(&page.payload[..page.used as usize]);
            expected_previous = current;
            current = page.next;
        }
        Ok(output)
    }

    pub fn free_pages(&self) -> Result<Vec<u32>> {
        let mut pages = Vec::with_capacity(self.header.free_page_count as usize);
        let mut visited = HashSet::new();
        let mut current = self.header.first_free_page;
        let mut expected_previous = 0;
        while current != 0 {
            if !visited.insert(current) {
                return Err(Error::InvalidLayout("cycle in table free-page list"));
            }
            let page = self.page(current)?;
            if page.previous != expected_previous || page.used != 0 {
                return Err(Error::InvalidLayout("invalid free page"));
            }
            pages.push(current);
            expected_previous = current;
            current = page.next;
        }
        if pages.len() != self.header.free_page_count as usize {
            return Err(Error::InvalidLayout(
                "free-page count does not match its list",
            ));
        }
        Ok(pages)
    }

    pub fn replace_chain(&mut self, first_page: u32, data: &[u8]) -> Result<u32> {
        let existing = self.chain_pages(first_page)?;
        if data.is_empty() {
            for page in existing {
                self.free_page(page)?;
            }
            return Ok(0);
        }

        let required = data.len().div_ceil(TABLE_PAGE_PAYLOAD_LEN);
        let mut selected: Vec<u32> = existing.iter().copied().take(required).collect();
        while selected.len() < required {
            let page = self.allocate_page()?;
            selected.push(page);
        }
        for page in existing.into_iter().skip(required) {
            self.free_page(page)?;
        }

        for (position, (&page_index, chunk)) in selected
            .iter()
            .zip(data.chunks(TABLE_PAGE_PAYLOAD_LEN))
            .enumerate()
        {
            let previous = position
                .checked_sub(1)
                .map(|index| selected[index])
                .unwrap_or(0);
            let next = selected.get(position + 1).copied().unwrap_or(0);
            let page = self.page_mut(page_index)?;
            page.previous = previous;
            page.next = next;
            page.used = chunk.len() as u32;
            page.payload.fill(0);
            page.payload[..chunk.len()].copy_from_slice(chunk);
        }
        Ok(selected[0])
    }

    pub fn to_bytes(&self, password: Option<&[u8]>) -> Result<Vec<u8>> {
        if self.pages.len() + 1 != self.header.page_count as usize {
            return Err(Error::InvalidLayout("table page count is inconsistent"));
        }
        let mut bytes = Vec::with_capacity(self.header.page_count as usize * TABLE_PAGE_LEN);
        bytes.extend_from_slice(&MAGIC_ENCRYPTED_TABLE);
        bytes.extend_from_slice(&self.header.version.to_le_bytes());
        bytes.extend_from_slice(&self.header.timestamp_bits.to_le_bytes());
        bytes.extend_from_slice(&self.header.page_count.to_le_bytes());
        bytes.extend_from_slice(&self.header.free_page_count.to_le_bytes());
        bytes.extend_from_slice(&self.header.first_free_page.to_le_bytes());
        bytes.extend_from_slice(&self.header.reserved.to_le_bytes());
        bytes.extend_from_slice(&self.header_padding);
        for page in &self.pages {
            if page.used as usize > TABLE_PAGE_PAYLOAD_LEN {
                return Err(Error::InvalidLayout(
                    "table page payload length exceeds 500 bytes",
                ));
            }
            bytes.extend_from_slice(&page.previous.to_le_bytes());
            bytes.extend_from_slice(&page.next.to_le_bytes());
            bytes.extend_from_slice(&page.used.to_le_bytes());
            bytes.extend_from_slice(&page.payload);
        }
        if let Some(password) = password {
            crypt_in_place(&mut bytes, password)?;
        }
        Ok(bytes)
    }

    fn chain_pages(&self, first_page: u32) -> Result<Vec<u32>> {
        let mut pages = Vec::new();
        let mut visited = HashSet::new();
        let mut current = first_page;
        let mut expected_previous = 0;
        while current != 0 {
            if !visited.insert(current) {
                return Err(Error::InvalidLayout("cycle in table page chain"));
            }
            let page = self.page(current)?;
            if page.previous != expected_previous {
                return Err(Error::InvalidLayout(
                    "broken previous link in table page chain",
                ));
            }
            pages.push(current);
            expected_previous = current;
            current = page.next;
        }
        Ok(pages)
    }

    fn page_mut(&mut self, page_index: u32) -> Result<&mut TablePage> {
        if page_index == 0 {
            return Err(Error::InvalidLayout("page zero is the table header"));
        }
        let len = self.pages.len() + 1;
        self.pages
            .get_mut(page_index as usize - 1)
            .ok_or(Error::IndexOutOfBounds {
                index: page_index as usize,
                len,
            })
    }

    fn allocate_page(&mut self) -> Result<u32> {
        if self.header.free_page_count > 0 {
            let page_index = self.header.first_free_page;
            if page_index == 0 {
                return Err(Error::InvalidLayout(
                    "free-page count is nonzero but list is empty",
                ));
            }
            let next = self.page(page_index)?.next;
            self.header.first_free_page = next;
            self.header.free_page_count -= 1;
            *self.page_mut(page_index)? = empty_page();
            if next != 0 {
                self.page_mut(next)?.previous = 0;
            }
            Ok(page_index)
        } else {
            let page_index = self.pages.len() + 1;
            if page_index > u32::MAX as usize {
                return Err(Error::InvalidLayout("table page index overflow"));
            }
            self.pages.push(empty_page());
            self.header.page_count = self.pages.len() as u32 + 1;
            Ok(page_index as u32)
        }
    }

    fn free_page(&mut self, page_index: u32) -> Result<()> {
        let first_free_page = self.header.first_free_page;
        if first_free_page != 0 {
            self.page_mut(first_free_page)?.previous = page_index;
        }
        let page = self.page_mut(page_index)?;
        *page = empty_page();
        page.next = first_free_page;
        self.header.first_free_page = page_index;
        self.header.free_page_count = self
            .header
            .free_page_count
            .checked_add(1)
            .ok_or(Error::InvalidLayout("free-page count overflow"))?;
        Ok(())
    }
}

fn empty_page() -> TablePage {
    TablePage {
        previous: 0,
        next: 0,
        used: 0,
        payload: [0; TABLE_PAGE_PAYLOAD_LEN],
    }
}
