use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::DomainEvent;
use crate::events::*;
use crate::models::budget_item::BudgetItem;
use crate::models::budget_period_id::PeriodId;
use crate::models::budget_period_store::BudgetPeriodStore;
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::{
    ActualItem, BankTransaction, BudgetPeriod, BudgetingTypeOverview, MatchRule, MonthBeginsOn,
};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        ItemAdded,
        ActualAdded,
        TransactionAdded,
        TransactionConnected,
        TransactionIgnored,
        ActualFundsReallocated,
        ActualFundsAdjusted,
        ItemModified,
        ActualModified,
        RuleAdded,
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_items: HashMap<Uuid, Arc<Mutex<BudgetItem>>>,
    budget_periods: BudgetPeriodStore,
    pub match_rules: HashSet<MatchRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
    pub currency: Currency,
}

impl Serialize for Budget {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Budget", 10)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("user_id", &self.user_id)?;
        let budget_item_hash: HashMap<String, BudgetItem> = self
            .budget_items
            .iter()
            .map(|(k, v)| (k.to_string(), v.lock().unwrap().clone()))
            .collect();
        state.serialize_field("budget_items", &budget_item_hash)?;
        
        let budget_period_hash: HashMap<String, BudgetPeriod> = self
            .budget_periods
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        state.serialize_field("budget_periods", &budget_period_hash)?;

        state.serialize_field("match_rules", &self.match_rules)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("default_budget", &self.default_budget)?;
        state.serialize_field("last_event", &self.last_event)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("currency", &self.currency)?;
        state.end()
    }
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
        self.budget_items.get(item_id)
    }

    pub fn list_ignored_transactions(&self, period_id: PeriodId) -> Vec<BankTransaction> {
        self.budget_periods.list_ignored_transactions(period_id)
    }

    pub fn month_begins_on(&self) -> &MonthBeginsOn {
        self.budget_periods.month_begins_on()
    }

    pub fn items_by_type(
        &self,
        budget_period_id: PeriodId,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)> {
        self.budget_periods.items_by_type(budget_period_id)
    }

    pub fn list_all_items(&self) -> Vec<BudgetItem> {
        self.budget_items.values().cloned().collect()
    }

    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType, period_id: PeriodId) -> Money {
        self.budget_periods
            .budgeted_for_type(budgeting_type, period_id)
    }

    pub fn spent_for_type(&self, budgeting_type: &BudgetingType, period_id: PeriodId) -> Money {
        self.budget_periods
            .spent_for_type(budgeting_type, period_id)
    }

    pub fn recalc_overview(&mut self, period_id: PeriodId) {
        self.budget_periods.recalc_overview(period_id);
    }

    pub fn insert_item(&mut self, item: &BudgetItem) {
        self.budget_items.insert(item.id, item.clone());
    }

    pub fn remove_item(&mut self, item_id: &Uuid) {
        self.budget_items.remove(item_id);
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

    pub fn get_period_for_transaction(&self, tx_id: &Uuid) -> Option<&BudgetPeriod> {
        self.budget_periods.get_period_for_transaction(tx_id)
    }

    pub fn contains_budget_item(&self, item_id: &Uuid) -> bool {
        self.budget_items.contains_key(item_id)
    }

    pub fn contains_item_with_name(&self, name: &str) -> bool {
        self.budget_items.values().any(|i| i.name == name)
    }

    pub fn get_transaction_mut(&mut self, tx_id: &Uuid) -> Option<&mut BankTransaction> {
        self.budget_periods.get_transaction_mut(tx_id)
    }

    pub fn get_transaction(&self, tx_id: &Uuid) -> Option<&BankTransaction> {
        self.budget_periods.get_transaction(tx_id)
    }

    pub fn modify_budget_item(
        &mut self,
        id: &Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
    ) {
        self.budget_items.entry(*id).and_modify(|item| {
            if let Some(mut item) = item.borrow_mut() {
                if let Some(name) = name {
                    item.name = name;
                }
                if let Some(item_type) = item_type {
                    item.item_type = item_type;
                }
            }
        });
    }

    pub fn get_budgeted_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Option<&Money> {
        self.budget_periods
            .get_budgeted_by_type(budgeting_type, period_id)
    }

    pub fn get_actual_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Option<&Money> {
        self.budget_periods
            .get_actual_by_type(budgeting_type, period_id)
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

    pub fn list_transactions_for_item(
        &self,
        item_id: &Uuid,
        sorted: bool,
    ) -> Vec<&BankTransaction> {
        self.budget_periods
            .list_transactions_for_item(item_id, sorted)
    }

    pub fn list_transactions_for_connection(
        &self,
        budget_period_id: Option<PeriodId>,
    ) -> Vec<BankTransaction> {
        self.budget_periods
            .list_transactions_for_connection(budget_period_id)
    }

    pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods.list_all_bank_transactions()
    }

    pub fn create_period_before(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        self.budget_periods.create_period_before(period_id)
    }

    pub fn create_period_after(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        self.budget_periods.create_period_after(period_id)
    }

    pub fn evaluate_rules(&self) -> Vec<(Uuid, Uuid)> {
        /* we must evaluate all transactions against all items for the BUDGET, not for
        a specific period.
         */
        self.budget_periods
            .evaluate_rules(&self.match_rules, &self.budget_items.list_all_items())
    }

    pub fn get_period_mut(&mut self, period_id: PeriodId) -> Option<&mut BudgetPeriod> {
        self.budget_periods.with_period_mut(period_id)
    }

    pub fn get_period(&self, period_id: PeriodId) -> Option<&BudgetPeriod> {
        self.budget_periods.with_period(period_id)
    }

    pub fn with_period_mut(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        //This can panic because we only use this in contexts where we KNOW we have a period!

        self.get_period_mut(period_id).unwrap()
    }

    pub fn with_period(&self, period_id: PeriodId) -> &BudgetPeriod {
        self.get_period(period_id).unwrap()
    }

    pub fn mutate_actual(
        &mut self,
        period_id: PeriodId,
        actual_id: Uuid,
        mutator: impl FnMut(&mut ActualItem),
    ) {
        self.with_period_mut(period_id)
            .mutate_actual(actual_id, mutator);
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
