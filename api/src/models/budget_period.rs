use crate::models::budget_period_id::PeriodId;
use crate::models::rule_packages::RulePackages;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{ActualItem, BankTransaction, BudgetItem, BudgetingType, MatchRule, Money, TransactionAllocation};
use crate::view_models::value_kind::ValueKind;
use core::fmt::Display;
use iter_tools::Itertools;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use uuid::Uuid;
use crate::view_models::BudgetingTypeOverview;

#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub tx_id: Uuid,
    pub amount: Money,
    pub actual_id: Option<Uuid>,
    pub item_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub id: PeriodId,
    pub actual_items: Vec<ActualItem>,
    pub transactions: Vec<BankTransaction>,
    #[serde(default)]
    pub allocations: Vec<TransactionAllocation>,
}

impl BudgetPeriod {
    pub fn mutate_actual(&mut self, actual_id: Uuid, mut mutator: impl FnMut(&mut ActualItem)) {
        if let Some(actual) = self.get_actual_mut(actual_id) {
            mutator(actual);
        }
    }

    pub fn update_actuals_from_item(&mut self, item: &BudgetItem) {
        for actual in self.actual_items.iter_mut() {
            if actual.budget_item_id == item.id {
                actual.item_name = item.name.clone();
                actual.budgeting_type = item.budgeting_type;
            }
        }
    }
    pub fn get_actual(&self, id: Uuid) -> Option<&ActualItem> {
        self.actual_items.iter().find(|i| i.id == id)
    }

    pub fn ignored_transactions(&self) -> Vec<&BankTransaction> {
        self.transactions.iter().filter(|t| t.ignored).collect()
    }

    pub fn items_by_type(
        &self,
        rules: &RulePackages,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<ActualItem>)> {
        self.actual_items
            .iter()
            .group_by(|item| item.budgeting_type)
            .into_iter()
            .enumerate()
            .map(|(index, (group, items))| {
                let overview = match group {
                    Income => self.get_income_overview(rules),
                    Expense => self.get_expense_overview(rules),
                    Savings => self.get_savings_overview(rules),
                };
                (index, group, overview, items.cloned().collect::<Vec<_>>())
            })
            .collect::<Vec<_>>()
    }

    pub fn budgeted_for_type(&self, budgeting_type: BudgetingType) -> Money {
        self.actual_items
            .iter()
            .filter(|item| item.budgeting_type == budgeting_type)
            .map(|item| item.budgeted_amount)
            .sum()
    }

    pub fn spent_for_type(&self, budgeting_type: BudgetingType) -> Money {
        let cost_types = [BudgetingType::Expense, BudgetingType::Savings, BudgetingType::InternalTransfer];
        self.actual_items
            .iter()
            .filter(|item| item.budgeting_type == budgeting_type)
            .map(|item| {
                let alloc_sum: Money = self
                    .allocations
                    .iter()
                    .filter(|a| a.actual_id == item.id)
                    .map(|a| if cost_types.contains(&item.budgeting_type) { -a.amount } else { a.amount })
                    .sum();
                if alloc_sum.is_zero() {
                    item.actual_amount
                } else {
                    alloc_sum
                }
            })
            .sum()
    }

    pub fn get_income_overview(
        &self,
        rules: &RulePackages,
    ) -> BudgetingTypeOverview {
        let rules = &rules
            .rule_packages
            .iter()
            .find(|p| p.budgeting_type == Income)
            .unwrap();
       
        let budgeted_income = rules
            .budgeted_rule
            .evaluate(&self.actual_items, Some(ValueKind::Budgeted));
        
        let spent_income = rules.actual_rule.evaluate(&self.actual_items, Some(ValueKind::Spent));
        
        let remaining_income = rules
            .remaining_rule
            .evaluate(&self.actual_items, Some(ValueKind::Budgeted));

        BudgetingTypeOverview {
            budgeting_type: Income,
            budgeted_amount: budgeted_income,
            actual_amount: spent_income,
            remaining_budget: remaining_income,
            is_ok: remaining_income == Money::zero(remaining_income.currency()),
        }
    }

