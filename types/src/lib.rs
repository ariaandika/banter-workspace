pub use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

pub type Date = DateTime<Utc>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Customer,
    Sales,
    Driver
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserId(pub i32);

#[derive(Debug, Serialize, Deserialize)]
pub struct Users {
    pub user_id: i32,
    pub name: String,
    pub phone: String,
    #[serde(skip_deserializing)]
    pub password: String,
    pub role: Role,
    pub created_at: Date,
    pub updated_at: Date,
    pub verified_at: Option<Date>,
}

