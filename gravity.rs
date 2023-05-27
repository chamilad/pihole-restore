use log::{debug, error, info, warn};
use rusqlite::{named_params, Connection, ToSql};
use serde::Deserialize;
use std::error::Error;

pub trait Restorable {
    type ListType;

    fn restore_table(&self, conn: Connection) -> Result<i32, Box<dyn Error>> {
        debug!("restoring table: {}", self.get_table_name());
        let sql = self.get_store_statement()?;
        let mut stmt = conn.prepare(&sql)?;

        let list = self.get_list();
        debug!(
            "starting to load {} records to domainlist",
            list.len() as i32
        );

        for idx in 0..list.len() {
            let params = self.get_store_parameters(idx)?;
            let result = stmt.execute(params);

            match result {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "error while inserting an entry to {} table: {}",
                        self.get_table_name(),
                        e
                    );
                }
            }
        }

        // todo: get the actual processed count
        Ok(0)
    }

    fn flush_table(&self, conn: Connection) -> Result<(), Box<dyn Error>> {
        debug!("flushing {} table", self.get_table_name());
        let clear_sql = format!("DELETE FROM \"{}\"", self.get_table_name());
        conn.execute(&clear_sql, [])?;
        Ok(())
    }

    fn get_table_name(&self) -> &str;
    fn get_list(&self) -> &Vec<Self::ListType>;
    fn get_store_statement(&self) -> Result<String, Box<dyn Error>>;
    fn get_store_parameters(&self, idx: usize) -> Result<Vec<(&str, &dyn ToSql)>, Box<dyn Error>>;
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
    type ListType = Domain;

    fn get_table_name(&self) -> &str {
        "domainlist"
    }

    fn flush_table(&self, conn: Connection) -> Result<(), Box<dyn Error>> {
        debug!("flushing type {} from domainlist table", self.domain_type);
        let clear_sql = format!(
            "DELETE FROM \"domainlist\" WHERE type == {}",
            self.domain_type
        );
        conn.execute(&clear_sql, [])?;
        Ok(())
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql = format!("INSERT OR IGNORE INTO domainlist (id,domain,enabled,date_added,comment,type) VALUES (:id,:domain,:enabled,:date_added,:comment,{});", self.domain_type);
        Ok(sql)
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = &self.list[idx];
        let params: Vec<(&'static str, Box<dyn ToSql + 'static>)> = named_params! {
            ":id": &record.id,
            ":domain": &record.domain,
            ":enabled": &record.enabled,
            ":date_added": &record.date_added,
            ":comment": &record.comment,
        };

        Ok(params)
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
    type ListType = Ad;

    fn get_table_name(&self) -> &str {
        "adlist"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql = "INSERT OR IGNORE INTO adlist (id,address,enabled,date_added,comment) VALUES (:id,:address,:enabled,:date_added,:comment);".to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
            ":id": &record.id,
            ":address": &record.address,
            ":enabled": &record.enabled,
            ":date_added": &record.date_added,
            ":comment": &record.comment,
        };
        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct DomainAuditList {
    pub list: Vec<DomainAuditEntry>,
}

#[derive(Debug, Deserialize)]
pub struct DomainAuditEntry {
    pub id: i32,
    pub domain: String,
    pub date_added: i64,
}

impl Restorable for DomainAuditList {
    type ListType = DomainAuditEntry;

    fn get_table_name(&self) -> &str {
        "domain_audit"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql = "INSERT OR IGNORE INTO domain_audit (id,domain,date_added) VALUES (:id,:domain,:date_added);".to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
            ":id": &record.id,
            ":domain": &record.domain,
            ":date_added": &record.date_added,
        };

        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct GroupList {
    pub list: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub date_added: i64,
    pub description: String,
}

impl Restorable for GroupList {
    type ListType = Group;

    fn get_table_name(&self) -> &str {
        "group"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql =
            "INSERT OR IGNORE INTO \"group\" (id,name,date_added,description) VALUES (:id,:name,:date_added,:description);".to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
            ":id": &record.id,
            ":name": &record.name,
            ":date_added": &record.date_added,
            ":description": &record.description,
        };
        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct ClientList {
    pub list: Vec<Client>,
}

#[derive(Debug, Deserialize)]
pub struct Client {
    pub id: i32,
    pub ip: String,
    pub date_added: i64,
    pub comment: String,
}

impl Restorable for ClientList {
    type ListType = Client;

    fn get_table_name(&self) -> &str {
        "client"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql =
            "INSERT OR IGNORE INTO client (id,ip,date_added,comment) VALUES (:id,:ip,:date_added,:comment);"
                .to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
                    ":id": &record.id,
                    ":ip": &record.ip,
                    ":date_added": &record.date_added,
                    ":comment": &record.comment,
        };
        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct ClientGroupAssignmentList {
    pub list: Vec<ClientGroupAssignment>,
}

#[derive(Debug, Deserialize)]
pub struct ClientGroupAssignment {
    pub client_id: i32,
    pub group_id: i32,
}

impl Restorable for ClientGroupAssignmentList {
    type ListType = ClientGroupAssignment;

    fn get_table_name(&self) -> &str {
        "client_by_group"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql =
            "INSERT OR IGNORE INTO client_by_group (client_id,group_id) VALUES (:client_id,:group_id);"
                .to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
         ":client_id": &record.client_id,
          ":group_id": &record.group_id,
        };
        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct DomainListGroupAssignmentList {
    pub list: Vec<DomainListGroupAssignment>,
}

#[derive(Debug, Deserialize)]
pub struct DomainListGroupAssignment {
    pub domainlist_id: i32,
    pub group_id: i32,
}

impl Restorable for DomainListGroupAssignmentList {
    type ListType = DomainListGroupAssignment;

    fn get_table_name(&self) -> &str {
        "domainlist_by_group"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql =
            "INSERT OR IGNORE INTO domainlist_by_group (domainlist_id,group_id) VALUES (:domainlist_id,:group_id);"
                .to_string();
        Ok(sql)
    }

    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
                ":domainlist_id": &record.domainlist_id,
                ":group_id": &record.group_id,
        };
        Ok(params)
    }
}

#[derive(Debug, Deserialize)]
pub struct AdListGroupAssignmentList {
    pub list: Vec<AdListGroupAssignment>,
}

#[derive(Debug, Deserialize)]
pub struct AdListGroupAssignment {
    pub adlist_id: i32,
    pub group_id: i32,
}

impl Restorable for AdListGroupAssignmentList {
    type ListType = AdListGroupAssignment;
    fn get_table_name(&self) -> &str {
        "adlist_by_group"
    }

    fn get_list(&self) -> &Vec<Self::ListType> {
        &self.list
    }

    fn get_store_statement(&self) -> Result<String, Box<dyn Error>> {
        let sql =
            "INSERT OR IGNORE INTO adlist_by_group (adlist_id,group_id) VALUES (:adlist_id,:group_id);"
                .to_string();
        Ok(sql)
    }
    fn get_store_parameters(&self, idx: usize) -> Result<&[(&str, &dyn ToSql)], Box<dyn Error>> {
        let record = self.list[idx];
        let params = named_params! {
                ":adlist_id": &record.adlist_id,
                ":group_id": &record.group_id,
        };
        Ok(params)
    }
}
