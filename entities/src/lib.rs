//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

use sha2::{Digest, Sha256};

pub mod prelude;

pub mod bank_transaction;
pub mod episode;
pub mod import;
pub mod import_row;
pub mod member;
pub mod post;
pub mod user;
pub mod budget_item;
pub mod budget_plan;
pub mod budget_plan_item;

pub trait RecordHash {
    fn hash(&self) -> String;
}

pub fn calculate_member_hash(first_name: &str, last_name: &str, birth_date: &str) -> String {
    let normalized = format!(
        "{}:{}:{}",
        first_name.trim().to_lowercase(),
        last_name.trim().to_lowercase(),
        birth_date.trim().to_lowercase()
    );

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(normalized);
    let result = hasher.finalize();

    // Convert to hex string
    format!("{:x}", result)
}

impl RecordHash for member::Model {
    fn hash(&self) -> String {
        let date_string = self
            .birth_date
            .as_ref()
            .map(|date| date.to_string())
            .unwrap_or("".to_string());
        calculate_member_hash(&self.first_name, &self.last_name, &date_string)
    }
}

impl RecordHash for import_row::Model {
    fn hash(&self) -> String {
        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(self.data.to_string());
        let result = hasher.finalize();

        // Convert to hex string
        format!("{:x}", result)
    }
}

impl RecordHash for bank_transaction::Model {
    fn hash(&self) -> String {
        // Calculate SHA-256 hash
        calculate_bank_transaction_hash(&self.other_fields)
    }
}

pub fn calculate_bank_transaction_hash(other_fields: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(other_fields);
    let result = hasher.finalize();

    // Convert to hex string
    format!("{:x}", result)
}
