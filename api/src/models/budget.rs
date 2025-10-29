use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::DomainEvent;
use crate::events::*;
use crate::models::budget_item::BudgetItem;
use crate::models::budget_period_id::BudgetPeriodId;
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::{BankTransaction, BudgetItemStore, BudgetPeriod, BudgetingTypeOverview, MatchRule, MonthBeginsOn};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use crate::models::budget_period_store::BudgetPeriodStore;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        ItemAdded,
        TransactionAdded,
        TransactionConnected,
        TransactionIgnored,
        ItemFundsReallocated,
        ItemFundsAdjusted,
        ItemModified,
        RuleAdded,
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    budget_periods: BudgetPeriodStore,
    pub budget_items: BudgetItemStore,
    pub match_rules: HashSet<MatchRule>,
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
            budget_items: Default::default(),
            match_rules: HashSet::default(),
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

    pub fn get_current_period_id(&self) -> BudgetPeriodId {
        self.budget_periods.current_period_id()
    }

    pub fn list_ignored_transactions(&self) -> Vec<BankTransaction> {
        self.budget_periods.list_ignored_transactions()
    }
    
    pub fn month_begins_on(&self)-> &MonthBeginsOn {
        self.budget_periods.month_begins_on()
    } 

    pub fn get_type_for_item(&self, item_id: &Uuid) -> Option<&BudgetingType> {
        self.budget_periods.get_type_for_item(item_id)
    }

    pub fn items_by_type(
        &self,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)> {
        self.budget_periods.items_by_type()
    }

    pub fn list_all_items(
        &self,
    ) -> Vec<BudgetItem> {
        self.budget_items.list_all_items()
    }

    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_periods.budgeted_for_type(budgeting_type)
    }

    pub fn spent_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_periods.spent_for_type(budgeting_type)
    }

    pub fn recalc_overview(&mut self, period_id: Option<BudgetPeriodId>) {
        self.budget_periods.recalc_overview(period_id);
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
        self.budget_items.contains(item_id)
    }

    pub fn contains_item_with_name(&self, name: &str) -> bool {
        self.budget_items.contains_item_with_name(name)
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

    pub fn update_budget_actual_amount(&mut self, period_id: BudgetPeriodId, budgeting_type: &BudgetingType, amount: &Money) {
        self.with_period_mut(period_id).update_actual_amount(budgeting_type, amount);
    }

    pub fn update_budget_budgeted_amount(
        &mut self,
        period_id: Option<BudgetPeriodId>,
        budgeting_type: &BudgetingType,
        amount: &Money,
    ) {
        if let Some(period_id) = period_id {
            self.with_period_mut(period_id).update_budgeted_amount(budgeting_type, amount);
        } else {
            self.with_current_period_mut().update_budgeted_amount(budgeting_type, amount);
        }
    }

    pub fn add_actual_amount_to_item(&mut self, period_id: BudgetPeriodId, item_id: &Uuid, amount: &Money) {
        self.with_period_mut(period_id).add_actual_amount_to_item(item_id, amount);
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
    
    pub fn move_transaction_to_ignored(&mut self, tx_id: &Uuid) -> bool {
        self.budget_periods.move_transaction_to_ignored(tx_id)
    }
    
    pub fn list_transactions_for_item(&self, item_id: &Uuid, sorted:bool) -> Vec<&BankTransaction> {
        self.budget_periods.list_transactions_for_item(item_id, sorted)
    }
    
    pub fn list_transactions_for_connection(&self) -> Vec<BankTransaction> {
        self.budget_periods.list_transactions_for_connection()
    }
    
    pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods.list_all_bank_transactions()
    }

    pub fn set_current_period(&mut self, date: &DateTime<Utc>) {
        self.budget_periods.set_current_period(date);
    }
    
    pub fn set_previous_period(&mut self) -> Self {
        self.budget_periods.set_previous_period();
        self.clone()
    }

    pub fn set_next_period(&mut self) -> Self {
        self.budget_periods.set_next_period();
        self.clone()
    }
    
    fn ensure_time_period(&mut self, updated_at: &DateTime<Utc>) {
        self.budget_periods.set_current_period(updated_at);
    }
    
    pub fn evaluate_rules(&self) -> Vec<(Uuid, Uuid)> {
        /* we must evaluate all transactions against all items for the BUDGET, not for 
        a specific period.
         */
        self.budget_periods.evaluate_rules(&self.match_rules, &self.budget_items.list_all_items())
    }
    
    pub fn with_period_mut(&mut self, period_id: BudgetPeriodId) -> &mut BudgetPeriod {
        self.budget_periods.with_period_mut(period_id)
    }

    pub fn with_period(&self, period_id: BudgetPeriodId) -> &BudgetPeriod {
        self.budget_periods.with_period(period_id)
    }
    
    pub fn with_current_period_mut(&mut self) -> &mut BudgetPeriod {
        let period_id = self.get_current_period_id();
        self.with_period_mut(period_id)
    }

    pub fn with_current_period(&self) -> &BudgetPeriod {
        let period_id = self.get_current_period_id();
        self.with_period(period_id)
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
