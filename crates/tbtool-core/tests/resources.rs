use std::{fs, io::Cursor, path::PathBuf};

use tbtool_core::{
    ExecutableArchitecture, ImageFormat, IniDocument, SkinPackage, TextEncoding, ToolCatalog,
    ToolLauncher, ToolTarget,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate is inside workspace/crates")
        .to_path_buf()
}

#[test]
fn root_config_decodes_to_unicode_and_round_trips() {
    let bytes = fs::read(workspace_root().join("Config.ini")).unwrap();
    let config = IniDocument::parse(&bytes).unwrap();
    assert_eq!(config.encoding, TextEncoding::Gbk);
    assert_eq!(config.get("设置", "静默检测硬件信息"), Some("假"));
    assert_eq!(config.get("皮肤", "文件名"), Some("经典蓝.skin"));
    assert_eq!(config.to_bytes().unwrap(), bytes);
}

#[test]
fn loads_all_legacy_and_utf8_skin_archives() {
    let skin_dir = workspace_root().join("skin");
    let mut count = 0;
    for entry in fs::read_dir(skin_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "skin") {
            continue;
        }
        count += 1;
        let bytes = fs::read(&path).unwrap();
        let package = SkinPackage::from_zip(Cursor::new(bytes))
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(package.assets.len(), 7, "{}", path.display());
        assert!(
            package
                .image("默认底图.png")
                .is_some_and(|(_, format)| matches!(format, ImageFormat::Png | ImageFormat::Bmp)),
            "{}",
            path.display()
        );
        assert!(package.manifest.name.is_some(), "{}", path.display());
    }
    assert_eq!(count, 8, "the skin compatibility corpus changed");

    let user = SkinPackage::from_directory(&workspace_root().join("skin/user")).unwrap();
    assert_eq!(user.assets.len(), 7);
    assert_eq!(user.manifest.name.as_deref(), Some("经典蓝"));
}

#[test]
fn loads_all_tool_categories_with_unicode_paths_and_icons() {
    let root = workspace_root();
    let catalog = ToolCatalog::load(&root, b"tulading123").unwrap();
    assert_eq!(catalog.categories.len(), 11);
    assert!(catalog.tool_count() >= 100);
    for category in &catalog.categories {
        assert!(!category.name.is_empty());
        for tool in &category.tools {
            assert!(!tool.name.is_empty(), "{}", category.name);
            match &tool.target {
                ToolTarget::Executable {
                    path,
                    working_directory,
                } => {
                    assert!(path.starts_with("tools"));
                    assert!(working_directory.starts_with("tools"));
                }
                ToolTarget::BuiltIn { name } => assert_eq!(name, &tool.executable_text),
            }
            assert!(!tool.icon.is_empty(), "{} / {}", category.name, tool.name);
        }
    }
}

#[test]
fn launcher_rejects_builtins_and_resolves_packaged_executables() {
    let root = workspace_root();
    let catalog = ToolCatalog::load(&root, b"tulading123").unwrap();
    let launcher = ToolLauncher::new(&root).unwrap();
    let mut executable_count = 0;
    let mut unavailable = Vec::new();
    let mut pe_count = 0;
    let mut script_count = 0;
    let mut association_count = 0;
    for category in &catalog.categories {
        for tool in &category.tools {
            match &tool.target {
                ToolTarget::Executable { .. } => {
                    executable_count += 1;
                    match launcher.plan(&tool.target) {
                        Ok(plan)
                            if matches!(
                                plan.architecture,
                                ExecutableArchitecture::X86
                                    | ExecutableArchitecture::X64
                                    | ExecutableArchitecture::Arm
                                    | ExecutableArchitecture::Arm64
                            ) =>
                        {
                            pe_count += 1;
                        }
                        Ok(plan) if plan.architecture == ExecutableArchitecture::Script => {
                            script_count += 1;
                        }
                        Ok(plan) if plan.architecture == ExecutableArchitecture::Unknown => {
                            assert_eq!(
                                plan.executable.extension().and_then(|value| value.to_str()),
                                Some("jpg")
                            );
                            association_count += 1;
                        }
                        Ok(_) => unreachable!(),
                        Err(error) => unavailable.push((tool.name.as_str(), error.to_string())),
                    }
                }
                ToolTarget::BuiltIn { .. } => {
                    assert!(launcher.plan(&tool.target).is_err());
                }
            }
        }
    }
    assert!(executable_count >= 100);
    assert!(
        pe_count >= 85,
        "only {pe_count} packaged targets are PE files"
    );
    assert_eq!(script_count, 32);
    assert_eq!(association_count, 2);
    assert_eq!(unavailable.len(), 2, "{unavailable:#?}");
    assert_eq!(unavailable[0].0, "Dism++  ARM64");
    assert!(unavailable[0].1.contains("cannot run a ARM64 tool"));
    assert_eq!(unavailable[1].0, "ZenTimings");
    assert!(unavailable[1].1.contains("ZenTimings.exe"));
}
