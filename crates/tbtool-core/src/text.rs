use std::borrow::Cow;

use encoding_rs::GBK;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Gbk,
}

pub fn decode_text(bytes: &[u8]) -> Result<(String, TextEncoding)> {
    if let Some(without_bom) = bytes.strip_prefix(b"\xef\xbb\xbf") {
        return String::from_utf8(without_bom.to_vec())
            .map(|text| (text, TextEncoding::Utf8Bom))
            .map_err(|_| Error::InvalidText);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_owned(), TextEncoding::Utf8));
    }
    let (text, _, had_errors) = GBK.decode(bytes);
    if had_errors {
        Err(Error::InvalidText)
    } else {
        Ok((text.into_owned(), TextEncoding::Gbk))
    }
}

pub fn encode_text(text: &str, encoding: TextEncoding) -> Result<Cow<'_, [u8]>> {
    match encoding {
        TextEncoding::Utf8 => Ok(Cow::Borrowed(text.as_bytes())),
        TextEncoding::Utf8Bom => {
            let mut output = Vec::with_capacity(text.len() + 3);
            output.extend_from_slice(b"\xef\xbb\xbf");
            output.extend_from_slice(text.as_bytes());
            Ok(Cow::Owned(output))
        }
        TextEncoding::Gbk => {
            let (encoded, _, had_errors) = GBK.encode(text);
            if had_errors {
                Err(Error::UnrepresentableText(encoding))
            } else {
                Ok(encoded)
            }
        }
    }
}
