use crate::pihole::dhcp;
use crate::pihole::dns;
use crate::pihole::gravity;
use clap::Parser;
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use std::fs::File;
use tar::Archive;

mod pihole;

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
                let result = gravity::restore_domainlist(
                    &sqlite_db_file,
                    gravity::DomainType::Blacklist,
                    &mut tar_file,
                    flush_tables,
                );
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
                let result = gravity::restore_domainlist(
                    &sqlite_db_file,
                    gravity::DomainType::BlacklistRegex,
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
                let result = gravity::restore_domainlist(
                    &sqlite_db_file,
                    gravity::DomainType::Whitelist,
                    &mut tar_file,
                    flush_tables,
                );
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
                let result = gravity::restore_domainlist(
                    &sqlite_db_file,
                    gravity::DomainType::WhitelistRegex,
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
                let result =
                    gravity::load_table(&sqlite_db_file, "adlist", &mut tar_file, flush_tables);
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
                let result = gravity::load_table(
                    &sqlite_db_file,
                    "domain_audit",
                    &mut tar_file,
                    flush_tables,
                );
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
                let result =
                    gravity::load_table(&sqlite_db_file, "group", &mut tar_file, flush_tables);
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
                let result =
                    gravity::load_table(&sqlite_db_file, "client", &mut tar_file, flush_tables);
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
                let result = gravity::load_table(
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
                let result = gravity::load_table(
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
                let result = gravity::load_table(
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
                match dhcp::process_static_dhcp(&mut tar_file, flush_tables) {
                    Err(e) => warn!("error while processing the static dhcp leases: {}", e),
                    Ok(count) => debug!("{} static dhcp leases successfully processed", count),
                }
            }
            "custom.list" => match dns::process_local_dns_entries(&mut tar_file, flush_tables) {
                Err(e) => warn!("error while processing custom.list restore: {}", e),
                Ok(count) => debug!("{} local dns entries processed", count),
            },
            "dnsmasq.d/05-pihole-custom-cname.conf" => {
                match dns::process_local_cname_entries(&mut tar_file, flush_tables) {
                    Err(e) => warn!("error while processing custom cname restore: {}", e),
                    Ok(count) => debug!("{} local cname entries processed", count),
                }
            }

            _ => info!("to be supported: {}", file_name),
        }
    }
}
