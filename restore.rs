use crate::types::Restorable;
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, Result as SQLResult};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use tar::Archive;

mod types;

fn connect_sqlite(db_file: &str) -> Result<Connection, Box<dyn Error>> {
    debug!("connecting to SQLite db: {}", db_file);
    let connection = Connection::open(db_file)?;
    Ok(connection)
}

fn load_table(
    db_file: &str,
    table: &str,
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush_table: bool,
) -> Result<i32, Box<dyn Error>> {
    let conn: Connection = connect_sqlite(db_file)?;

    // flush table if neededA
    if flush_table == true {
        let table_exists_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
        let mut table_entry_stmt = conn.prepare(&table_exists_sql)?;
        let mut table_entry = table_entry_stmt.query(params![table])?;
        if let Some(_) = table_entry.next()? {
            debug!("flushing table {}", table);
            let clear_sql = format!("DELETE FROM \"{}\"", table);
            conn.execute(&clear_sql, [])?;
        } else {
            debug!("cannot flush table since it doesn't exist: {}", table);
        }
    }

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    debug!("before match");
    let modified: i32;
    match table {
        "adlist" => {
            let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
            debug!("processing adlist table");
            // load_adlist(conn, &s);
            modified = 0;
        }
        _ => {
            debug!("not adlist");
            let domain_type: i32 = match table {
                "whitelist" => 0,
                "blacklist" => 1,
                "regex_whitelist" => 2,
                "regex_blacklist" => 3,
                _ => {
                    warn!("invalid table sent for domain: {}", table);
                    -1
                }
            };

            debug!("loading contents to domainlist table");
            debug!("{}", &s);
            let records: Vec<types::Domain> = serde_json::from_str(&s).unwrap();
            let record_list: types::DomainList = types::DomainList { list: records };
            modified = record_list.restore_table(conn, domain_type)?;
        }
    }

    Ok(modified)
}

fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run -- <tar_gz_file> <sqlite_db_file>");
        return;
    }
    let tar_gz_file = &args[1];
    let sqlite_db_file = &args[2];

    let file = match File::open(tar_gz_file) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open {}: {}", tar_gz_file, e);
            return;
        }
    };

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    for file_result in archive.entries().expect("Failed to read tar.gz entries") {
        let mut tar_file = file_result.unwrap();

        let file_path = tar_file.path().unwrap();
        let file_name = file_path.to_str().unwrap();

        match file_name {
            "blacklist.txt" => {}
            "blacklist.exact.json" => {
                let result = load_table(sqlite_db_file, "blacklist", &mut tar_file, true);
                match result {
                    Ok(count) => {
                        debug!("loaded {} blacklist domains to domainlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading blacklist domains: {}", e);
                    }
                }
            }
            "blacklist.regex.json" => {
                let result = load_table(sqlite_db_file, "regex_blacklist", &mut tar_file, true);
                match result {
                    Ok(count) => {
                        debug!("loaded {} regex_blacklist domains to domainlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading regex_blacklist domains: {}", e);
                    }
                }
            }
            "whitelist.exact.json" => {
                let result = load_table(sqlite_db_file, "whitelist", &mut tar_file, true);
                match result {
                    Ok(count) => {
                        debug!("loaded {} whitelist domains to domainlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading whitelist domains: {}", e);
                    }
                }
            }
            "whitelist.regex.json" => {
                let result = load_table(sqlite_db_file, "regex_whitelist", &mut tar_file, true);
                match result {
                    Ok(count) => {
                        debug!("loaded {} regex_whitelist domains to domainlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading regex_whitelist domains: {}", e);
                    }
                }
            }
            "adlist.json" => {
                let result = load_table(sqlite_db_file, "adlist", &mut tar_file, true);
                match result {
                    Ok(count) => {
                        debug!("loaded {} adlist domains to domainlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading adlist domains: {}", e);
                    }
                }
            }
            _ => debug!("to be supported: {}", file_name),
        }
    }
}
