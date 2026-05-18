use crate::models::budget_item::Periodicity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub periodicity: Periodicity,
    pub deleted: bool,
}

impl Tag {
    pub fn new(id: Uuid, name: String, periodicity: Periodicity) -> Self {
        Self {
            id,
            name,
            periodicity,
            deleted: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tag_not_deleted() {
        let id = Uuid::new_v4();
        let tag = Tag::new(id, "Electricity".to_string(), Periodicity::Monthly);
        assert_eq!(tag.id, id);
        assert_eq!(tag.name, "Electricity");
        assert_eq!(tag.periodicity, Periodicity::Monthly);
        assert!(!tag.deleted);
    }

    #[test]
    fn serde_round_trip() {
        let tag = Tag::new(Uuid::new_v4(), "Insurance".to_string(), Periodicity::Annual);
        let json = serde_json::to_string(&tag).unwrap();
        let back: Tag = serde_json::from_str(&json).unwrap();
        assert_eq!(tag, back);
    }
}
