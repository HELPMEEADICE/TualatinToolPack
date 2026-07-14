use std::{fs, path::PathBuf};

use tbtool_edb::{Database, FileHeader, Table, crypt_in_place};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate is inside workspace/crates")
        .to_path_buf()
}

#[test]
fn decrypts_all_current_edb_headers() {
    let root = workspace_root();
    let mut paths = Vec::new();
    for directory in ["data", "List"] {
        for entry in fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("edb"))
            {
                paths.push(path);
            }
        }
    }

    assert_eq!(paths.len(), 16, "the compatibility corpus changed");
    for path in paths {
        let original = fs::read(&path).unwrap();
        let header = FileHeader::detect(&original).unwrap();
        assert!(header.encrypted, "{} should be encrypted", path.display());

        let mut decoded = original.clone();
        crypt_in_place(&mut decoded, b"tulading123").unwrap();
        assert_eq!(&decoded[..4], b"WCDB", "{}", path.display());
        assert_eq!(&decoded[4..8], &[0, 0, 1, 0], "{}", path.display());

        crypt_in_place(&mut decoded, b"tulading123").unwrap();
        assert_eq!(decoded, original, "{}", path.display());
    }
}

#[test]
fn decrypts_all_current_edt_headers() {
    let root = workspace_root();
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("List")).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("edt"))
        {
            paths.push(path);
        }
    }

    assert_eq!(paths.len(), 11, "the compatibility corpus changed");
    for path in paths {
        let original = fs::read(&path).unwrap();
        let mut decoded = original.clone();
        crypt_in_place(&mut decoded, b"tulading123").unwrap();
        assert_eq!(&decoded[..4], b"WEDT", "{}", path.display());
        assert_eq!(&decoded[4..8], &[0, 0, 1, 0], "{}", path.display());

        crypt_in_place(&mut decoded, b"tulading123").unwrap();
        assert_eq!(decoded, original, "{}", path.display());
    }
}

#[test]
fn parses_and_recreates_all_encrypted_databases() {
    let root = workspace_root();
    let mut paths = Vec::new();
    for directory in ["data", "List"] {
        for entry in fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("edb"))
            {
                paths.push(path);
            }
        }
    }

    assert_eq!(paths.len(), 16, "the compatibility corpus changed");
    for path in paths {
        let original = fs::read(&path).unwrap();
        let database = Database::open(original.clone(), Some(b"tulading123"))
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            database.header.record_count as usize,
            database.records().len(),
            "{}",
            path.display()
        );
        assert_eq!(
            database.to_bytes(Some(b"tulading123")).unwrap(),
            original,
            "{}",
            path.display()
        );
    }
}

#[test]
fn parses_and_recreates_all_encrypted_tables() {
    let root = workspace_root();
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("List")).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("edt"))
        {
            paths.push(path);
        }
    }

    assert_eq!(paths.len(), 11, "the compatibility corpus changed");
    for path in paths {
        let original = fs::read(&path).unwrap();
        let table = Table::open(original.clone(), Some(b"tulading123"))
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            table.to_bytes(Some(b"tulading123")).unwrap(),
            original,
            "{}",
            path.display()
        );
    }
}

#[test]
fn decodes_gbk_schema_and_reads_real_blob_chain() {
    let root = workspace_root();
    let database = Database::open(
        fs::read(root.join("data/Graphicscard.edb")).unwrap(),
        Some(b"tulading123"),
    )
    .unwrap();
    assert_eq!(database.fields[0].name_gbk(), "标识");
    assert_eq!(database.fields[1].name_gbk(), "翻译");
    assert_eq!(database.field_text_gbk(0, 0).unwrap(), "7377");
    assert_eq!(database.field_text_gbk(0, 1).unwrap(), "七彩虹");

    let table = Table::open(
        fs::read(root.join("List/CPU工具.EDT")).unwrap(),
        Some(b"tulading123"),
    )
    .unwrap();
    let png = table.read_chain(1).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(png.ends_with(b"IEND\xaeB`\x82"));
    assert_eq!(table.free_pages().unwrap().len(), 44);
}

#[test]
fn edits_gbk_text_and_reopens_encrypted_database() {
    let root = workspace_root();
    let mut database = Database::open(
        fs::read(root.join("data/Graphicscard.edb")).unwrap(),
        Some(b"tulading123"),
    )
    .unwrap();
    database.set_field_text_gbk(0, 1, "厂商").unwrap();

    let encoded = database.to_bytes(Some(b"new-password")).unwrap();
    let reopened = Database::open(encoded, Some(b"new-password")).unwrap();
    assert_eq!(reopened.field_text_gbk(0, 1).unwrap(), "厂商");
}

#[test]
fn grows_and_shrinks_real_table_chain_using_free_pages() {
    let root = workspace_root();
    let mut table = Table::open(
        fs::read(root.join("List/CPU工具.EDT")).unwrap(),
        Some(b"tulading123"),
    )
    .unwrap();
    let untouched = table.read_chain(7).unwrap();
    let original_page_count = table.header.page_count;
    let original_free_count = table.header.free_page_count;

    let grown = vec![0x5a; 4_000];
    assert_eq!(table.replace_chain(1, &grown).unwrap(), 1);
    assert_eq!(table.read_chain(1).unwrap(), grown);
    assert_eq!(table.header.page_count, original_page_count);
    assert_eq!(table.header.free_page_count, original_free_count - 2);
    assert_eq!(table.read_chain(7).unwrap(), untouched);

    let shrunk = vec![0xa5; 100];
    assert_eq!(table.replace_chain(1, &shrunk).unwrap(), 1);
    assert_eq!(table.read_chain(1).unwrap(), shrunk);
    assert_eq!(table.header.free_page_count, original_free_count + 5);
    assert_eq!(
        table.free_pages().unwrap().len(),
        (original_free_count + 5) as usize
    );

    let encoded = table.to_bytes(Some(b"new-password")).unwrap();
    let reopened = Table::open(encoded, Some(b"new-password")).unwrap();
    assert_eq!(reopened.read_chain(1).unwrap(), shrunk);
    assert_eq!(reopened.read_chain(7).unwrap(), untouched);
    assert_eq!(reopened.free_pages().unwrap(), table.free_pages().unwrap());
}
