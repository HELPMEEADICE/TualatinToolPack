use crate::{Result, TextEncoding, decode_text, encode_text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniSection {
    pub name: String,
    pub entries: Vec<IniEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniDocument {
    pub sections: Vec<IniSection>,
    pub encoding: TextEncoding,
    line_ending: &'static str,
    trailing_newline: bool,
}

impl IniDocument {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (text, encoding) = decode_text(bytes)?;
        let line_ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing_newline = text.ends_with('\n');
        let mut sections = Vec::<IniSection>::new();

        for (line_index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim_end_matches('\r').trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                let Some(name) = line
                    .strip_prefix('[')
                    .and_then(|line| line.strip_suffix(']'))
                else {
                    return Err(crate::Error::InvalidIni {
                        line: line_index + 1,
                        reason: "malformed section header",
                    });
                };
                sections.push(IniSection {
                    name: name.trim().to_owned(),
                    entries: Vec::new(),
                });
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(crate::Error::InvalidIni {
                    line: line_index + 1,
                    reason: "expected key=value",
                });
            };
            let Some(section) = sections.last_mut() else {
                return Err(crate::Error::InvalidIni {
                    line: line_index + 1,
                    reason: "entry appears before the first section",
                });
            };
            section.entries.push(IniEntry {
                key: key.trim().to_owned(),
                value: value.trim().to_owned(),
            });
        }

        Ok(Self {
            sections,
            encoding,
            line_ending,
            trailing_newline,
        })
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(section))?
            .entries
            .iter()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
            .map(|entry| entry.value.as_str())
    }

    pub fn set(&mut self, section: &str, key: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some(existing_section) = self
            .sections
            .iter_mut()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(section))
        {
            if let Some(entry) = existing_section
                .entries
                .iter_mut()
                .find(|entry| entry.key.eq_ignore_ascii_case(key))
            {
                entry.value = value;
            } else {
                existing_section.entries.push(IniEntry {
                    key: key.to_owned(),
                    value,
                });
            }
        } else {
            self.sections.push(IniSection {
                name: section.to_owned(),
                entries: vec![IniEntry {
                    key: key.to_owned(),
                    value,
                }],
            });
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut text = String::new();
        for section in &self.sections {
            text.push('[');
            text.push_str(&section.name);
            text.push(']');
            text.push_str(self.line_ending);
            for entry in &section.entries {
                text.push_str(&entry.key);
                text.push('=');
                text.push_str(&entry.value);
                text.push_str(self.line_ending);
            }
        }
        if !self.trailing_newline && text.ends_with(self.line_ending) {
            text.truncate(text.len() - self.line_ending.len());
        }
        Ok(encode_text(&text, self.encoding)?.into_owned())
    }
}
