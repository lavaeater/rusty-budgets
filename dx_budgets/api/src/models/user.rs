use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::WeldsModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(WeldsModel))]
#[cfg_attr(feature = "server", welds(table = "users"))]
pub struct User {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub user_name: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub birthday: Option<NaiveDate>,
}
