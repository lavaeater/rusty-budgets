use crate::cqrs::budget::{
    get_transaction_hash, BankTransaction, Budget, BudgetGroup, BudgetItem, BudgetingType,
};
use crate::cqrs::framework::DomainEvent;
use crate::cqrs::framework::{Aggregate, CommandError};
use crate::cqrs::money::{Currency, Money};
use chrono::{DateTime, Utc};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BudgetCreated {
    #[event_id]
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
        currency: Currency,
    ) -> Result<BudgetCreated, CommandError> {
        if self.version == 0 && self.last_event == 0 {
            Ok(BudgetCreated {
                budget_id: Uuid::new_v4(),
                name,
                user_id,
                default_budget,
                currency,
            })
        } else {
            Err(CommandError::Validation("Budget already exists"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub item_id: Uuid,
    pub name: String,
    pub item_type: BudgetingType,
    pub budgeted_amount: Money,
}

impl ItemAddedHandler for Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) -> Uuid {
        let new_item = BudgetItem::new(
            event.item_id,
            &event.name,
            event.budgeted_amount,
            None,
            None,
        );
        let new_item_id = new_item.id;
        self.budget_items.insert(&new_item, event.item_type);
        self.budgeted_by_type
            .entry(event.item_type)
            .and_modify(|v| {
                *v += event.budgeted_amount;
            })
            .or_insert(event.budgeted_amount);

        new_item_id
    }

    fn add_item_impl(
        &self,
        name: String,
        item_type: BudgetingType,
        budgeted_amount: Money,
    ) -> Result<ItemAdded, CommandError> {
        Ok(ItemAdded {
            budget_id: self.id,
            item_id: Uuid::new_v4(),
            name,
            item_type,
            budgeted_amount,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionAdded {
    pub budget_id: Uuid,
    #[event_id]
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

impl TransactionAddedHandler for Budget {
    fn apply_add_transaction(&mut self, event: &TransactionAdded) -> Uuid {
        self.bank_transactions.insert(BankTransaction::new(
            event.transaction_id,
            &event.account_number,
            event.amount,
            event.balance,
            &event.description,
            event.date,
        ));
        event.transaction_id
    }

    fn add_transaction_impl(
        &self,
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
                transaction_id: Uuid::new_v4(),
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

impl Display for TransactionConnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransactionConnected {{ budget_id: {}, tx_id: {}, item_id: {} }}",
            self.budget_id, self.tx_id, self.item_id
        )
    }
}

impl TransactionConnectedHandler for Budget {
    fn apply_connect_transaction(&mut self, event: &TransactionConnected) -> Uuid {
        println!("Applying transaction connected event: {}", event);
        // Connect transaction to item
        let tx = self.bank_transactions.get_mut(&event.tx_id).unwrap();

        if tx.budget_item_id.is_some() {
            println!("Transaction: {}", tx);
            println!(
                "Transaction {} is already connected to item {}",
                event.tx_id,
                tx.budget_item_id.unwrap()
            );
            let previous_budget_item_id = tx.budget_item_id.unwrap();
            println!("Previous budget item id: {}", previous_budget_item_id);
            let previous_budgeting_type = self
                .budget_items.type_for(&previous_budget_item_id)
                .unwrap();
            println!("Previous type id: {}", previous_budgeting_type);
            
            //Update budget total
            self.spent_by_type
                .entry(*previous_budgeting_type)
                .and_modify(|v| {
                    *v -= tx.amount;
                });

            let previous_item = self.budget_items.get_mut(&previous_budget_item_id).unwrap();
            previous_item.actual_spent -= tx.amount;
        }
        tx.budget_item_id = Some(event.item_id);
        // Update group
        let budgeting_type = self.budget_items.type_for(&event.item_id).unwrap();
       
        //Update budget total
        self.spent_by_type
            .entry(*budgeting_type)
            .and_modify(|v| {
                *v += tx.amount;
            })
            .or_insert(tx.amount);
        // Update item
        let item = self.budget_items.get_mut(&event.item_id).unwrap();
        item.actual_spent += tx.amount;
        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        item_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if self.bank_transactions.contains(&tx_id)
            && self.budget_items.contains(&item_id)
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
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemFundsReallocated {
    budget_id: Uuid,
    from_item_id: Uuid,
    to_item_id: Uuid,
    amount: Money,
}

impl ItemFundsReallocatedHandler for Budget {
    fn apply_reallocate_item_funds(&mut self, event: &ItemFundsReallocated) -> Uuid {
        let from_item = self.get_item_mut(&event.from_item_id).unwrap();
        from_item.budgeted_amount -= event.amount;
        
        let from_type = self.budget_items.type_for(&event.from_item_id).unwrap();
        self.budgeted_by_type
            .entry(*from_type)
            .and_modify(|v| {
                *v -= event.amount;
            }).or_insert(-event.amount);

        
        let to_item = self.get_item_mut(&event.to_item_id).unwrap();
        to_item.budgeted_amount += event.amount;

        let to_type = self.budget_items.type_for(&event.to_item_id).unwrap();
        self.budgeted_by_type
            .entry(*to_type)
            .and_modify(|v| {
                *v += event.amount;
            }).or_insert(event.amount);
        
        event.from_item_id
    }

    fn reallocate_item_funds_impl(
        &self,
        from_item_id: Uuid,
        to_item_id: Uuid,
        amount: Money,
    ) -> Result<ItemFundsReallocated, CommandError> {
        /*
        Re-allocations of funds are only allowed if both items are of
        budget item type expense OR savings - income cannot be reallocated, only modified.
         */
        let from_item = self.budget_items.get(&from_item_id);
        let to_item = self.budget_items.get(&to_item_id);

        if from_item.is_none() || to_item.is_none() {
            return Err(CommandError::Validation(
                "Either Item to take funds from or Item to deliver funds to does not exist.",
            ));
        }
        let from_type = self.budget_items.type_for(&from_item_id).unwrap();
        let to_type = self.budget_items.type_for(&to_item_id).unwrap();

        if from_type == &BudgetingType::Income
            || to_type == &BudgetingType::Income
        {
            return Err(CommandError::Validation("Re-allocations of funds are only allowed if both items are of budget item type expense OR savings - income cannot be reallocated, only modified."));
        }

        let from_item = from_item.unwrap();

        if from_item.budgeted_amount < amount {
            return Err(CommandError::Validation(
                "Item to take funds from does not have enough budgeted amount.",
            ));
        }

        Ok(ItemFundsReallocated {
            budget_id: self.id,
            from_item_id,
            to_item_id,
            amount,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemFundsAdjusted {
    budget_id: Uuid,
    item_id: Uuid,
    amount: Money,
}

impl ItemFundsAdjustedHandler for Budget {
    fn apply_adjust_item_funds(&mut self, event: &ItemFundsAdjusted) -> Uuid {
        let item = self.get_item_mut(&event.item_id).unwrap();
        item.budgeted_amount += event.amount;
        let item_type = self.budget_items.type_for(&event.item_id).unwrap();
        self.budgeted_by_type.entry(*item_type)
            .and_modify(|v|
                {*v += event.amount})
            .or_insert(event.amount);
        event.item_id
    }

    fn adjust_item_funds_impl(
        &self,
        item_id: Uuid,
        amount: Money,
    ) -> Result<ItemFundsAdjusted, CommandError> {
        let item = self.get_item(&item_id);

        if item.is_none() {
            return Err(CommandError::Validation("Item does not exist"));
        }
        let item = item.unwrap();

        if (item.budgeted_amount + amount) < Money::default() {
            return Err(CommandError::Validation(
                "Items are not allowed to be less than zero.",
            ));
        }

        Ok(ItemFundsAdjusted {
            budget_id: self.id,
            item_id,
            amount,
        })
    }
}
