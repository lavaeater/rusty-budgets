use sqlx::types::{chrono, Uuid};
use welds::WeldsModel;

#[derive(Debug, Clone, WeldsModel)]
#[welds(table = "users")]
pub struct User {
    #[welds(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub birthday: Option<chrono::NaiveDate>,
}