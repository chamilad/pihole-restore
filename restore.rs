use crate::types::Restorable;
use clap::Parser;
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, Result as SQLResult};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use tar::Archive;

mod types;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    // teleporter archive to restore from
    #[arg(short = 'f', long = "file")]
    file: String,

    // gravity db file
    #[arg(short, long)]
    database: String,

    // clean existing tables
    #[arg(short = 'c', long = "clear", default_value_t = false)]
    flush: bool,
}

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
            debug!("processing adlist table");
            let records: Vec<types::Ad> = serde_json::from_str(&s).unwrap();
            let record_list: types::AdList = types::AdList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "domain_audit" => {
            debug!("processing domain_audit table");
            let records: Vec<types::DomainAuditEntry> = serde_json::from_str(&s).unwrap();
            let record_list: types::DomainAuditList = types::DomainAuditList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "group" => {
            debug!("processing group table");
            let records: Vec<types::Group> = serde_json::from_str(&s).unwrap();
            let record_list: types::GroupList = types::GroupList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "client" => {
            debug!("processing client table");
            let records: Vec<types::Client> = serde_json::from_str(&s).unwrap();
            let record_list: types::ClientList = types::ClientList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "client_by_group" => {
            debug!("processing client_by_group table");
            let records: Vec<types::ClientGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: types::ClientGroupAssignmentList =
                types::ClientGroupAssignmentList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "domainlist_by_group" => {
            debug!("processing domainlist_by_group table");
            let records: Vec<types::DomainListGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: types::DomainListGroupAssignmentList =
                types::DomainListGroupAssignmentList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "adlist_by_group" => {
            debug!("processing adlist_by_group table");
            let records: Vec<types::AdListGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: types::AdListGroupAssignmentList =
                types::AdListGroupAssignmentList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        _ => {
            debug!("processing unmatched table name: {}", table);
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
            let records: Vec<types::Domain> = serde_json::from_str(&s).unwrap();
            let record_list: types::DomainList = types::DomainList {
                list: records,
                domain_type,
            };
            modified = record_list.restore_table(conn)?;
        }
    }

    Ok(modified)
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let tar_gz_file = args.file;
    let sqlite_db_file = args.database;
    let flush_tables = args.flush;

    let file = match File::open(&tar_gz_file) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open {}: {}", &tar_gz_file, e);
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
            "blacklist.exact.json" => {
                let result = load_table(&sqlite_db_file, "blacklist", &mut tar_file, flush_tables);
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
                let result = load_table(
                    &sqlite_db_file,
                    "regex_blacklist",
                    &mut tar_file,
                    flush_tables,
                );
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
                let result = load_table(&sqlite_db_file, "whitelist", &mut tar_file, flush_tables);
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
                let result = load_table(
                    &sqlite_db_file,
                    "regex_whitelist",
                    &mut tar_file,
                    flush_tables,
                );
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
                let result = load_table(&sqlite_db_file, "adlist", &mut tar_file, flush_tables);
                match result {
                    Ok(count) => {
                        debug!("loaded {} adlist domains to adlist", count);
                    }
                    Err(e) => {
                        warn!("error while loading adlist domains: {}", e);
                    }
                }
            }
            "domain_audit.json" => {
                let result =
                    load_table(&sqlite_db_file, "domain_audit", &mut tar_file, flush_tables);
                match result {
                    Ok(count) => {
                        debug!("loaded {} domain audit entries to domain_audit", count);
                    }
                    Err(e) => {
                        warn!("error while loading audit domains: {}", e);
                    }
                }
            }
            "group.json" => {
                let result = load_table(&sqlite_db_file, "group", &mut tar_file, flush_tables);
                match result {
                    Ok(count) => {
                        debug!("loaded {} domain group entries to group", count);
                    }
                    Err(e) => {
                        warn!("error while loading groups: {}", e);
                    }
                }
            }
            "client.json" => {
                let result = load_table(&sqlite_db_file, "client", &mut tar_file, flush_tables);
                match result {
                    Ok(count) => {
                        debug!("loaded {} entries to client", count);
                    }
                    Err(e) => {
                        warn!("error while loading clients: {}", e);
                    }
                }
            }
            "client_by_group.json" => {
                let result = load_table(
                    &sqlite_db_file,
                    "client_by_group",
                    &mut tar_file,
                    flush_tables,
                );
                match result {
                    Ok(count) => {
                        debug!("loaded {} entries to client_by_group", count);
                    }
                    Err(e) => {
                        warn!("error while loading client_by_group: {}", e);
                    }
                }
            }
            "domainlist_by_group.json" => {
                let result = load_table(
                    &sqlite_db_file,
                    "domainlist_by_group",
                    &mut tar_file,
                    flush_tables,
                );
                match result {
                    Ok(count) => {
                        debug!("loaded {} entries to domainlist_by_group", count);
                    }
                    Err(e) => {
                        warn!("error while loading domainlist_by_group: {}", e);
                    }
                }
            }
            "adlist_by_group.json" => {
                let result = load_table(
                    &sqlite_db_file,
                    "adlist_by_group",
                    &mut tar_file,
                    flush_tables,
                );
                match result {
                    Ok(count) => {
                        debug!("loaded {} entries to adlist_by_group", count);
                    }
                    Err(e) => {
                        warn!("error while loading adlist_by_group: {}", e);
                    }
                }
            }
            "dnsmasq.d/04-pihole-static-dhcp.conf" => {}
            _ => info!("to be supported: {}", file_name),
        }
    }
}
