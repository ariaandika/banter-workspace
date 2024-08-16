use paste::paste;

macro_rules! table {
    ($tb:ident,$id:ident) => { table!($tb,$id,$tb); };
    ($tb:ident,$id:ident,$al:ident) => {paste!{
imtable!($tb,$id,$al);
pub const [<DELETE_ $al:upper>]:&str = concat!("DELETE FROM ",stringify!($tb)," WHERE ",stringify!($id)," = $1");
    }};
}

macro_rules! imtable {
    ($tb:ident,$id:ident) => { table!($tb,$id,$tb); };
    ($tb:ident,$id:ident,$al:ident) => {paste!{
pub const [<$al:upper _TABLE>]:&str = stringify!($tb);
pub const [<SELECT_ $al:upper>]:&str = concat!("SELECT * FROM ",stringify!($tb)," LIMIT $1 OFFSET $2");
pub const [<FIND_ $al:upper>]:&str = concat!("SELECT * FROM ",stringify!($tb)," WHERE ",stringify!($id)," = $1");
    }};
}

macro_rules! select {
    ($tb:ident,$id:ident) => { select!($tb,$id,$tb); };
    ($tb:ident,$f:ident,$al:ident) => {paste!{
pub const [<SELECT_ $tb:upper _BY_ $f:upper>]:&str = concat!("SELECT * FROM ",stringify!($tb)," WHERE ",stringify!($f)," = $1 LIMIT $2 OFFSET $3");
    }};
}

macro_rules! find {
    ($tb:ident,$id:ident) => { find!($tb,$id,$tb); };
    ($tb:ident,$f:ident,$al:ident) => {paste!{
pub const [<FIND_ $al:upper _BY_ $f:upper>]:&str = concat!("SELECT * FROM ",stringify!($tb)," WHERE ",stringify!($f)," = $1 LIMIT 1");
    }};
}

pub const MAX_LIMIT: i32 = 100;
pub const DEFAULT_LIMIT: i32 = 10;

table!(users, user_id);
table!(warehouses, user_id, wh);
imtable!(orders, order_id);
imtable!(tracings, tracing_id);
imtable!(manifests, manifest_id);

table!(manifest_orders, manifest_id);

imtable!(order_status, order_id);
imtable!(users_snapshot, snapshot_id, users_sn);
imtable!(wh_snapshot, snapshot_id, wh_sn);

find!(users, phone);
find!(warehouses, phone, wh);
select!(tracings, order_id);
select!(order_status, wh_id);
find!(order_status, wh_id);

pub const FIND_LATEST_TRACING: &str = concat!(
    "SELECT * FROM tracings WHERE order_id = $1 ORDER BY traced_at DESC LIMIT 1"
);

pub const INSERT_USERS: &str = concat!("INSERT INTO users(",
    "name,phone,password,role",
    ") VALUES ($1,$2,$3,$4)"
);
pub const INSERT_WH: &str = "INSERT INTO warehouses(wh_name,wh_type) VALUES ($1,$2)";
pub const INSERT_ORDERS: &str = concat!("INSERT INTO orders(",
    "sender_sid,receiver_sid,destination,packages",
    ") VALUES ($1,$2,$3,$4)"
);
pub const INSERT_TRACING: &str = concat!("INSERT INTO tracings(",
    "order_id,subject_sid,wh_sid,status",
    ") VALUES ($1,$2,$3,$4)"
);
pub const INSERT_USERS_SN: &str = "INSERT INTO users_snapshot(data) VALUES ($1)";
pub const INSERT_WH_SN: &str = "INSERT INTO wh_snapshot(data) VALUES ($1)";
pub const INSERT_EMPLOYEES: &str = "INSERT INTO employees(user_id,wh_id) VALUES($1,$2)";
pub const INSERT_ORDER_STATUS: &str = "INSERT INTO order_status(order_id,tracing_id,wh_id) VALUES($1,$2,$3)";

