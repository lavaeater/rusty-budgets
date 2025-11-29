use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::DomainEvent;
use crate::events::*;
use crate::models::budget_item::BudgetItem;
use crate::models::budget_period_id::PeriodId;
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::rule_packages::RulePackages;
use crate::models::{ActualItem, BankTransaction, BudgetPeriod, MatchRule, MonthBeginsOn};
use crate::pub_events_enum;
use crate::view_models::BudgetingTypeOverview;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
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
        BudgetedFundsReallocated,
        ActualBudgetedFundsAdjusted,
        ItemModified,
        ActualModified,
        RuleAdded,
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub items: Vec<BudgetItem>,
    pub month_begins_on: MonthBeginsOn,
    pub periods: Vec<BudgetPeriod>,
    #[serde(default)]
    pub rules: RulePackages,
    pub transaction_hashes: HashSet<u64>,
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
            periods: Default::default(),
            rules: Default::default(),
            items: Default::default(),
            match_rules: HashSet::default(),
            created_at: Default::default(),
            updated_at: Default::default(),
            default_budget: false,
            last_event: 0,
            version: 0,
            currency: Default::default(),
            month_begins_on: Default::default(),
            transaction_hashes: Default::default(),
        }
    }
}

impl Budget {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            periods: vec![BudgetPeriod::new(PeriodId::from_date(
                Utc::now(),
                MonthBeginsOn::default(),
            ))],
            ..Default::default()
        }
    }

    pub fn get_item(&self, item_id: Uuid) -> Option<&BudgetItem> {
        self.items.iter().find(|item| item.id == item_id)
    }

    pub fn get_item_mut(&mut self, item_id: Uuid) -> Option<&mut BudgetItem> {
        self.items.iter_mut().find(|item| item.id == item_id)
    }

    pub fn get_period(&self, period_id: PeriodId) -> Option<&BudgetPeriod> {
        self.periods.iter().find(|period| period.id == period_id)
    }

    pub fn all_actuals(&self, period_id: PeriodId) -> Vec<&ActualItem> {
        self.get_period(period_id).map(|p|p.all_actuals()).unwrap_or_default()
    }

    pub fn ignored_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| p.ignored_transactions())
            .unwrap_or_default()
    }

    pub fn month_begins_on(&self) -> MonthBeginsOn {
        self.month_begins_on
    }

    pub fn items_by_type(
        &self,
        budget_period_id: PeriodId,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<ActualItem>)> {
        self.get_period(budget_period_id)
            .map(|p| p.items_by_type(&self.rules))
            .unwrap_or_default()
    }

    pub fn all_items(&self) -> &Vec<BudgetItem> {
        &self.items
    }

    pub fn budgeted_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.get_period(period_id)
            .map(|p| p.budgeted_for_type(budgeting_type))
            .unwrap_or_default()
    }

    pub fn spent_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.get_period(period_id)
            .map(|p| p.spent_for_type(budgeting_type))
            .unwrap_or_default()
    }

    pub fn insert_item(&mut self, item: BudgetItem) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, item_id: Uuid) {
        self.items.retain(|item| item.id != item_id);
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) -> bool {
        if self.transaction_hashes.insert(tx.get_hash()) {
            let period_id = PeriodId::from_date(tx.date, self.month_begins_on);
            self.with_period_mut(period_id).insert_transaction(tx);
            true
        } else {
            false
        }
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        !self.transaction_hashes.contains(tx_hash)
    }

    pub fn contains_transaction(&self, tx_id: Uuid) -> bool {
        self.periods
            .iter()
            .any(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn get_period_for_transaction(&self, tx_id: Uuid) -> Option<&BudgetPeriod> {
        self.periods
            .iter()
            .find(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn get_period_for_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BudgetPeriod> {
        self.periods
            .iter_mut()
            .find(|p| p.transactions.iter().any(|t| t.id == tx_id))
    }

    pub fn contains_budget_item(&self, item_id: Uuid) -> bool {
        self.items.iter().any(|i| i.id == item_id)
    }

    pub fn contains_item_with_name(&self, name: &str) -> bool {
        self.items.iter().any(|i| i.name == name)
    }

    pub fn get_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BankTransaction> {
        self.get_period_for_transaction_mut(tx_id)
            .and_then(|p| p.transactions.iter_mut().find(|t| t.id == tx_id))
    }

    pub fn get_transaction(&self, tx_id: Uuid) -> Option<&BankTransaction> {
        self.get_period_for_transaction(tx_id)
            .and_then(|p| p.transactions.iter().find(|t| t.id == tx_id))
    }

    pub fn update_actuals_for_item(&mut self, item_id: Uuid) {
        if let Some(item) = self.get_item(item_id).cloned() {
            self.periods
                .iter_mut()
                .for_each(|p| p.update_actuals_from_item(&item));
        }
    }

    pub fn modify_budget_item(
        &mut self,
        id: Uuid,
        name: Option<String>,
        budgeting_type: Option<BudgetingType>,
    ) {
        let mut was_updated = false;
        self.items
            .iter_mut()
            .find(|item| item.id == id)
            .map(|item| {
                if let Some(name) = name {
                    item.name = name;
                }
                if let Some(item_type) = budgeting_type {
                    item.budgeting_type = item_type;
                }
                was_updated = true;
            });
        if was_updated {
            self.update_actuals_for_item(id);
        }
    }

    pub fn get_budgeted_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Money {
        match self.get_period(period_id) {
            Some(p) => p.budgeted_for_type(*budgeting_type),
            None => Money::zero(self.currency),
        }
    }

    pub fn get_actual_by_type(&self, budgeting_type: &BudgetingType, period_id: PeriodId) -> Money {
        match self.get_period(period_id) {
            Some(p) => p.spent_for_type(*budgeting_type),
            None => Money::zero(self.currency),
        }
    }

    pub fn get_budgeting_overview(
        &self,
        budgeting_type: BudgetingType,
        period_id: PeriodId,
    ) -> BudgetingTypeOverview {
        match budgeting_type {
            BudgetingType::Expense => self
                .get_period(period_id)
                .map(|p| p.get_expense_overview(&self.rules))
                .unwrap_or_default(),
            BudgetingType::Income => self
                .get_period(period_id)
                .map(|p| p.get_income_overview(&self.rules))
                .unwrap_or_default(),
            BudgetingType::Savings => self
                .get_period(period_id)
                .map(|p| p.get_savings_overview(&self.rules))
                .unwrap_or_default(),
        }
    }

    pub fn set_transaction_ignored(&mut self, tx_id: Uuid) -> bool {
        match self.get_transaction_mut(tx_id) {
            Some(tx) => {
                if tx.ignored {
                    return false;
                }
                tx.ignored = true;
                true
            }
            None => false,
        }
    }

    pub fn transactions_for_actual(
        &self,
        period_id: PeriodId,
        actual_id: Uuid,
        sorted: bool,
    ) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| p.transactions_for_actual(actual_id, sorted))
            .unwrap_or_default()
    }

    pub fn unconnected_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| {
                p.transactions
                    .iter()
                    .filter(|t| t.actual_id.is_none() && !t.ignored)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn connected_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.get_period(period_id)
            .map(|p| {
                p.transactions
                    .iter()
                    .filter(|t| t.actual_id.is_some())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn all_transactions(&self) -> Vec<&BankTransaction> {
        self.periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .collect()
    }

    pub fn all_transactions_mut(&mut self) -> Vec<&mut BankTransaction> {
        self.periods
            .iter_mut()
            .flat_map(|p| p.transactions.iter_mut())
            .collect()
    }

    pub fn create_period_before(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period_id = period_id.month_before();
        self.get_or_create_period(period_id)
    }

    pub fn get_period_before(&self, id: PeriodId) -> Option<&BudgetPeriod> {
        if self.periods.is_empty() {
            return None;
        }
        self.periods
            .iter()
            .map(|p| p.id)
            .filter(|key| key < &id)
            .max()
            .map(|key| self.periods.iter().find(|p| p.id == key).unwrap())
    }

    fn get_or_create_period(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let idx = if let Some(idx) = self.periods.iter().position(|p| p.id == period_id) {
            idx
        } else {
            let previous_period = self.get_period_before(period_id);
            let period = if let Some(previous_period) = previous_period {
                previous_period.clone_to(period_id)
            } else {
                BudgetPeriod::new(period_id)
            };
            self.periods.push(period);
            self.periods.len() - 1
        };
        &mut self.periods[idx]
    }

    pub fn create_period_after(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period_id = period_id.month_after();
        self.get_or_create_period(period_id)
    }

    pub fn evaluate_rules(&self) -> Vec<(Uuid, Option<Uuid>, Option<Uuid>)> {
        self.periods
            .iter()
            .flat_map(|p| p.evaluate_rules(&self.match_rules, &self.items))
            .collect::<Vec<_>>()
    }

    pub fn contains_period(&self, period_id: PeriodId) -> bool {
        self.periods.iter().any(|p| p.id == period_id)
    }

    fn ensure_period(&mut self, period_id: PeriodId) -> usize {
        if let Some(idx) = self.periods.iter().position(|p| p.id == period_id) {
            idx
        } else {
            self.periods.push(BudgetPeriod::new(period_id));
            self.periods.len() - 1
        }
    }

    pub fn with_period_mut(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let idx = self.ensure_period(period_id);
        &mut self.periods[idx]
    }

    pub fn with_period(&mut self, period_id: PeriodId) -> &BudgetPeriod {
        let idx = self.ensure_period(period_id);
        &self.periods[idx]
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
