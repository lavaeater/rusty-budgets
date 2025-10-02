use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::DomainEvent;
use crate::events::budget_created::BudgetCreated;
use crate::events::item_added::ItemAdded;
use crate::events::item_funds_adjusted::ItemFundsAdjusted;
use crate::events::item_funds_reallocated::ItemFundsReallocated;
use crate::events::transaction_added::TransactionAdded;
use crate::events::transaction_connected::TransactionConnected;
use crate::events::ItemModified;
use crate::models::bank_transaction::BankTransactionStore;
use crate::models::budget_item::{BudgetItem, BudgetItemStore};
use crate::models::budget_period::BudgetPeriodStore;
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};
use crate::models::{BankTransaction, BudgetingTypeOverview, ValueKind};
use crate::pub_events_enum;
use chrono::{DateTime, Datelike, Utc};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        ItemAdded,
        TransactionAdded,
        TransactionConnected,
        ItemFundsReallocated,
        ItemFundsAdjusted,
        ItemModified,
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    budget_periods: BudgetPeriodStore,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
    pub currency: Currency,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: "".to_string(),
            user_id: Default::default(),
            budget_periods: Default::default(),
            created_at: Default::default(),
            updated_at: Default::default(),
            default_budget: false,
            last_event: 0,
            version: 0,
            currency: Default::default(),
        }
    }
}

impl Budget {
    pub fn new(id: Uuid) -> Self {
        let today = Utc::now();
        Self {
            id,
            budget_periods: BudgetPeriodStore::new(today, None),
            ..Default::default()
        }
    }

    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self.budget_periods.get_item(item_id)
    }

    pub fn get_type_for_item(&self, item_id: &Uuid) -> Option<&BudgetingType> {
        self.budget_periods.get_type_for_item(item_id)
    }

    pub fn items_by_type(
        &self,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)> {
        self.budget_periods.items_by_type()
    }

    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_periods.budgeted_for_type(budgeting_type)
    }

    pub fn spent_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_periods.spent_for_type(budgeting_type)
    }

    pub fn recalc_overview(&mut self) {
        self.budget_periods.recalc_overview();
    }

    pub fn insert_item(&mut self, item: &BudgetItem, item_type: BudgetingType) {
        self.budget_periods.insert_item(item, item_type);
    }

    pub fn remove_item(&mut self, item_id: &Uuid) {
        self.budget_periods.remove_item(item_id);
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) {
        self.budget_periods.insert_transaction(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.budget_periods.can_insert_transaction(tx_hash)
    }

    pub fn contains_transaction(&self, tx_id: &Uuid) -> bool {
        self.budget_periods.contains_transaction(tx_id)
    }

    pub fn contains_budget_item(&self, item_id: &Uuid) -> bool {
        self.budget_periods.contains_budget_item(item_id)
    }

    pub fn get_transaction_mut(&mut self, tx_id: &Uuid) -> Option<&mut BankTransaction> {
        self.budget_periods.get_transaction_mut(tx_id)
    }

    pub fn get_transaction(&self, tx_id: &Uuid) -> Option<&BankTransaction> {
        self.budget_periods.get_transaction(tx_id)
    }

    pub fn type_for_item(&self, item_id: &Uuid) -> Option<BudgetingType> {
        self.budget_periods.type_for_item(item_id)
    }

    pub fn update_budget_actual_amount(&mut self, budgeting_type: &BudgetingType, amount: &Money) {
        self.budget_periods
            .update_budget_actual_amount(budgeting_type, amount);
    }

    pub fn update_budget_budgeted_amount(
        &mut self,
        budgeting_type: &BudgetingType,
        amount: &Money,
    ) {
        self.budget_periods
            .update_budget_budgeted_amount(budgeting_type, amount)
    }

    pub fn add_actual_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_periods
            .add_actual_amount_to_item(item_id, amount);
    }

    pub fn add_budgeted_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_periods
            .add_budgeted_amount_to_item(item_id, amount);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modify_budget_item(
        &mut self,
        id: &Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) {
        self.budget_periods.modify_budget_item(
            id,
            name,
            item_type,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        );
    }

    pub fn get_budgeted_by_type(&self, budgeting_type: &BudgetingType) -> Option<&Money> {
        self.budget_periods.get_budgeted_by_type(budgeting_type)
    }

    pub fn get_actual_by_type(&self, budgeting_type: &BudgetingType) -> Option<&Money> {
        self.budget_periods.get_actual_by_type(budgeting_type)
    }

    pub fn get_budgeting_overview(
        &self,
        budgeting_type: &BudgetingType,
    ) -> Option<&BudgetingTypeOverview> {
        self.budget_periods.get_budgeting_overview(budgeting_type)
    }

    pub fn list_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods.list_bank_transactions()
    }

    fn ensure_time_period(&mut self, updated_at: DateTime<Utc>) {
        self.budget_periods.set_current_period(updated_at);
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn _new(id: Self::Id) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    fn _default() -> Self {
        Self::default()
    }

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>) {
        if self.last_event < timestamp {
            self.last_event = timestamp;
            self.updated_at = updated_at;
            if self.version == 0 {
                self.created_at = updated_at;
            }
            self.version += 1;
        } else {
            panic!("Event timestamp is older than last event timestamp");
        }
    }

    fn version(&self) -> u64 {
        self.version
    }
}
