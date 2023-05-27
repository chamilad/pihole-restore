use flate2::read::GzDecoder;
use log::{debug, warn};
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Clone, Copy)]
pub enum DomainType {
    Whitelist = 0,
    Blacklist = 1,
    WhitelistRegex = 2,
    BlacklistRegex = 3,
}

pub fn restore_domainlist(
    db_file: &str,
    domain_type: DomainType,
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush: bool,
) -> Result<i32, Box<dyn Error>> {
    let _ = flush
        && flush_table(
            db_file,
            "domainlist",
            format!("WHERE type = {}", domain_type as i32).as_str(),
        )?;

    let conn: Connection = connect_sqlite(db_file)?;

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    let records: Vec<Domain> = serde_json::from_str(&s).unwrap();
    let record_list: DomainList = DomainList {
        list: records,
        domain_type: domain_type as i32,
    };

    Ok(record_list.restore_table(conn)?)
}

pub fn load_table(
    db_file: &str,
    table: &str,
    file: &mut tar::Entry<'_, GzDecoder<File>>,
    flush: bool,
) -> Result<i32, Box<dyn Error>> {
    let conn: Connection = connect_sqlite(db_file)?;

    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    match table {
        "adlist" => {
            debug!("processing adlist table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<Ad> = serde_json::from_str(&s).unwrap();
            let record_list: AdList = AdList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "domain_audit" => {
            debug!("processing domain_audit table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<DomainAuditEntry> = serde_json::from_str(&s).unwrap();
            let record_list: DomainAuditList = DomainAuditList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "group" => {
            debug!("processing group table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<Group> = serde_json::from_str(&s).unwrap();
            let record_list: GroupList = GroupList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "client" => {
            debug!("processing client table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<Client> = serde_json::from_str(&s).unwrap();
            let record_list: ClientList = ClientList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "client_by_group" => {
            debug!("processing client_by_group table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<ClientGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: ClientGroupAssignmentList =
                ClientGroupAssignmentList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "domainlist_by_group" => {
            debug!("processing domainlist_by_group table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<DomainListGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: DomainListGroupAssignmentList =
                DomainListGroupAssignmentList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        "adlist_by_group" => {
            debug!("processing adlist_by_group table");
            let _ = flush && flush_table(db_file, table, "")?;
            let records: Vec<AdListGroupAssignment> = serde_json::from_str(&s).unwrap();
            let record_list: AdListGroupAssignmentList =
                AdListGroupAssignmentList { list: records };
            Ok(record_list.restore_table(conn)?)
        }
        _ => Err(Box::<dyn Error>::from(format!(
            "invalid table name provided: {}",
            table
        ))),
    }
}

fn connect_sqlite(db_file: &str) -> Result<Connection, Box<dyn Error>> {
    debug!("connecting to SQLite db: {}", db_file);
    let connection = Connection::open(db_file)?;
    Ok(connection)
}

fn flush_table(db_file: &str, table: &str, condition: &str) -> Result<bool, Box<dyn Error>> {
    let conn: Connection = connect_sqlite(db_file)?;
    let table_exists_sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
    let mut table_entry_stmt = conn.prepare(&table_exists_sql)?;
    let mut table_entry = table_entry_stmt.query(params![table])?;
    if let Some(_) = table_entry.next()? {
        let sanitised_condition: String;
        if !condition.is_empty() && !str::starts_with(condition, " ") {
            sanitised_condition = format!(" {}", condition).to_string();
        } else {
            sanitised_condition = condition.to_string();
        }

        debug!("flushing table {}", table);
        let clear_sql = format!("DELETE FROM \"{}\"{}", table, sanitised_condition);
        let count = conn.execute(&clear_sql, [])?;
        debug!("flushed {} records from {} table", count, table);
        Ok(true)
    } else {
        Err(Box::<dyn Error>::from(format!(
            "cannot flush table since it doesn't exist: {}",
            table,
        )))
    }
}

trait Restorable {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>>;
}

#[derive(Debug, Deserialize)]
struct DomainList {
    pub domain_type: i32,
    pub list: Vec<Domain>,
}

#[derive(Debug, Deserialize)]
struct Domain {
    pub id: i32,
    pub domain: String,
    pub enabled: i32,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for DomainList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring domainlist table");

        let sql = format!("INSERT OR IGNORE INTO domainlist (id,domain,enabled,date_added,comment,type) VALUES (:id,:domain,:enabled,:date_added,:comment,{});", self.domain_type);
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to domainlist", record_count);

        for record in &self.list {
            // todo: stmt.execute(named_params!{":id": &record.id, ":domain": &record.domain})
            let result = stmt.execute_named(&[
                (":id", &record.id),
                (":domain", &record.domain),
                (":enabled", &record.enabled),
                (":date_added", &record.date_added),
                (":comment", &record.comment),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!("error while inserting an entry to domainlist table: {}", e);
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct AdList {
    pub list: Vec<Ad>,
}

#[derive(Debug, Deserialize)]
struct Ad {
    pub id: i32,
    pub address: String,
    pub enabled: i32,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for AdList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring adlist table");

        let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to adlist", record_count);

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":id", &record.id),
                (":address", &record.address),
                (":enabled", &record.enabled),
                (":date_added", &record.date_added),
                (":comment", &record.comment),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!("error while inserting an entry to adlist table: {}", e);
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct DomainAuditList {
    pub list: Vec<DomainAuditEntry>,
}

#[derive(Debug, Deserialize)]
struct DomainAuditEntry {
    pub id: i32,
    pub domain: String,
    pub date_added: i64,
}

impl Restorable for DomainAuditList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring domain_audit table");

        let sql = "INSERT OR IGNORE INTO domain_audit (id,domain,date_added) VALUES (:id,:domain,:date_added);".to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to domain_audit", record_count);

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":id", &record.id),
                (":domain", &record.domain),
                (":date_added", &record.date_added),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "error while inserting an entry to domain_audit table: {}",
                        e
                    );
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct GroupList {
    pub list: Vec<Group>,
}

#[derive(Debug, Deserialize)]
struct Group {
    pub id: i32,
    pub name: String,
    pub date_added: i64,
    pub description: String,
}

impl Restorable for GroupList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring group table");

        let sql =
            "INSERT OR IGNORE INTO \"group\" (id,name,date_added,description) VALUES (:id,:name,:date_added,:description);"
                .to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to group", record_count);

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":id", &record.id),
                (":name", &record.name),
                (":date_added", &record.date_added),
                (":description", &record.description),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!("error while inserting an entry to group table: {}", e);
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct ClientList {
    pub list: Vec<Client>,
}

#[derive(Debug, Deserialize)]
struct Client {
    pub id: i32,
    pub ip: String,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for ClientList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring client table");

        let sql =
            "INSERT OR IGNORE INTO client (id,ip,date_added,comment) VALUES (:id,:ip,:date_added,:comment);"
                .to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!("starting to load {} records to client", record_count);

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":id", &record.id),
                (":ip", &record.ip),
                (":date_added", &record.date_added),
                (":comment", &record.comment),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!("error while inserting an entry to group table: {}", e);
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct ClientGroupAssignmentList {
    pub list: Vec<ClientGroupAssignment>,
}

#[derive(Debug, Deserialize)]
struct ClientGroupAssignment {
    pub client_id: i32,
    pub group_id: i32,
}

impl Restorable for ClientGroupAssignmentList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring client_by_group table");

        let sql =
            "INSERT OR IGNORE INTO client_by_group (client_id,group_id) VALUES (:client_id,:group_id);"
                .to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!(
            "starting to load {} records to client_by_group",
            record_count
        );

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":client_id", &record.client_id),
                (":group_id", &record.group_id),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "error while inserting an entry to client_by_group table: {}",
                        e
                    );
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct DomainListGroupAssignmentList {
    pub list: Vec<DomainListGroupAssignment>,
}

#[derive(Debug, Deserialize)]
struct DomainListGroupAssignment {
    pub domainlist_id: i32,
    pub group_id: i32,
}

impl Restorable for DomainListGroupAssignmentList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring domainlist_by_group table");

        let sql =
            "INSERT OR IGNORE INTO domainlist_by_group (domainlist_id,group_id) VALUES (:domainlist_id,:group_id);"
                .to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!(
            "starting to load {} records to domainlist_by_group",
            record_count
        );

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":domainlist_id", &record.domainlist_id),
                (":group_id", &record.group_id),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "error while inserting an entry to domainlist_by_group table: {}",
                        e
                    );
                }
            }
        }

        Ok(record_count)
    }
}

#[derive(Debug, Deserialize)]
struct AdListGroupAssignmentList {
    pub list: Vec<AdListGroupAssignment>,
}

#[derive(Debug, Deserialize)]
struct AdListGroupAssignment {
    pub adlist_id: i32,
    pub group_id: i32,
}

impl Restorable for AdListGroupAssignmentList {
    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring adlist_by_group table");

        let sql =
            "INSERT OR IGNORE INTO adlist_by_group (adlist_id,group_id) VALUES (:adlist_id,:group_id);"
                .to_string();
        let mut stmt = conn.prepare(&sql)?;

        let record_count = self.list.len() as i32;
        debug!(
            "starting to load {} records to adlist_by_group",
            record_count
        );

        for record in &self.list {
            let result = stmt.execute_named(&[
                (":adlist_id", &record.adlist_id),
                (":group_id", &record.group_id),
            ]);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "error while inserting an entry to adlist_by_group table: {}",
                        e
                    );
                }
            }
        }

        Ok(record_count)
    }
}
