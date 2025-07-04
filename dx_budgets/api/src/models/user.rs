use uuid::Uuid;
use chrono::NaiveDate;
#[cfg(feature = "server")]
use welds::WeldsModel;

#[derive(Debug, Clone)]
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