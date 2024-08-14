//! Users
//! user_id name phone password role metadata created_at updated_at verified_at

pub const SELECT_MAX: i32 = 10;

pub const SELECT_USERS: &str = "SELECT * FROM users";
pub const FIND_USERS  : &str = "SELECT * FROM users WHERE user_id = $1";
pub const FIND_USERS_BY_PHONE  : &str = "SELECT * FROM users WHERE phone = $1";
pub const DELETE_USERS: &str = "DELETE FROM users WHERE user_id = $1";
pub const INSERT_USERS: &str = concat!(
    "INSERT INTO users(",
    "name,phone,password,role,metadata",
    ") VALUES ($1,$2,$3,$4,'null'::json)"
);



