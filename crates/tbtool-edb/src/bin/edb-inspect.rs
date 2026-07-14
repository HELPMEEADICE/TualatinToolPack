use std::{env, fs, process::ExitCode};

use tbtool_edb::{Database, FileHeader, FileKind, Table, crypt_in_place};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: edb-inspect <path> [password]");
        return ExitCode::FAILURE;
    };
    let password = args
        .next()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "tulading123".to_owned());

    let mut bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = crypt_in_place(&mut bytes, password.as_bytes()) {
        eprintln!("failed to decrypt {}: {error}", path.to_string_lossy());
        return ExitCode::FAILURE;
    }

    println!("path: {}", path.to_string_lossy());
    println!("size: {} bytes", bytes.len());
    match FileHeader::detect(&bytes).map(|header| header.kind) {
        Ok(FileKind::Database) => match Database::from_decrypted_bytes(&bytes) {
            Ok(database) => {
                println!(
                    "database: {} records, {} fields, {} bytes/record, max id {}",
                    database.records().len(),
                    database.fields.len(),
                    database.header.record_size,
                    database.header.max_record_id
                );
                for (index, field) in database.fields.iter().enumerate() {
                    println!(
                        "field {index}: {:?}, type {}, offset {}, storage {}, aux {}",
                        field.name_gbk(),
                        field.field_type,
                        field.offset,
                        field.storage_size().unwrap_or(0),
                        field.size
                    );
                }
            }
            Err(error) => eprintln!("database parse failed: {error}"),
        },
        Ok(FileKind::Table) => match Table::from_decrypted_bytes(&bytes) {
            Ok(table) => {
                println!(
                    "table: {} pages, {} free, first free {}",
                    table.header.page_count,
                    table.header.free_page_count,
                    table.header.first_free_page
                );
                for (index, page) in table.pages.iter().enumerate() {
                    println!(
                        "page {}: previous {}, next {}, used {}",
                        index + 1,
                        page.previous,
                        page.next,
                        page.used
                    );
                }
            }
            Err(error) => eprintln!("table parse failed: {error}"),
        },
        Err(error) => eprintln!("header parse failed: {error}"),
    }
    for (row, chunk) in bytes[..bytes.len().min(4096)].chunks(16).enumerate() {
        if chunk.iter().all(|byte| *byte == 0) {
            continue;
        }
        print!("{:08x}  ", row * 16);
        for byte in chunk {
            print!("{byte:02x} ");
        }
        for _ in chunk.len()..16 {
            print!("   ");
        }
        print!(" |");
        for byte in chunk {
            let character = if byte.is_ascii_graphic() || *byte == b' ' {
                char::from(*byte)
            } else {
                '.'
            };
            print!("{character}");
        }
        println!("|");
    }

    ExitCode::SUCCESS
}
