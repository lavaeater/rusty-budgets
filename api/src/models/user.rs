use chrono::NaiveDate;
use joydb::Model as JoyModel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, JoyModel)]
pub struct User {
    pub id: Uuid,
    pub user_name: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub birthday: Option<NaiveDate>,
}

impl User {
    pub fn new(
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_name: user_name.to_string(),
            email: email.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            phone,
            birthday,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_user_fields() {
        let dob = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
        let user = User::new(
            "alice",
            "alice@example.com",
            "Alice",
            "Smith",
            Some("0701234567".to_string()),
            Some(dob),
        );
        assert_eq!(user.user_name, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.first_name, "Alice");
        assert_eq!(user.last_name, "Smith");
        assert_eq!(user.phone, Some("0701234567".to_string()));
        assert_eq!(user.birthday, Some(dob));
    }

    #[test]
    fn new_user_no_optional_fields() {
        let user = User::new("bob", "bob@example.com", "Bob", "Jones", None, None);
        assert!(user.phone.is_none());
        assert!(user.birthday.is_none());
    }

    #[test]
    fn unique_ids() {
        let a = User::new("a", "a@a.com", "A", "A", None, None);
        let b = User::new("a", "a@a.com", "A", "A", None, None);
        assert_ne!(a.id, b.id);
    }
}
