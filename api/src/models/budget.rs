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
use crate::models::budgeting_type::BudgetingType;
use crate::models::money::{Currency, Money};
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};
use crate::models::{BankTransaction, BudgetingTypeOverview, ValueKind};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
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
    budget_items: BudgetItemStore,
    bank_transactions: BankTransactionStore,
    budgeted_by_type: HashMap<BudgetingType, Money>,
    actual_by_type: HashMap<BudgetingType, Money>,
    budgeting_overview: HashMap<BudgetingType, BudgetingTypeOverview>,
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
            budget_items: Default::default(),
            bank_transactions: Default::default(),
            created_at: Default::default(),
            updated_at: Default::default(),
            default_budget: false,
            last_event: 0,
            version: 0,
            currency: Default::default(),
            budgeting_overview: HashMap::from([
                (BudgetingType::Expense, BudgetingTypeOverview::default()),
                (BudgetingType::Savings, BudgetingTypeOverview::default()),
                (BudgetingType::Income, BudgetingTypeOverview::default()),
            ]),
            budgeted_by_type: HashMap::from([
                (BudgetingType::Expense, Money::default()),
                (BudgetingType::Savings, Money::default()),
                (BudgetingType::Income, Money::default()),
            ]),
            actual_by_type: HashMap::from([
                (BudgetingType::Expense, Money::default()),
                (BudgetingType::Savings, Money::default()),
                (BudgetingType::Income, Money::default()),
            ]),
        }
    }
}

impl Budget {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self.budget_items.get(item_id)
    }

    pub fn get_type_for_item(&self, item_id: &Uuid) -> Option<&BudgetingType> {
        self.budget_items.type_for(item_id)
    }

    pub fn items_by_type(
        &self,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)> {
        self.budget_items
            .items_by_type()
            .iter()
            .map(|(index, t, items)| {
                let overview = self.budgeting_overview.get(t).unwrap();
                (*index, *t, *overview, items.clone())
            })
            .collect::<Vec<_>>()
    }

    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_items
            .by_type(budgeting_type)
            .unwrap_or_default()
            .iter()
            .map(|item| item.budgeted_amount)
            .sum()
    }

    pub fn spent_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_items
            .by_type(budgeting_type)
            .unwrap_or_default()
            .iter()
            .map(|item| item.actual_amount)
            .sum()
    }

    pub fn recalc_overview(&mut self) {
        let income_sum = Sum(vec![Income]);
        let budgeted_income =
            income_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Budgeted));
        let spent_income =
            income_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Spent));
        let remaining_rule = Difference(Income, vec![Expense, Savings]);
        let remaining_income =
            remaining_rule.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Budgeted));

        let income_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_income,
            actual_amount: spent_income,
            remaining_budget: remaining_income,
        };

        self.budgeting_overview.insert(Income, income_overview);

        let expense_sum = Sum(vec![Expense]);
        let budgeted_expenses =
            expense_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Budgeted));
        let spent_expenses =
            expense_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Spent));

        let self_difference_rule = SelfDiff(Expense);
        let self_diff = self_difference_rule.evaluate(&self.budget_items.hash_by_type(), None);

        let expense_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_expenses,
            actual_amount: spent_expenses,
            remaining_budget: self_diff,
        };

        self.budgeting_overview.insert(Expense, expense_overview);

        let savings_sum = Sum(vec![Savings]);
        let budgeted_savings =
            savings_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Budgeted));
        let spent_savings =
            savings_sum.evaluate(&self.budget_items.hash_by_type(), Some(ValueKind::Spent));

        let self_difference_rule = SelfDiff(Savings);
        let self_diff = self_difference_rule.evaluate(&self.budget_items.hash_by_type(), None);

        let savings_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_savings,
            actual_amount: spent_savings,
            remaining_budget: self_diff,
        };

        self.budgeting_overview.insert(Savings, savings_overview);
    }

    pub fn insert_item(&mut self, item: &BudgetItem, item_type: BudgetingType) {
        self.budget_items.insert(item, item_type);
        self.budgeted_by_type
            .entry(item_type)
            .and_modify(|v| *v += item.budgeted_amount)
            .or_insert(item.budgeted_amount);
        self.recalc_overview();
    }

    pub fn remove_item(&mut self, item_id: &Uuid) {
        if let Some((item, item_type)) = self.budget_items.remove(*item_id) {
            self.budgeted_by_type
                .entry(item_type)
                .and_modify(|v| *v -= item.budgeted_amount);
            self.recalc_overview();
        }
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) {
        self.bank_transactions.insert(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.bank_transactions.can_insert(tx_hash)
    }

    pub fn contains_transaction(&self, tx_id: &Uuid) -> bool {
        self.bank_transactions.contains(tx_id)
    }

    pub fn contains_budget_item(&self, item_id: &Uuid) -> bool {
        self.budget_items.contains(item_id)
    }

    pub fn get_transaction_mut(&mut self, tx_id: &Uuid) -> Option<&mut BankTransaction> {
        self.bank_transactions.get_mut(tx_id)
    }

    pub fn get_transaction(&self, tx_id: &Uuid) -> Option<&BankTransaction> {
        self.bank_transactions.get(tx_id)
    }

    pub fn type_for_item(&self, item_id: &Uuid) -> Option<BudgetingType> {
        self.budget_items.type_for(item_id).cloned()
    }

    pub fn update_budget_actual_amount(&mut self, budgeting_type: &BudgetingType, amount: &Money) {
        self.actual_by_type.entry(*budgeting_type).and_modify(|v| {
            *v += *amount;
        });
    }

    pub fn update_budget_budgeted_amount(
        &mut self,
        budgeting_type: &BudgetingType,
        amount: &Money,
    ) {
        self.budgeted_by_type
            .entry(*budgeting_type)
            .and_modify(|v| {
                *v += *amount;
            });
    }

    pub fn add_actual_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_items.add_actual_amount(item_id, amount);
    }

    pub fn add_budgeted_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_items.add_budgeted_amount(item_id, amount);
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
        self.budget_items.modify_item(
            id,
            name,
            item_type,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        );
    }
    
    pub fn get_budgeted_by_type(&self, budgeting_type: &BudgetingType)-> Option<&Money> {
        self.budgeted_by_type.get(budgeting_type)
    }
    
    pub fn get_actual_by_type(&self, budgeting_type: &BudgetingType)-> Option<&Money> {
        self.actual_by_type.get(budgeting_type) 
    }
    
    pub fn get_budgeting_overview(&self, budgeting_type: &BudgetingType)-> Option<&BudgetingTypeOverview> {
        self
            .budgeting_overview
            .get(budgeting_type)
    }
    
    pub fn list_bank_transactions(&self) -> Vec<&BankTransaction>{
        self.bank_transactions.list_transactions()
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
