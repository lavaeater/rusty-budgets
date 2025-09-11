use crate::cqrs::budget::Budget;
use crate::cqrs::domain_events::{BudgetCreated, GroupAdded};
use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::{DomainEvent, Runtime};
use crate::pub_events_enum;
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;
use uuid::Uuid;
use crate::runtime::JoyDbBudgetRuntime;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        GroupAdded,
        // ItemAdded,
        // TransactionAdded,
        // TransactionConnected,
        // FundsReallocated
        // ... add other events here
    }
}

//


//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct GroupAdded {
//     pub budget_id: Uuid,
//     pub group_id: Uuid,
//     pub name: String,
// }
//
// impl DomainEvent<Budget> for GroupAdded {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         self.budget_id
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         state.budget_groups.insert(
//             self.group_id,
//             BudgetGroup {
//                 id: self.group_id,
//                 name: self.name.clone(),
//                 items: Vec::default(),
//             },
//         );
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ItemAdded {
//     pub budget_id: Uuid,
//     pub group_id: Uuid,
//     pub item: BudgetItem,
// }
//
// impl DomainEvent<Budget> for ItemAdded {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         self.budget_id
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         if let Some(group) = state.budget_groups.get_mut(&self.group_id) {
//             group.items.push(self.item.clone());
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TransactionAdded {
//     budget_id: Uuid,
//     tx: BankTransaction,
// }
//
// impl DomainEvent<Budget> for TransactionAdded {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         self.budget_id
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         state.bank_transactions.insert(self.tx.id, self.tx.clone());
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TransactionConnected {
//     budget_id: Uuid,
//     tx_id: Uuid,
//     item_id: Uuid,
// }
//
// impl TransactionConnected {
//     pub fn new(budget_id: Uuid, tx_id: Uuid, item_id: Uuid) -> Self {
//         Self {
//             budget_id,
//             tx_id,
//             item_id,
//         }
//     }
// }
//
// impl DomainEvent<Budget> for TransactionConnected {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         self.budget_id
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         if let Some(tx) = state.bank_transactions.get_mut(&self.tx_id) {
//             tx.budget_item_id = Some(self.item_id);
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct FundsReallocated {
//     budget_id: Uuid,
//     from_item: Uuid,
//     to_item: Uuid,
//     amount: Money,
// }
//
// impl SubAssign for Money {
//     fn sub_assign(&mut self, rhs: Self) {
//         self.cents -= rhs.cents;
//     }
// }
//
// impl AddAssign for Money {
//     fn add_assign(&mut self, rhs: Self) {
//         self.cents += rhs.cents;
//     }
// }
//
// impl DomainEvent<Budget> for FundsReallocated {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         self.budget_id
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         if let Some(from_item) = state.get_item_mut(&self.from_item) {
//             from_item.budgeted_amount -= self.amount;
//         }
//         if let Some(to_item) = state.get_item_mut(&self.to_item) {
//             to_item.budgeted_amount += self.amount;
//         }
//     }
// }
//
// impl DomainEvent<Budget> for BudgetEvent {
//     fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
//         match self {
//             BudgetEvent::BudgetCreated(e) => e.budget_id,
//             BudgetEvent::GroupAdded(e) => e.budget_id,
//             BudgetEvent::ItemAdded(e) => e.budget_id,
//             BudgetEvent::TransactionAdded(e) => e.budget_id,
//             BudgetEvent::TransactionConnected(e) => e.budget_id,
//             BudgetEvent::FundsReallocated(e) => e.budget_id,
//         }
//     }
//
//     fn apply(&self, state: &mut Budget) {
//         match self {
//             BudgetEvent::BudgetCreated(e) => e.apply(state),
//             BudgetEvent::GroupAdded(e) => e.apply(state),
//             BudgetEvent::ItemAdded(e) => e.apply(state),
//             BudgetEvent::TransactionAdded(e) => e.apply(state),
//             BudgetEvent::TransactionConnected(e) => e.apply(state),
//             BudgetEvent::FundsReallocated(e) => e.apply(state),
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BudgetGroup {
//     pub id: Uuid,
//     pub name: String,
//     pub items: Vec<BudgetItem>,
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BudgetItem {
//     pub id: Uuid,
//     pub name: String,
//     pub item_type: BudgetItemType,
//     pub budgeted_amount: Money,
//     pub actual_spent: Money,
//     pub notes: Option<String>,
//     pub tags: Vec<String>,
// }
//
// #[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
// pub enum BudgetItemType {
//     Income,
//     Expense,
//     Savings,
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BankTransaction {
//     pub id: Uuid,
//     pub amount: Money,
//     pub description: String,
//     pub date: DateTime<Utc>,
//     pub budget_item_id: Option<Uuid>,
// }
//
// impl Display for BankTransaction {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{}, {}, {}", self.description, self.amount, self.date)
//     }
// }
//
// impl BudgetItem {
//     pub fn new(
//         name: &str,
//         item_type: BudgetItemType,
//         budgeted_amount: Money,
//         notes: Option<String>,
//         tags: Option<Vec<String>>,
//     ) -> Self {
//         Self {
//             id: Uuid::new_v4(),
//             name: name.to_string(),
//             item_type,
//             budgeted_amount,
//             actual_spent: Money::new(0, budgeted_amount.currency),
//             notes,
//             tags: tags.unwrap_or_default(),
//         }
//     }
// }
//
// impl BankTransaction {
//     pub fn new(amount: Money, description: &str, date: DateTime<Utc>) -> Self {
//         Self {
//             id: Uuid::new_v4(),
//             amount,
//             description: description.to_string(),
//             date,
//             budget_item_id: None,
//         }
//     }
// }
//
// pub struct AddGroup {
//     pub name: String,
// }
//
// impl AddGroup {
//     pub fn new(name: String) -> Self {
//         Self { name }
//     }
// }
//
// impl Debug for AddGroup {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("AddGroup")
//             .field("name", &self.name)
//             .finish()
//     }
// }
//
// impl Decision<Budget, BudgetEvent> for AddGroup {
//     fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
//         match state {
//             Some(state) => {
//                 if state
//                     .budget_groups
//                     .values()
//                     .find(|g| g.name == self.name)
//                     .is_some()
//                 {
//                     Err(CommandError::Conflict("Group already exists"))
//                 } else {
//                     Ok(BudgetEvent::GroupAdded(GroupAdded {
//                         budget_id: state.id,
//                         group_id: Uuid::new_v4(),
//                         name: self.name,
//                     }))
//                 }
//             }
//             None => Err(CommandError::NotFound("Budget not found")),
//         }
//     }
// }


//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AddItem {
//     pub group_id: Uuid,
//     pub name: String,
//     pub item_type: BudgetItemType,
//     pub budgeted_amount: Money,
//     pub notes: Option<String>,
//     pub tags: Option<Vec<String>>,
// }
//
// impl AddItem {
//     pub fn new(
//         group_id: Uuid,
//         name: String,
//         item_type: BudgetItemType,
//         budgeted_amount: Money,
//         notes: Option<String>,
//         tags: Option<Vec<String>>,
//     ) -> Self {
//         Self {
//             group_id,
//             name,
//             item_type,
//             budgeted_amount,
//             notes,
//             tags,
//         }
//     }
// }
//
// impl Decision<Budget, BudgetEvent> for AddItem {
//     fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
//         match state {
//             None => Err(CommandError::NotFound("Budget not found")),
//             Some(state) => Ok(BudgetEvent::ItemAdded(ItemAdded {
//                 budget_id: state.id,
//                 group_id: self.group_id,
//                 item: BudgetItem::new(
//                     &self.name,
//                     self.item_type,
//                     self.budgeted_amount,
//                     self.notes,
//                     self.tags,
//                 ),
//             })),
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AddTransaction {
//     pub amount: Money,
//     pub description: String,
//     pub date: DateTime<Utc>,
// }
//
// impl Decision<Budget, BudgetEvent> for AddTransaction {
//     fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
//         match state {
//             None => Err(CommandError::NotFound("Budget not found")),
//             Some(state) => Ok(BudgetEvent::TransactionAdded(TransactionAdded {
//                 budget_id: state.id,
//                 tx: BankTransaction {
//                     id: Uuid::new_v4(),
//                     amount: self.amount,
//                     description: self.description,
//                     date: self.date,
//                     budget_item_id: None,
//                 },
//             })),
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ConnectTransaction {
//     pub tx_id: Uuid,
//     pub item_id: Uuid,
// }
//
// impl Decision<Budget, BudgetEvent> for ConnectTransaction {
//     fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
//         match state {
//             None => Err(CommandError::NotFound("Budget not found")),
//             Some(state) => {
//                 if state.bank_transactions.contains_key(&self.tx_id)
//                     && state.get_item(&self.item_id).is_some()
//                 {
//                     Ok(BudgetEvent::TransactionConnected(
//                         TransactionConnected::new(state.id, self.tx_id, self.item_id),
//                     ))
//                 } else {
//                     Err(CommandError::NotFound("Transaction or item not found"))
//                 }
//             }
//         }
//     }
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ReallocateFunds {
//     pub from_item: Uuid,
//     pub to_item: Uuid,
//     pub amount: Money,
// }
//
// impl Decision<Budget, BudgetEvent> for ReallocateFunds {
//     fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
//         match state {
//             None => Err(CommandError::NotFound("Budget not found")),
//             Some(state) => match state.get_item(&self.from_item) {
//                 None => Err(CommandError::NotFound("From item not found")),
//                 Some(from_item) => match state.get_item(&self.to_item) {
//                     None => Err(CommandError::NotFound("To item not found")),
//                     Some(_) => {
//                         if from_item.budgeted_amount < self.amount {
//                             Err(CommandError::Validation("From item has not enough funds"))
//                         } else {
//                             Ok(BudgetEvent::FundsReallocated(FundsReallocated {
//                                 budget_id: state.id,
//                                 from_item: self.from_item,
//                                 to_item: self.to_item,
//                                 amount: self.amount,
//                             }))
//                         }
//                     }
//                 },
//             },
//         }
//     }
// }
//
// pub struct CreateBudgetArgs {
//     pub id: Uuid,
//     pub name: String,
//     pub user_id: Uuid,
//     pub default_budget: bool,
// }
//
//

//
