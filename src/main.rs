use crate::pihole::dhcp;
use crate::pihole::dns;
use crate::pihole::gravity;
use clap::Parser;
use env_logger::Env;
use flate2::read::GzDecoder;
use log::{debug, error, info, warn};
use std::fs::File;
use tar::Archive;

mod pihole;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// teleporter archive file to restore from
    #[arg(short = 'f', long = "file")]
    file: String,

    /// gravity db file location
    #[arg(short, long, default_value = "/etc/pihole/gravity.db")]
    database: String,

    /// clean existing tables and files
    #[arg(short = 'c', long = "clear", default_value_t = false)]
    flush: bool,

    /// filter which config to restore, specify in comma separated keywords
    #[arg(long = "filters", default_value = "all")]
    filters: String,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let tar_gz_file = args.file;
    let sqlite_db_file = args.database;
    let flush_tables = args.flush;

    let all_filters = vec![
        "blacklist".to_string(),
        "blacklistregex".to_string(),
        "whitelist".to_string(),
        "whitelistregex".to_string(),
        "adlist".to_string(),
        "auditlog".to_string(),
        "group".to_string(),
        "client".to_string(),
        "staticdhcp".to_string(),
        "localdns".to_string(),
        "localcname".to_string(),
    ];

    let filters;
    if args.filters == "all" {
        filters = all_filters.clone();
    } else {
        filters = args
            .filters
            .split(",")
            .map(String::from)
            .map(|f| f.to_lowercase())
            .collect();

        let check_filters = filters.clone();
        for f in check_filters {
            if !all_filters.contains(&f) {
                error!("invalid filter found: {}", f);
                std::process::exit(1);
            }
        }
    }

    info!("start importing...");
    let file = match File::open(&tar_gz_file) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open {}: {}", &tar_gz_file, e);
            std::process::exit(1);
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
                if filters.contains(&String::from("blacklist")) {
                    let result = gravity::restore_domainlist(
                        &sqlite_db_file,
                        gravity::DomainType::Blacklist,
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed blacklist (exact) ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while processing blacklist domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify blacklist",
                        file_name
                    );
                }
            }
            "blacklist.regex.json" => {
                if filters.contains(&String::from("blacklistregex")) {
                    let result = gravity::restore_domainlist(
                        &sqlite_db_file,
                        gravity::DomainType::BlacklistRegex,
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed blacklist (regex) ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading regex_blacklist domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify blacklistregex",
                        file_name
                    );
                }
            }
            "whitelist.exact.json" => {
                if filters.contains(&String::from("whitelist")) {
                    let result = gravity::restore_domainlist(
                        &sqlite_db_file,
                        gravity::DomainType::Whitelist,
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed whitelist (exact) ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading whitelist domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify whitelist",
                        file_name
                    );
                }
            }
            "whitelist.regex.json" => {
                if filters.contains(&String::from("whitelistregex")) {
                    let result = gravity::restore_domainlist(
                        &sqlite_db_file,
                        gravity::DomainType::WhitelistRegex,
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed whitelist (regex) ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading regex_whitelist domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify whitelistregex",
                        file_name
                    );
                }
            }
            "adlist.json" => {
                if filters.contains(&String::from("adlist")) {
                    let result =
                        gravity::load_table(&sqlite_db_file, "adlist", &mut tar_file, flush_tables);
                    match result {
                        Ok(count) => {
                            info!("processed adlist ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading adlist domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify adlist",
                        file_name
                    );
                }
            }
            "domain_audit.json" => {
                if filters.contains(&String::from("auditlog")) {
                    let result = gravity::load_table(
                        &sqlite_db_file,
                        "domain_audit",
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed domain_audit ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading audit domains: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify auditlog",
                        file_name
                    );
                }
            }
            "group.json" => {
                if filters.contains(&String::from("group")) {
                    let result =
                        gravity::load_table(&sqlite_db_file, "group", &mut tar_file, flush_tables);
                    match result {
                        Ok(count) => {
                            info!("processed group ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading groups: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify group",
                        file_name
                    );
                }
            }
            "client.json" => {
                if filters.contains(&String::from("client")) {
                    let result =
                        gravity::load_table(&sqlite_db_file, "client", &mut tar_file, flush_tables);
                    match result {
                        Ok(count) => {
                            info!("processed client ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading clients: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify client",
                        file_name
                    );
                }
            }
            "client_by_group.json" => {
                if filters.contains(&String::from("client")) {
                    let result = gravity::load_table(
                        &sqlite_db_file,
                        "client_by_group",
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed client group assignments ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading client_by_group: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify client",
                        file_name
                    );
                }
            }
            "domainlist_by_group.json" => {
                if filters.contains(&String::from("blacklist"))
                    || filters.contains(&String::from("blacklistregex"))
                    || filters.contains(&String::from("whitelist"))
                    || filters.contains(&String::from("whitelistregex"))
                {
                    let result = gravity::load_table(
                        &sqlite_db_file,
                        "domainlist_by_group",
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!(
                                "processed black-/whitelist group assginments ({} entries)",
                                count
                            );
                        }
                        Err(e) => {
                            warn!("error while loading domainlist_by_group: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify either blacklist, blacklistregex, whitelist, or whitelistregex",
                        file_name
                    );
                }
            }
            "adlist_by_group.json" => {
                if filters.contains(&String::from("adlist")) {
                    let result = gravity::load_table(
                        &sqlite_db_file,
                        "adlist_by_group",
                        &mut tar_file,
                        flush_tables,
                    );
                    match result {
                        Ok(count) => {
                            info!("processed adlist group assginments ({} entries)", count);
                        }
                        Err(e) => {
                            warn!("error while loading adlist_by_group: {}", e);
                        }
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify adlist",
                        file_name
                    );
                }
            }
            "dnsmasq.d/04-pihole-static-dhcp.conf" => {
                if filters.contains(&String::from("staticdhcp")) {
                    match dhcp::process_static_dhcp(&mut tar_file, flush_tables) {
                        Err(e) => warn!("error while processing the static dhcp leases: {}", e),
                        Ok(count) => info!("processed static dhcp leases ({} entries)", count),
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify staticdhcp",
                        file_name
                    );
                }
            }
            "custom.list" => {
                if filters.contains(&String::from("localdns")) {
                    match dns::process_local_dns_entries(&mut tar_file, flush_tables) {
                        Err(e) => warn!("error while processing custom.list restore: {}", e),
                        Ok(count) => info!("processed local DNS records ({} entries)", count),
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify localdns",
                        file_name
                    );
                }
            }
            "dnsmasq.d/05-pihole-custom-cname.conf" => {
                if filters.contains(&String::from("localcname")) {
                    match dns::process_local_cname_entries(&mut tar_file, flush_tables) {
                        Err(e) => warn!("error while processing custom cname restore: {}", e),
                        Ok(count) => info!("processed local CNAME records ({} entries)", count),
                    }
                } else {
                    info!(
                        "not processing {} because enforced filter does not specify localcname",
                        file_name
                    );
                }
            }

            _ => debug!("to be supported: {}", file_name),
        }
    }

    match pihole::cli::restart_dns() {
        Ok(_) => {
            info!("restarted dns service");
        }
        Err(e) => {
            error!(
                "error while restarting dns service after processing archive: {}",
                e
            );
            std::process::exit(2);
        }
    }

    info!("done importing");
}
