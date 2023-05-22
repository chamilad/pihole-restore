use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, Result as SQLResult};
use serde_json::{Result as JSONResult, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use tar::Archive;

mod types;

fn connect_sqlite(db_file: &str) -> Result<Connection, Box<dyn Error>> {
    debug!("connecting to SQLite db: {}", db_file);
    let connection = Connection::open(db_file)?;
    Ok(connection)
}

fn load_domainlist(
    conn: Connection,
    contents: &str,
    domain_type: i32,
) -> Result<i32, Box<dyn Error>> {
    debug!("loading domainlist table");
    // json load contents
    let records: Vec<types::Domain> = serde_json::from_str(contents).unwrap();

    let sql = format!("INSERT OR IGNORE INTO domainlist (id,domain,enabled,date_added,comment,type) VALUES (:id,:domain,:enabled,:date_added,:comment,{});", domain_type);
    let mut stmt = conn.prepare(&sql)?;

    let record_count = records.len() as i32;
    debug!("starting to load {} records to domainlist", record_count);

    for record in records {
        stmt.execute_named(&[
            (":id", &record.id),
            (":domain", &record.domain),
            (":enabled", &record.enabled),
            (":date_added", &record.date_added),
            (":comment", &record.comment),
        ]);
    }

    Ok(record_count)

    // match records {
    //     Value::Array(arr) => {
    //         debug!("processing multiple entries for table {}", table);
    //         for entry in arr {
    //             match entry {
    //                 Value::Object(obj) => {}
    //                 _ => warn!("invalid json type found iterating contents for {}", table),
    //             }
    //         }
    //     }
    //     _ => debug!("processing a single entry for {}", table),
    // }
}

fn load_table(
    db_file: &str,
    table: &str,
    contents: &str,
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

    debug!("before match");
    let modified: i32;
    match table {
        "adlist" => {
            // let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
            debug!("not doing anything for adlist for now");
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
            modified = load_domainlist(conn, contents, domain_type)?;
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
        let mut file = file_result.unwrap();

        let file_name = file.header().path().unwrap();
        // println!("{}", file_name.to_string_lossy());

        if file_name.to_string_lossy() == "blacklist.exact.json" {
            let mut s = String::new();
            file.read_to_string(&mut s).unwrap();
            // println!("blacklist: {}", s);
            let result = load_table(sqlite_db_file, "blacklist", &s, true);
            match result {
                Ok(count) => {
                    debug!("loaded {} blacklist domains to domainlist", count);
                }
                Err(e) => {
                    warn!("error while loading blacklist domains: {}", e);
                }
            }
        } else {
            debug!("to be supported: {}", file_name.to_string_lossy());
        }
    }
}
