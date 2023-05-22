use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use rusqlite::{Connection, Result as SQLResult};
use serde_json::{Result as JSONResult, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use tar::Archive;

fn connect_sqlite(db_file: &str) -> Result<Connection, Box<dyn Error>> {
    let connection = Connection::open(db_file)?;
    Ok(connection)
}

fn load_domainlist(conn: Connection, contents: &str, domain_type: i32) -> Result<i32, Box<dyn Error>> {
    // json load contents
    let records: Value = serde_json::from_str(contents)?;
    let mut sql = format!("INSERT OR IGNORE INTO domainlist (id,domain,enabled,date_added,comment,type) VALUES (:id,:domain,:enabled,:date_added,:comment,{});", domain_type)
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
        debug!("flushing table {}", table);
        let clear_sql = format!("DELETE FROM TABLE \"{}\"", table);
        conn.execute(&clear_sql, [])?;
    }

    match table {
        "adlist" => {
            let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
            debug!("not doing anything for adlist for now");
        },
        _ => {
            let domain_type: i32 = match table {
                "whitelist" => 0,
                "blacklist" => 1,
                "regex_whitelist" => 2,
                "regex_blacklist" => 3,
            };
            load_domainlist(conn, contents, domain_type)?;
        }
    }

    let sql: String = match table {
        "adlist" => 
        _ => {
        } 
    };

    let mut stmt = conn.prepare(&sql)?;

    match records {
        Value::Array(arr) => {
            debug!("processing multiple entries for table {}", table);
            for entry in arr {
                match entry {
                    Value::Object(obj) => {
                        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
                        for (key, value) in obj.iter() {
                            params.push((format!(":{}", key).as_str(), &value));
                        }
                    }
                    _ => warn!("invalid json type found iterating contents for {}", table),
                }
            }
        },
        _ => debug!("processing a single entry for {}", table),
    }
    // build table specific query
}

fn insert_into_table(table: &str, contents: &str, flush_table: bool) {}

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
        println!("{}", file_name.to_string_lossy());

        if file_name.to_string_lossy() == "blacklist.exact.json" {
            let mut s = String::new();
            file.read_to_string(&mut s).unwrap();
            println!("blacklist: {}", s);
            load_table(sqlite_db_file, "blacklist", &s, true);
        }

        //     Ok(f) => {
        //         let file_name = match file.header().path() {
        //             Ok(path) => match path.file_name() {
        //                 Some(file_name) => file_name.to_owned(),
        //                 None => {
        //                     println!("Failed to ge the file name")
        //                     continue;
        //                 }
        //             },
        //             Err(e) => {
        //                 println!("Failed to get file path: {}", e);
        //                 continue;
        //             }
        //         };

        //         let mut file_content = Vec::new();
        //         if let Err(e) = file.read_to_end(&mut file_content) {
        //             println!("Failed to read file content: {}", e);
        //             continue;
        //         }

        //         file_name
        //     }
        //     Err(e) => {
        //         println!("Failed to read tar.gz entry: {}", e);
        //         continue;
        //     }
        // };

        // let table_name = match file_name.file_stem() {
        //     Some(stem) => stem.to_str().unwrap(),
        //     None => {
        //         println!("Failed to get file name stem");
        //         continue;
        //     }
        // };

        // let table_creation_query =
        //     format!("CREATE TABLE IF NOT EXISTS {} (content BLOB);", table_name);
        // if let Err(e) = connection.execute(&table_creation_query, NO_PARAMS) {
        //     println!("Failed to create table {}: {}", table_name, e);
        //     continue;
        // }

        // let insert_query = format!("INSERT INTO {} (content) VALUES (?)", table_name);
        // if let Err(e) = connection.execute(&insert_query, [file_content]) {
        //     println!("Failed to insert content into table {}: {}", table_name, e);
        //     continue;
        // }

        // println!(
        //     "Inserted file {} into table {}",
        //     file_name.display(),
        //     table_name
        // );
    }
}
