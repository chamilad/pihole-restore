use crate::gravity::Restorable;
use clap::Parser;
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use regex::Regex;
use rusqlite::{params, Connection, Result as SQLResult};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output};
use tar::Archive;

mod gravity;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    // teleporter archive to restore from
    #[arg(short = 'f', long = "file")]
    file: String,

    // gravity db file
    #[arg(short, long, default_value_t = String::from("/etc/pihole/gravity.db"))]
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

    // flush table if needed
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
            let records: Vec<gravity::Ad> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::AdList = gravity::AdList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "domain_audit" => {
            debug!("processing domain_audit table");
            let records: Vec<gravity::DomainAuditEntry> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::DomainAuditList = gravity::DomainAuditList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "group" => {
            debug!("processing group table");
            let records: Vec<gravity::Group> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::GroupList = gravity::GroupList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "client" => {
            debug!("processing client table");
            let records: Vec<gravity::Client> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::ClientList = gravity::ClientList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "client_by_group" => {
            debug!("processing client_by_group table");
            let records: Vec<gravity::ClientGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::ClientGroupAssignmentList =
                gravity::ClientGroupAssignmentList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "domainlist_by_group" => {
            debug!("processing domainlist_by_group table");
            let records: Vec<gravity::DomainListGroupAssignment> =
                serde_json::from_str(&s).unwrap();
            let record_list: gravity::DomainListGroupAssignmentList =
                gravity::DomainListGroupAssignmentList { list: records };
            modified = record_list.restore_table(conn)?;
        }
        "adlist_by_group" => {
            debug!("processing adlist_by_group table");
            let records: Vec<gravity::AdListGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::AdListGroupAssignmentList =
                gravity::AdListGroupAssignmentList { list: records };
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
            let records: Vec<gravity::Domain> = serde_json::from_str(&s).unwrap();
            let record_list: gravity::DomainList = gravity::DomainList {
                list: records,
                domain_type,
            };
            modified = record_list.restore_table(conn)?;
        }
    }

    Ok(modified)
}

fn process_static_dhcp(file: &mut tar::Entry<'_, GzDecoder<File>>) -> Result<i32, Box<dyn Error>> {
    // https://github.com/pi-hole/pi-hole/blob/d885e92674e8d8d9a673b35ae706b2c49ea05840/advanced/Scripts/webpage.sh#L537
    // this inserts different formats according to given input, so it's possible the backup could
    // contain static dhcp entries of these types
    // Pihole's Admin page Teleporer code is buggy when partial information is specified
    enum StaticDHCPType {
        Full,           // when all three are defined
        StaticIP,       // when no host name is defined
        StaticHostName, // when no IP address is defined
    }

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    for entry in s.lines() {
        debug!("processing static dhcp lease: {}", entry);
        // dhcp-host=<MAC_ADDR>,<IP>,<HOSTNAME>  or variants where ip or hostname is missing
        let sections: Vec<&str> = entry.split(",").collect();

        let mut mode: StaticDHCPType = StaticDHCPType::Full;
        if sections.len() == 3 {
            mode = StaticDHCPType::Full;
        } else if sections.len() == 2 {
            // check if the second part is a valid ip
            if is_valid_ip_addr(sections[1]) {
                mode = StaticDHCPType::StaticIP;
            } else {
                mode = StaticDHCPType::StaticHostName;
            }
        } else {
            warn!("invalid dhcp lease entry found: {}", entry);
            continue;
        }

        // extract MAC address from the first slice
        match get_mac_addr(sections[0]) {
            Some(addr) => {
                let dhcp_added = match mode {
                    StaticDHCPType::Full => {
                        add_static_dhcp_entry(addr.as_str(), sections[1], sections[2])?
                    }
                    StaticDHCPType::StaticIP => {
                        add_static_dhcp_entry(addr.as_str(), sections[1], "nohost")?
                    }
                    StaticDHCPType::StaticHostName => {
                        add_static_dhcp_entry(addr.as_str(), "noip", sections[1])?
                    }
                };

                if dhcp_added {
                    debug!("dhcp entry succesfully added: {}", entry);
                } else {
                    warn!("could not add the dhcp entry: {}", entry);
                }
            }
            None => warn!(
                "non-existent or invalid mac address found in the dhcp lease entry: {}",
                entry
            ),
        }
    }

    return Ok(0);
}

fn get_mac_addr(s: &str) -> Option<regex::Match> {
    let mac_pattern = r"([0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}";
    let mac_regex = Regex::new(mac_pattern).unwrap();

    // dhcp-host=<MAC_ADDR>
    mac_regex.find(s)
}

fn is_valid_ip_addr(s: &str) -> bool {
    let ipv4_pattern = r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";
    let ipv4_regex = Regex::new(ipv4_pattern).unwrap();

    let ipv6_pattern = r"^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$";
    let ipv6_regex = Regex::new(ipv6_pattern).unwrap();

    ipv4_regex.is_match(s) || ipv6_regex.is_match(s)
}

fn add_static_dhcp_entry(mac: &str, ip: &str, hostname: &str) -> Result<bool, Box<dyn Error>> {
    // todo: sanitisation

    // check for duplicates
    if Path::new("/etc/dnsmasq.d/04-pihole-static-dhcp.conf").exists() {
        let mut file = File::open("/etc/dnsmasq.d/04-pihole-static-dhcp.conf")?;
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();

        // assuming O(n+m) is enough here
        if s.contains(mac) {
            warn!(
                "mac address already exists in the static dhcp config: {}",
                mac
            );
            return Ok(false);
        }
    }

    // add the entry through the pihole cmd
    let add_cmd: Vec<&str> = vec!["-a", "addstaticdhcp", mac, ip, hostname];
    let exec_result = pihole_execute(add_cmd);
    match exec_result {
        Ok(_) => {
            debug!("static dhcp entry added successfully: {}", mac);
            Ok(true)
        }
        Err(e) => {
            warn!("error while adding static dhcp entry: {}, {}", mac, e);
            Err(Box::new(e))
        }
    }
}

fn pihole_execute(arguments: Vec<&str>) -> Result<Output, std::io::Error> {
    Command::new("pihole").args(arguments).output()
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
            "dnsmasq.d/04-pihole-static-dhcp.conf" => {
                // todo: do this later and pass flush_tables to add fn, avoid opening files unless needed
                if flush_tables {
                    match File::open("/etc/dnsmasq.d/04-pihole-static-dhcp.conf") {
                        Err(e) => warn!("error while opening static dhcp config to flush: {}", e),
                        Ok(file) => match file.set_len(0) {
                            Err(e) => {
                                warn!("error while truncating static dhcp config file: {}", e)
                            }
                            Ok(_) => debug!("static dhcp config truncated successfully"),
                        },
                    }
                }

                match process_static_dhcp(&mut tar_file) {
                    Err(e) => warn!("error while processing the static dhcp leases: {}", e),
                    Ok(count) => debug!("{} static dhcp leases successfully processed", count),
                }
            }
            _ => info!("to be supported: {}", file_name),
        }
    }
}
