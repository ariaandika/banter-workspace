use chrono::{DateTime, Utc};
use derives::{EnumDecode, EnumExt, FromRow, IdDecode};
use serde_json::Value;

pub use serde::{Serialize, Deserialize};
pub type Date = DateTime<Utc>;

#[derive(Debug, Serialize, Deserialize, EnumExt, EnumDecode)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Customer,
    Sales,
    Driver
}

#[derive(Debug, Serialize, Deserialize, IdDecode)]
pub struct UserId(pub i32);

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Users {
    pub user_id: UserId,
    pub name: String,
    pub phone: String,
    #[serde(skip_deserializing)]
    pub password: String,
    pub role: Role,
    pub metadata: Value,
    pub created_at: Date,
    pub updated_at: Date,
    pub verified_at: Option<Date>,
}

