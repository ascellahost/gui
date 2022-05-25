use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub discord_id: String,
    pub domain: String,
    pub id: i32,
    pub key: String,
    pub name: String,
    pub autodelete: Option<i32>,
    pub deleteall: Option<i32>,
    pub upload_key: Option<String>,
    pub url_style: i32,
    pub invite_code: Option<String>,
    pub invited_by: i32,
    pub lang: String,
    pub flags: i32,
}
