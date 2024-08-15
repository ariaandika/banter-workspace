use chrono::{DateTime, Utc};
use derives::{EnumDecode, EnumExt, FromRow, IdDecode};
use serde_json::Value;

macro_rules! id {
    ($n:tt) => {
        #[derive(Debug, Serialize, Deserialize, IdDecode)]
        pub struct $n(pub i32);
    };
}

pub use serde::{Serialize, Deserialize};
pub type Date = DateTime<Utc>;

#[derive(Debug, Serialize, Deserialize, EnumExt, EnumDecode)]
pub enum Role {
    Admin,
    Customer,
    Sales,
    Driver
}

#[derive(Debug, Serialize, Deserialize, EnumExt, EnumDecode)]
pub enum WhType {
    Counter,
    Warehouse,
    DistCenter
}

#[derive(Debug, Serialize, Deserialize, EnumExt, EnumDecode)]
pub enum Status {
    Warehouse,
    Driver,
    Completed,
}

id!(UserId);
id!(WhId);
id!(OrderId);
id!(TracingId);
id!(ManifestId);
id!(UserSid);
id!(WhSid);

#[derive(Debug, Serialize, FromRow)]
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

#[derive(Debug, Serialize, FromRow)]
pub struct Warehouses {
    pub wh_id: WhId,
    pub wh_name: String,
    pub wh_type: WhType,
    pub created_at: Date,
    pub updated_at: Date,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Orders {
    pub order_id: OrderId,
    pub sender_sid: UserSid,
    pub receiver_sid: UserSid,
    pub destination: String,
    pub packages: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Tracings {
    pub tracing_id: TracingId,
    pub order_id: OrderId,
    pub subject_sid: UserSid,
    pub wh_sid: WhSid,
    pub status: Status,
    pub traced_at: Date,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Manifests {
    pub manifest_id: ManifestId,
    pub sales_sid: UserSid,
    pub driver_sid: UserSid,
    pub wh_from_sid: WhSid,
    pub wh_to_sid: WhSid,
    pub created_at: Date,
    pub completed_at: Option<Date>,
}



#[derive(Debug, Serialize, FromRow)]
pub struct UsersSnapshot {
  pub snapshot_id: UserSid,
  pub data: String, // json
  pub snapshoted_at: Date,
}

#[derive(Debug, Serialize, FromRow)]
pub struct WhSnapshot {
  pub snapshot_id: WhSid,
  pub data: String, // json
  pub snapshoted_at: Date,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Employees {
    pub user_id: UserId,
    pub wh_id: WhId,
    pub created_at: Date,
}

#[derive(Debug, Serialize, FromRow)]
pub struct OrderStatus {
    pub order_id: OrderId,
    pub tracing_id: TracingId,
    pub wh_id: WhId, // query fields
}

#[derive(Debug, Serialize, FromRow)]
pub struct ManifestOrders {
    pub manifest_id: ManifestId,
    pub order_id: OrderId,
}



