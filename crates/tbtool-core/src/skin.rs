use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek},
    path::{Component, Path},
};

use encoding_rs::GBK;
use zip::ZipArchive;

use crate::{Error, IniDocument, Result};

const MAX_ASSET_SIZE: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Bmp,
}

impl ImageFormat {
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(b"BM") {
            Some(Self::Bmp)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinManifest {
    pub opacity: Option<u8>,
    pub name: Option<String>,
    pub author: Option<String>,
    pub border_color: Option<u32>,
    pub text_color: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinPackage {
    pub manifest: SkinManifest,
    pub config: IniDocument,
    pub assets: BTreeMap<String, Vec<u8>>,
}

impl SkinPackage {
    pub fn from_zip<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)?;
        let mut files = BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index)?;
            if file.is_dir() {
                continue;
            }
            if file.size() > MAX_ASSET_SIZE {
                return Err(Error::AssetTooLarge);
            }
            let name = decode_zip_name(file.name_raw());
            validate_relative_path(&name)?;
            let mut data = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut data)?;
            files.insert(name.replace('\\', "/"), data);
        }

        let config_name = files
            .keys()
            .find(|name| name.eq_ignore_ascii_case("Config.ini"))
            .cloned()
            .ok_or(Error::MissingSkinAsset("Config.ini"))?;
        let config = IniDocument::parse(&files.remove(&config_name).expect("key came from map"))?;
        Self::from_parts(config, files)
    }

    pub fn from_directory(directory: &Path) -> Result<Self> {
        let config = IniDocument::parse(&fs::read(directory.join("Config.ini"))?)?;
        let mut assets = BTreeMap::new();
        for name in [
            "默认底图.png",
            "运行时间.png",
            "系统信息.png",
            "硬件信息底图.png",
            "控制按钮.png",
            "型号信息.png",
            "列表按钮.png",
        ] {
            assets.insert(name.to_owned(), fs::read(directory.join(name))?);
        }
        Self::from_parts(config, assets)
    }

    pub fn from_parts(config: IniDocument, assets: BTreeMap<String, Vec<u8>>) -> Result<Self> {
        for required in [
            "默认底图.png",
            "运行时间.png",
            "系统信息.png",
            "硬件信息底图.png",
            "控制按钮.png",
            "型号信息.png",
            "列表按钮.png",
        ] {
            if !assets.contains_key(required) {
                return Err(Error::MissingSkinAsset(required));
            }
        }
        let manifest = SkinManifest {
            opacity: config
                .get("皮肤", "透明度")
                .and_then(|value| value.parse().ok()),
            name: config.get("皮肤", "皮肤名").map(str::to_owned),
            author: config.get("皮肤", "作者").map(str::to_owned),
            border_color: config
                .get("皮肤", "边框色")
                .and_then(|value| value.parse().ok()),
            text_color: config
                .get("皮肤", "文本色")
                .and_then(|value| value.parse().ok()),
        };
        Ok(Self {
            manifest,
            config,
            assets,
        })
    }

    pub fn asset(&self, name: &str) -> Option<&[u8]> {
        self.assets.get(name).map(Vec::as_slice)
    }

    pub fn image(&self, name: &str) -> Option<(&[u8], ImageFormat)> {
        let bytes = self.asset(name)?;
        Some((bytes, ImageFormat::detect(bytes)?))
    }
}

fn decode_zip_name(raw: &[u8]) -> String {
    if let Ok(name) = std::str::from_utf8(raw) {
        return name.to_owned();
    }
    let (name, _, _) = GBK.decode(raw);
    name.into_owned()
}

fn validate_relative_path(name: &str) -> Result<()> {
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::UnsafeArchivePath(name.to_owned()));
    }
    Ok(())
}
