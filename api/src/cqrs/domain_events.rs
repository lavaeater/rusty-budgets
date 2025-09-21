use crate::cqrs::budget::{
    get_transaction_hash, BankTransaction, Budget, BudgetGroup, BudgetItem, BudgetingType,
};
use crate::cqrs::framework::DomainEvent;
use crate::cqrs::framework::{Aggregate, CommandError};
use crate::cqrs::money::{Currency, Money};
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
    pub currency: Currency,
}

impl BudgetCreatedHandler for Budget {
    fn apply_create_budget(&mut self, event: &BudgetCreated) -> Uuid {
        self.id = event.budget_id;
        self.name = event.name.clone();
        self.user_id = event.user_id;
        self.default_budget = event.default_budget;
        self.currency = event.currency;
        self.id
    }

    fn create_budget_impl(
        &self,
        name: String,
        user_id: Uuid,
        default_budget: bool,
        currency: Currency
    ) -> Result<BudgetCreated, CommandError> {
        if self.version == 0 && self.last_event == 0 {
            Ok(BudgetCreated {
                budget_id: self.id,
                name,
                user_id,
                default_budget,
                currency
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
    pub group_type: BudgetingType,
}

impl GroupAddedHandler for Budget {
    fn apply_add_group(&mut self, event: &GroupAdded)-> Uuid {
        self.budget_groups.insert(
            event.group_id,
            BudgetGroup::new(event.group_id, &event.name, event.group_type, self.currency),
        );
        event.group_id
    }
    
    fn add_group_impl(
        &self,
        group_id: Uuid,
        name: String,
        group_type: BudgetingType,
    ) -> Result<GroupAdded, CommandError> {
        if self.budget_groups.contains_key(&group_id)
            || self.budget_groups.values().any(|g| g.name == name)
        {
            Err(CommandError::Validation("Budget group already exists"))
        } else {
            Ok(GroupAdded {
                budget_id: self.id,
                group_id,
                name,
                group_type,
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
    pub item_type: BudgetingType,
    pub budgeted_amount: Money,
}

impl Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) {
        let new_item = BudgetItem::new(
            &event.name,
            event.item_type,
            event.budgeted_amount,
            None,
            None,
        );
        let new_item_id = new_item.id;
        _ = self.budget_groups.get_mut(&event.group_id).map(|f| {
            f.items.push(new_item);
            f.budgeted_amount += event.budgeted_amount;
            self.total_by_type.entry(event.item_type).and_modify(|v| {
                *v += event.budgeted_amount;
            }).or_insert(event.budgeted_amount);
            Some(f)
        });
        self.budget_items_and_groups
            .insert(new_item_id, event.group_id);
    }

    fn add_item_impl(
        &self,
        group_id: Uuid,
        name: String,
        item_type: BudgetingType,
        budgeted_amount: Money,
    ) -> Result<ItemAdded, CommandError> {
        if self.budget_groups.contains_key(&group_id) {
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
        let hash = get_transaction_hash(&amount, &balance, &account_number, &description, &date);

        if self.bank_transactions.can_insert(&hash) {
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

// TransactionConnected,
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionConnected {
    budget_id: Uuid,
    tx_id: Uuid,
    item_id: Uuid,
}

impl TransactionConnectedHandler for Budget {
    fn apply_do_transaction_connected(&mut self, event: &TransactionConnected) {
        // Connect transaction to item
        let tx = self.bank_transactions
            .get_mut(&event.tx_id)
            .unwrap();
            tx.budget_item_id = Some(event.item_id);
        let group_id = self.budget_items_and_groups.get(&event.item_id).unwrap();
        // Update group
        let group = self.budget_groups.get_mut(&group_id).unwrap();
        group.actual_spent += tx.amount;
        
        //Update budget total
        self.total_by_type.entry(group.group_type).and_modify(|v| {
            *v += tx.amount;
        }).or_insert(tx.amount);
        // Update item
        let item = group.items.iter_mut().find(|item| item.id == event.item_id).unwrap();
        item.actual_spent += tx.amount;
    }

    fn do_transaction_connected_impl(
        &self,
        tx_id: Uuid,
        item_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if self.bank_transactions.contains(&tx_id)
            && self.budget_items_and_groups.contains_key(&item_id)
        {
            let ev = TransactionConnected {
                budget_id: self.id,
                tx_id,
                item_id,
            };
            Ok(ev)
        } else {
            Err(CommandError::Validation(
                "Transaction or item does not exist.",
            ))
        }
    }
}

// FundsReallocated
