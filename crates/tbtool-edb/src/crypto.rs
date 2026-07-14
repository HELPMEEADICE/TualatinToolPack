use md5::{Digest, Md5};

use crate::{Error, Result};

const CLEAR_PREFIX_LEN: usize = 4;
const BLOCK_LEN: usize = 4096;
const BLOCK_KEY_LEN: usize = 40;
const BLOCK_DROP_LEN: usize = 36;

#[derive(Clone)]
struct Rc4 {
    state: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    fn new(key: &[u8]) -> Result<Self> {
        if key.is_empty() {
            return Err(Error::EmptyPassword);
        }

        let mut state = [0u8; 256];
        for (index, value) in state.iter_mut().enumerate() {
            *value = index as u8;
        }

        let mut j = 0u8;
        for i in 0..256 {
            j = j.wrapping_add(state[i]).wrapping_add(key[i % key.len()]);
            state.swap(i, j as usize);
        }

        Ok(Self { state, i: 0, j: 0 })
    }

    fn next_byte(&mut self) -> u8 {
        self.i = self.i.wrapping_add(1);
        self.j = self.j.wrapping_add(self.state[self.i as usize]);
        self.state.swap(self.i as usize, self.j as usize);
        let index = self.state[self.i as usize].wrapping_add(self.state[self.j as usize]);
        self.state[index as usize]
    }

    fn discard(&mut self, count: usize) {
        for _ in 0..count {
            self.next_byte();
        }
    }

    fn apply(&mut self, data: &mut [u8]) {
        for byte in data {
            *byte ^= self.next_byte();
        }
    }

    fn generate_u32_le(&mut self) -> u32 {
        u32::from_le_bytes([
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
            self.next_byte(),
        ])
    }
}

pub fn password_verifier(password: &[u8]) -> [u8; 32] {
    let digest: [u8; 16] = Md5::digest(password).into();
    let mut verifier = [0u8; 32];
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (index, byte) in digest.into_iter().enumerate() {
        verifier[index * 2] = HEX[(byte >> 4) as usize];
        verifier[index * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }

    for pair in verifier.chunks_exact_mut(2) {
        pair.swap(0, 1);
    }
    verifier.reverse();
    verifier
}

/// Applies the EDB/EDT stream transform to a complete file image.
///
/// The transform is symmetric. Calling it twice with the same password restores
/// the original bytes. The four-byte file signature is intentionally left clear.
pub fn crypt_in_place(file: &mut [u8], password: &[u8]) -> Result<()> {
    if file.len() <= CLEAR_PREFIX_LEN {
        return Ok(());
    }

    let verifier = password_verifier(password);
    let mut seed_stream = Rc4::new(password)?;
    let block_count = file.len().div_ceil(BLOCK_LEN);

    for block_index in 0..block_count {
        let seed = seed_stream.generate_u32_le();
        let block_start = (block_index * BLOCK_LEN).max(CLEAR_PREFIX_LEN);
        let block_end = ((block_index + 1) * BLOCK_LEN).min(file.len());
        if block_start >= block_end {
            continue;
        }

        let mut block_key = [0u8; BLOCK_KEY_LEN];
        block_key[..4].copy_from_slice(&seed.to_le_bytes());
        block_key[4..36].copy_from_slice(&verifier);
        block_key[36..].copy_from_slice(&(seed ^ block_index as u32).to_le_bytes());

        let mut cipher = Rc4::new(&block_key)?;
        cipher.discard(BLOCK_DROP_LEN + block_start % BLOCK_LEN);
        cipher.apply(&mut file[block_start..block_end]);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_is_symmetric_and_preserves_magic() {
        let mut bytes: Vec<u8> = (0..10_000).map(|value| value as u8).collect();
        bytes[..4].copy_from_slice(b"WCDB");
        let original = bytes.clone();

        crypt_in_place(&mut bytes, b"tulading123").unwrap();
        assert_eq!(&bytes[..4], b"WCDB");
        assert_ne!(bytes, original);

        crypt_in_place(&mut bytes, b"tulading123").unwrap();
        assert_eq!(bytes, original);
    }
}
