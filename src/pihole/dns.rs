use crate::pihole::cli;
use flate2::read::GzDecoder;
use log::{debug, warn};
use std::error::Error;
use std::fs::File;
use std::io::Read;

const CUSTOM_DNS_FILE: &str = "/etc/pihole/custom.list";
const CNAME_CONFIG_FILE: &str = "/etc/dnsmasq.d/05-pihole-custom-cname.conf";

#[derive(Debug)]
struct CustomDNSEntry {
    ip: String,
    domain: String,
}

#[derive(Debug)]
struct CNameConfigEntry {
    domain: String,
    target: String,
}

pub fn process_local_dns_entries(
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush: bool,
) -> Result<i32, Box<dyn Error>> {
    if flush {
        if !flush_local_dns_entries()? {
            warn!("could not flush local dns entries");
        }
    }

    // todo: dedup
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    let incoming_dns_entries = get_local_dns_entries(&s);
    for entry in incoming_dns_entries {
        let add_cmd = vec!["-a", "addcustomdns", &entry.ip, &entry.domain, "false"];
        match cli::execute(add_cmd) {
            Ok(_) => debug!("added dns entry: {}->{}", entry.ip, entry.domain),
            Err(e) => warn!(
                "error while adding dns entry {}-.{}: {}",
                entry.ip, entry.domain, e
            ),
        }
    }

    match cli::restart_dns() {
        Ok(_) => {
            debug!("restarted dns service after loading custom dns entries");
            Ok(0)
        }
        Err(e) => {
            warn!(
                "error while restarting dns service after loading custom dns entries: {}",
                e
            );
            Err(Box::new(e))
        }
    }
}

fn flush_local_dns_entries() -> Result<bool, Box<dyn Error>> {
    let current_entries = get_current_local_dns_entries()?;
    for entry in current_entries {
        // setting false at the end avoids pihole restarting dns for every command execution
        let flush_cmd: Vec<&str> = vec!["-a", "removecustomdns", &entry.ip, &entry.domain, "false"];
        match cli::execute(flush_cmd) {
            Ok(_) => debug!("removed dns entry: {}->{}", entry.ip, entry.domain),
            Err(e) => warn!(
                "error while trying remove custom dns entry {}->{}: {}",
                entry.ip, entry.domain, e
            ),
        }
    }
    Ok(true)
}

fn get_current_local_dns_entries() -> Result<Vec<CustomDNSEntry>, Box<dyn Error>> {
    let mut file = File::open(CUSTOM_DNS_FILE)?;
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    Ok(get_local_dns_entries(&s))
}

fn get_local_dns_entries(contents: &str) -> Vec<CustomDNSEntry> {
    let mut entries: Vec<CustomDNSEntry> = Vec::new();
    for entry in contents.lines() {
        let sections: Vec<&str> = entry.split(" ").collect();
        if sections.len() != 2 {
            warn!(
                "invalid entry found while reading existing custom.list file: {}",
                entry
            );
            continue;
        }

        let dns_entry = CustomDNSEntry {
            ip: sections[0].to_string(),
            domain: sections[1].to_string(),
        };
        entries.push(dns_entry);
    }
    entries
}

pub fn process_local_cname_entries(
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush: bool,
) -> Result<i32, Box<dyn Error>> {
    if flush {
        if !flush_cname_config()? {
            warn!("could not flush existing cname config");
        } else {
            debug!("flushed existing cname config");
        }
    }

    // todo: dedup
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    let incoming_dns_entries = get_cname_entries(&s);
    for entry in incoming_dns_entries {
        let add_cmd = vec![
            "-a",
            "addcustomcname",
            &entry.domain,
            &entry.target,
            "false",
        ];
        match cli::execute(add_cmd) {
            Ok(_) => debug!("added cname entry: {}->{}", entry.domain, entry.target),
            Err(e) => warn!(
                "error while adding cname entry {}-.{}: {}",
                entry.domain, entry.target, e
            ),
        }
    }

    match cli::restart_dns() {
        Ok(_) => {
            debug!("restarted dns service after loading custom cname entries");
            Ok(0)
        }
        Err(e) => {
            warn!(
                "error while restarting dns service after loading custom cname entries: {}",
                e
            );
            Err(Box::new(e))
        }
    }
}

fn flush_cname_config() -> Result<bool, Box<dyn Error>> {
    let current_entries = get_current_cname_config()?;
    for entry in current_entries {
        // setting false at the end avoids pihole restarting dns for every command execution
        let flush_cmd: Vec<&str> = vec![
            "-a",
            "removecustomcname",
            &entry.domain,
            &entry.target,
            "false",
        ];
        match cli::execute(flush_cmd) {
            Ok(_) => debug!("removed cname entry: {}->{}", entry.domain, entry.target),
            Err(e) => warn!(
                "error while trying remove custom dns entry {}->{}: {}",
                entry.domain, entry.target, e
            ),
        }
    }
    Ok(true)
}

fn get_current_cname_config() -> Result<Vec<CNameConfigEntry>, Box<dyn Error>> {
    let mut file = File::open(CNAME_CONFIG_FILE)?;
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    Ok(get_cname_entries(&s))
}

fn get_cname_entries(contents: &str) -> Vec<CNameConfigEntry> {
    let mut entries: Vec<CNameConfigEntry> = Vec::new();
    for entry in contents.lines() {
        //cname=<DOMAIN>,<TARGET>
        let cleaned_entry = str::replace(entry, "cname=", "");
        let sections: Vec<&str> = cleaned_entry.split(",").collect();
        if sections.len() != 2 {
            warn!(
                "invalid entry found while reading existing cname config file: {}",
                entry
            );
            continue;
        }

        let cname_entry = CNameConfigEntry {
            domain: sections[0].to_string(),
            target: sections[1].to_string(),
        };
        entries.push(cname_entry);
    }
    entries
}