    pub fn get_expense_overview(
        &self,
        rules: &RulePackages,
    ) -> BudgetingTypeOverview {
        let rules = &rules
            .rule_packages
            .iter()
            .find(|p| p.budgeting_type == Expense)
            .unwrap();
        let budgeted_expenses = rules
            .budgeted_rule
            .evaluate(&self.actual_items, Some(ValueKind::Budgeted));
        let spent_expenses = rules.actual_rule.evaluate(&self.actual_items, Some(ValueKind::Spent));
        let self_diff = rules.remaining_rule.evaluate(&self.actual_items, None);

        BudgetingTypeOverview {
            budgeting_type: Expense,
            budgeted_amount: budgeted_expenses,
            actual_amount: spent_expenses,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency()),
        }
    }

    pub fn get_savings_overview(
        &self,
        rules: &RulePackages,
    ) -> BudgetingTypeOverview {
        let rules = &rules
            .rule_packages
            .iter()
            .find(|p| p.budgeting_type == Savings)
            .unwrap();
        let budgeted_savings = rules
            .budgeted_rule
            .evaluate(&self.actual_items, Some(ValueKind::Budgeted));
        let spent_savings = rules.actual_rule.evaluate(&self.actual_items, Some(ValueKind::Spent));
        let self_diff = rules.remaining_rule.evaluate(&self.actual_items, None);

        BudgetingTypeOverview {
            budgeting_type: Savings,
            budgeted_amount: budgeted_savings,
            actual_amount: spent_savings,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency()),
        }
    }

    pub fn get_actual_mut(&mut self, id: Uuid) -> Option<&mut ActualItem> {
        self.actual_items.iter_mut().find(|i| i.id == id)
    }
    pub fn add_actual(&mut self, actual_item: ActualItem) {
        self.actual_items.push(actual_item);
    }
    pub fn contains_actual_for_item(&self, item_id: Uuid) -> bool {
        self.actual_items
            .iter()
            .any(|i| i.budget_item_id == item_id)
    }

    pub fn contains_actual(&self, actual_id: Uuid) -> bool {
        self.actual_items.iter().any(|i| i.id == actual_id)
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) {
        self.transactions.push(tx);
    }

    fn clear_hashmaps_and_transactions(&mut self) {
        self.transactions.clear();
    }
    pub fn clone_to(&self, id: PeriodId) -> Self {
        let mut period = self.clone();
        period.id = id;
        period.clear_hashmaps_and_transactions();
        period
    }
    pub fn new(id: PeriodId) -> Self {
        let mut period = Self {
            id,
            actual_items: Vec::new(),
            transactions: Vec::new(),
            allocations: Vec::new(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }

    pub fn add_allocation(&mut self, allocation: TransactionAllocation) {
        self.allocations.push(allocation);
    }

    pub fn remove_allocation(&mut self, allocation_id: Uuid) {
        self.allocations.retain(|a| a.id != allocation_id);
    }

    pub fn contains_allocation(&self, allocation_id: Uuid) -> bool {
        self.allocations.iter().any(|a| a.id == allocation_id)
    }

    pub fn allocations_for_transaction(&self, transaction_id: Uuid) -> Vec<&TransactionAllocation> {
        self.allocations
            .iter()
            .filter(|a| a.transaction_id == transaction_id)
            .collect()
    }

    pub fn allocations_for_actual(&self, actual_id: Uuid) -> Vec<&TransactionAllocation> {
        self.allocations
            .iter()
            .filter(|a| a.actual_id == actual_id)
            .collect()
    }

    pub fn allocated_amount_for_actual(&self, actual_id: Uuid) -> Money {
        self.allocations
            .iter()
            .filter(|a| a.actual_id == actual_id)
            .map(|a| a.amount)
            .sum()
    }

    pub fn evaluate_rules(
        &self,
        rules: &HashSet<MatchRule>,
        items: &[BudgetItem],
    ) -> Vec<RuleMatch> {
        let mut matched = Vec::new();
        let actuals = self.actual_items.iter().collect::<Vec<_>>();
        for transaction in self.transactions.iter().filter(|tx| {
            !tx.ignored
                && tx.actual_id.is_none()
                && !self.allocations.iter().any(|a| a.transaction_id == tx.id)
        }) {
            for rule in rules {
                if rule.matches_transaction(transaction) {
                    if let Some(actual_id) = self.get_actual_id_for_rule(rule, &actuals) {
                        matched.push(RuleMatch {
                            tx_id: transaction.id,
                            amount: transaction.amount,
                            actual_id: Some(actual_id),
                            item_id: None,
                        });
                        break;
                    } else if let Some(item_id) = self.get_item_id_for_rule(rule, items) {
                        matched.push(RuleMatch {
                            tx_id: transaction.id,
                            amount: transaction.amount,
                            actual_id: None,
                            item_id: Some(item_id),
                        });
                        break;
                    }
                }
            }
        }
        matched
    }

    pub fn get_actual_id_for_rule(
        &self,
        rule: &MatchRule,
        actuals: &Vec<&ActualItem>,
    ) -> Option<Uuid> {
        actuals
            .iter()
            .find(|i| rule.matches_actual(i))
            .map(|i| i.id)
    }

    pub fn transactions_for_actual(&self, actual_id: Uuid, sorted: bool) -> Vec<&BankTransaction> {
        let mut transactions = self.transactions
            .iter()
            .filter(|i| i.actual_id == Some(actual_id))
            .collect::<Vec<_>>();
        if sorted {
            transactions.sort_by_key(|tx| tx.date);
        }
        transactions
    }

    pub fn get_item_id_for_rule(&self, rule: &MatchRule, items: &[BudgetItem]) -> Option<Uuid> {
        items.iter().find(|i| rule.matches_item(i)).map(|i| i.id)
    }

    pub fn all_actuals(&self) -> Vec<&ActualItem> {
        self.actual_items.iter().collect()
    }
}
