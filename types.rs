use log::{debug, error, info, warn};
use rusqlite::Connection;
use serde::Deserialize;
use std::error::Error;

pub trait Restorable {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>>;
}

#[derive(Debug, Deserialize)]
pub struct DomainList {
    pub domain_type: i32,
    pub list: Vec<Domain>,
}

#[derive(Debug, Deserialize)]
pub struct Domain {
    pub id: i32,
    pub domain: String,
    pub enabled: i32,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for DomainList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("loading domainlist table");

        let sql = format!("INSERT OR IGNORE INTO domainlist (id,domain,enabled,date_added,comment,type) VALUES (:id,:domain,:enabled,:date_added,:comment,{});", self.domain_type);
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to domainlist", record_count);

        for record in &self.list {
            stmt.execute_named(&[
                (":id", &record.id),
                (":domain", &record.domain),
                (":enabled", &record.enabled),
                (":date_added", &record.date_added),
                (":comment", &record.comment),
            ]);
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
pub struct AdList {
    pub list: Vec<Ad>,
}

#[derive(Debug, Deserialize)]
pub struct Ad {
    pub id: i32,
    pub address: String,
    pub enabled: i32,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for AdList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("loading adlist table");

        let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to adlist", record_count);

        for record in &self.list {
            stmt.execute_named(&[
                (":id", &record.id),
                (":address", &record.address),
                (":enabled", &record.enabled),
                (":date_added", &record.date_added),
                (":comment", &record.comment),
            ]);
        }

        Ok(record_count)
    }
}
