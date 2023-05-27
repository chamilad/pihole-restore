use crate::pihole::cli;
use flate2::read::GzDecoder;
use log::{debug, warn};
use regex::Regex;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

const STATIC_DHCP_CONF_FILE: &str = "/etc/dnsmasq.d/04-pihole-static-dhcp.conf";

pub fn process_static_dhcp(
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush: bool,
) -> Result<i32, Box<dyn Error>> {
    // https://github.com/pi-hole/pi-hole/blob/d885e92674e8d8d9a673b35ae706b2c49ea05840/advanced/Scripts/webpage.sh#L537
    // this inserts different formats according to given input, so it's possible the backup could
    // contain static dhcp entries of these types
    // Pihole's Admin page Teleporer code is buggy when partial information is specified
    enum StaticDHCPType {
        Full,           // when all three are defined
        StaticIP,       // when no host name is defined
        StaticHostName, // when no IP address is defined
    }

    if flush && Path::new(STATIC_DHCP_CONF_FILE).exists() {
        debug!("flushing existing static dhcp configuration");
        match OpenOptions::new()
            .read(true)
            .write(true)
            .open(STATIC_DHCP_CONF_FILE)
        {
            Err(e) => warn!("error while opening static dhcp config to flush: {}", e),
            Ok(file) => match file.set_len(0) {
                Err(e) => {
                    warn!("error while truncating static dhcp config file: {}", e)
                }
                Ok(_) => debug!("static dhcp config truncated successfully"),
            },
        }
    }

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    for entry in s.lines() {
        debug!("processing static dhcp lease: {}", entry);
        // dhcp-host=<MAC_ADDR>,<IP>,<HOSTNAME>  or variants where ip or hostname is missing
        let sections: Vec<&str> = entry.split(",").collect();

        let mode: StaticDHCPType;
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
                        add_static_dhcp_entry(addr.as_str(), sections[1], sections[2], flush)?
                    }
                    StaticDHCPType::StaticIP => {
                        add_static_dhcp_entry(addr.as_str(), sections[1], "nohost", flush)?
                    }
                    StaticDHCPType::StaticHostName => {
                        add_static_dhcp_entry(addr.as_str(), "noip", sections[1], flush)?
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

    match cli::restart_dns() {
        Ok(_) => {
            debug!("restarted dns service after loading static dhcp entries");
            Ok(0)
        }
        Err(e) => {
            warn!(
                "error while restarting dns service after loading static dhcp entries: {}",
                e
            );
            Err(Box::new(e))
        }
    }
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

fn add_static_dhcp_entry(
    mac: &str,
    ip: &str,
    hostname: &str,
    flushed: bool,
) -> Result<bool, Box<dyn Error>> {
    // todo: sanitisation

    // check for duplicates, if the file was flushed, no need to check for duplicates
    if Path::new(STATIC_DHCP_CONF_FILE).exists() && !flushed {
        let mut file = File::open(STATIC_DHCP_CONF_FILE)?;
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
    let exec_result = cli::execute(add_cmd);
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
