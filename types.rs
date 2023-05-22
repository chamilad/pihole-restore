use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Domain {
    pub id: i32,
    pub domain: String,
    pub enabled: i32,
    pub date_added: i64,
    pub comment: String,
}
