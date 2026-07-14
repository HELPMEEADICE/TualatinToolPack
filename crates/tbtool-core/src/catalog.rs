use std::{
    fs,
    path::{Path, PathBuf},
};

use tbtool_edb::{Database, Table};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolEntry {
    pub id: u32,
    pub name: String,
    pub executable_text: String,
    pub working_directory_text: String,
    pub target: ToolTarget,
    pub description: String,
    pub icon: Vec<u8>,
    pub icon_40: Vec<u8>,
    pub icon_48: Vec<u8>,
    pub normalized_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolTarget {
    Executable {
        path: PathBuf,
        working_directory: PathBuf,
    },
    BuiltIn {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCategory {
    pub name: String,
    pub tools: Vec<ToolEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCatalog {
    pub categories: Vec<ToolCategory>,
}

impl ToolCatalog {
    pub fn load(root: &Path, password: &[u8]) -> Result<Self> {
        let list_dir = root.join("List");
        let mut database_paths = Vec::new();
        for entry in fs::read_dir(&list_dir)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("edb"))
            {
                database_paths.push(path);
            }
        }
        database_paths.sort();

        let mut categories = Vec::with_capacity(database_paths.len());
        for database_path in database_paths {
            let name = database_path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| Error::InvalidToolPath(database_path.display().to_string()))?
                .to_owned();
            let table_path = database_path.with_extension("EDT");
            let table_path = if table_path.exists() {
                table_path
            } else {
                database_path.with_extension("edt")
            };
            let database = fs::read(&database_path)?;
            let table = fs::read(table_path)?;
            categories.push(ToolCategory::from_bytes(name, database, table, password)?);
        }
        Ok(Self { categories })
    }

    pub fn tool_count(&self) -> usize {
        self.categories
            .iter()
            .map(|category| category.tools.len())
            .sum()
    }
}

impl ToolCategory {
    pub fn from_bytes(
        name: String,
        database_bytes: Vec<u8>,
        table_bytes: Vec<u8>,
        password: &[u8],
    ) -> Result<Self> {
        let database = Database::open(database_bytes, Some(password))?;
        let table = Table::open(table_bytes, Some(password))?;
        let name_field = required_field(&database, "工具名")?;
        let executable_field = required_field(&database, "路径")?;
        let directory_field = required_field(&database, "目录")?;
        let description_field = required_field(&database, "说明")?;
        let icon_field = required_field(&database, "图标")?;
        let icon_40_field = required_field(&database, "图标40")?;
        let icon_48_field = required_field(&database, "图标48")?;
        let normalized_name_field = required_field(&database, "处理后的工具名")?;

        let mut tools = Vec::with_capacity(database.records().len());
        for record_index in 0..database.records().len() {
            let executable_text = database.field_text_gbk(record_index, executable_field)?;
            let working_directory_text = database.field_text_gbk(record_index, directory_field)?;
            let target = if working_directory_text == "内置工具" {
                ToolTarget::BuiltIn {
                    name: executable_text.clone(),
                }
            } else {
                let path = normalize_packaged_path(&executable_text)?;
                let working_directory = normalize_packaged_path(&working_directory_text)?;
                if !path.starts_with("tools") || !working_directory.starts_with("tools") {
                    return Err(Error::InvalidToolPath(executable_text));
                }
                ToolTarget::Executable {
                    path,
                    working_directory,
                }
            };
            tools.push(ToolEntry {
                id: database.record_id(record_index)?,
                name: database.field_text_gbk(record_index, name_field)?,
                executable_text,
                working_directory_text,
                target,
                description: database.field_text_gbk(record_index, description_field)?,
                icon: table.read_chain(read_page_reference(
                    database.field_bytes(record_index, icon_field)?,
                )?)?,
                icon_40: table.read_chain(read_page_reference(
                    database.field_bytes(record_index, icon_40_field)?,
                )?)?,
                icon_48: table.read_chain(read_page_reference(
                    database.field_bytes(record_index, icon_48_field)?,
                )?)?,
                normalized_name: database.field_text_gbk(record_index, normalized_name_field)?,
            });
        }
        Ok(Self { name, tools })
    }
}

pub fn normalize_packaged_path(value: &str) -> Result<PathBuf> {
    let normalized = value.trim().replace('/', "\\");
    let components: Vec<&str> = normalized
        .split('\\')
        .filter(|component| !component.is_empty())
        .collect();
    let start = components
        .iter()
        .position(|component| component.eq_ignore_ascii_case("tools"))
        .unwrap_or(0);
    let components = &components[start..];
    if components.is_empty()
        || components.iter().any(|component| {
            *component == "."
                || *component == ".."
                || component.contains(':')
                || component.contains('\0')
        })
    {
        return Err(Error::InvalidToolPath(value.to_owned()));
    }
    let mut path = PathBuf::new();
    for component in components {
        path.push(component);
    }
    Ok(path)
}

fn required_field(database: &Database, name: &'static str) -> Result<usize> {
    database
        .field_index(name)
        .ok_or(Error::MissingToolField(name))
}

fn read_page_reference(bytes: &[u8]) -> Result<u32> {
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| tbtool_edb::Error::InvalidLayout("EDT page reference is not four bytes"))?;
    Ok(u32::from_le_bytes(bytes))
}
