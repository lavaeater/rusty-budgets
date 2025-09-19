use crate::cqrs::budget::{BankTransaction, Budget, BudgetGroup, BudgetItem, BudgetItemType};
use crate::cqrs::framework::DomainEvent;
use crate::cqrs::framework::{Aggregate, CommandError};
use crate::cqrs::money::Money;
use chrono::{DateTime, Utc};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default_budget: bool,
}

impl Budget {
    fn apply_create_budget(&mut self, event: &BudgetCreated) {
        self.id = event.budget_id;
        self.name = event.name.clone();
        self.user_id = event.user_id;
        self.default_budget = event.default_budget;
    }

    fn create_budget_impl(
        &self,
        name: String,
        user_id: Uuid,
        default: bool,
    ) -> Result<BudgetCreated, CommandError> {
        if self.version == 0 && self.last_event == 0 {
            Ok(BudgetCreated {
                budget_id: self.id,
                name,
                user_id,
                default_budget: default,
            })
        } else {
            Err(CommandError::Validation("Budget already exists"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct GroupAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
}

impl Budget {
    fn apply_add_group(&mut self, event: &GroupAdded) {
        self.budget_groups.insert(
            event.group_id,
            BudgetGroup::new(event.group_id, &event.name),
        );
    }

    fn add_group_impl(&self, group_id: Uuid, name: String) -> Result<GroupAdded, CommandError> {
        if self.budget_groups.contains_key(&group_id)
            || self.budget_groups.values().any(|g| g.name == name)
        {
            Err(CommandError::Validation("Budget group already exists"))
        } else {
            Ok(GroupAdded {
                budget_id: self.id,
                group_id,
                name,
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: Money,
}

impl Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) {
        _ = self.budget_groups.get_mut(&event.group_id).map(|f| {
            f.items.push(BudgetItem::new(
                &event.name,
                event.item_type,
                event.budgeted_amount,
                None,
                None,
            ));
            Some(f)
        });
    }

    fn add_item_impl(
        &self,
        group_id: Uuid,
        name: String,
        item_type: BudgetItemType,
        budgeted_amount: Money,
    ) -> Result<ItemAdded, CommandError> {
        if self.budget_groups.contains_key(&group_id) {
            //&& group.items.iter().find(|item| item.name == name).is_none()
            Ok(ItemAdded {
                budget_id: self.id,
                group_id,
                name,
                item_type,
                budgeted_amount,
            })
        } else {
            Err(CommandError::Validation("Budget group does not exist"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionAdded {
    pub budget_id: Uuid,
    pub transaction_id: Uuid,
    pub account_number: String,
    pub amount: Money,
    pub balance: Money,
    pub description: String,
    pub date: DateTime<Utc>,
}

impl Hash for TransactionAdded {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.amount.hash(state);
        self.account_number.hash(state);
        self.description.hash(state);
        self.date.hash(state);
    }
}

impl Budget {
    fn apply_add_transaction(&mut self, event: &TransactionAdded) {
        self.bank_transactions.insert(BankTransaction::new(
            event.transaction_id,
            &event.account_number,
            event.amount,
            event.balance,
            &event.description,
            event.date,
        ));
    }

    fn add_transaction_impl(
        &self,
        transaction_id: Uuid,
        account_number: String,
        amount: Money,
        balance: Money,
        description: String,
        date: DateTime<Utc>,
    ) -> Result<TransactionAdded, CommandError> {
        let bt = BankTransaction::new(transaction_id, &account_number, amount, balance, &description, date);
        if !self.bank_transactions.contains(&bt) {
            Ok(TransactionAdded {
                budget_id: self.id,
                account_number,
                transaction_id,
                amount,
                balance,
                description,
                date,
            })
        } else {
            Err(CommandError::Validation("Transaction already exists."))
        }
    }
}

// TransactionAdded,
// TransactionConnected,
// FundsReallocated
