use crate::cqrs::budget::Budget;
use crate::cqrs::domain_events::{BudgetCreated, GroupAdded};
use crate::cqrs::framework::Aggregate;
use crate::cqrs::framework::{DomainEvent, Runtime, StoredEvent};
use crate::pub_events_enum;
use joydb::adapters::JsonAdapter;
use joydb::{Joydb, Model};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug};
use std::str::FromStr;
use uuid::Uuid;

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

// impl Display for BudgetEvent {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match self {
//             BudgetEvent::BudgetCreated(event) => write!(f, "Budget created: {}", event.budget_id),
//             // BudgetEvent::GroupAdded(event) => {
//             //     write!(f, "Group added to budget: {}", event.group_id)
//             // }
//             // BudgetEvent::ItemAdded(event) => write!(f, "Item added: {}", event.item.id),
//             // BudgetEvent::TransactionAdded(event) => {
//             //     write!(f, "Transaction added: {}", event.tx)
//             // }
//             // BudgetEvent::TransactionConnected(event) => {
//             //     write!(f, "Transaction connected: {}", event.tx_id)
//             // }
//             // BudgetEvent::FundsReallocated(event) => {
//             //     write!(f, "Funds reallocated: {}", event.from_item)
//             // }
//         }
//     }
// }
//
type StoredBudgetEvent = StoredEvent<Budget, BudgetEvent>;

impl Model for StoredBudgetEvent {
    type Id = Uuid;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn model_name() -> &'static str {
        "budget_event"
    }
}
//
joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget],
}

type Db = Joydb<AppState, JsonAdapter>;
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

pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

impl JoyDbBudgetRuntime {
    fn new() -> Self {
        Self {
            db: Db::open("data.json").unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    pub fn cmd<F, E>(&mut self, id: Uuid, command: F) -> anyhow::Result<BudgetEvent>
    where
        F: FnOnce(&Budget) -> Result<E, crate::cqrs::framework::CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(id, |aggregate| {
            command(aggregate).map(|event| event.into())
        })
    }
    
    
    fn fetch_events(
        &self,
        id: &Uuid,
        last_timestamp: i64,
    ) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        let mut events: Vec<StoredBudgetEvent> = self.db.get_all_by(|e: &StoredBudgetEvent| {
            e.aggregate_id == *id && e.timestamp > last_timestamp
        })?;
        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }

    fn get_budget(&self, id: &Uuid) -> anyhow::Result<Option<Budget>> {
        let budget = self.db.get::<Budget>(id)?;
        if let Some(budget) = budget {
            Ok(Some(budget))
        } else {
            Ok(None)
        }
    }
}


impl Runtime<Budget, BudgetEvent> for JoyDbBudgetRuntime {
    fn load(&self, id: &Uuid) -> Result<Option<Budget>, anyhow::Error> {
        let budget = self.get_budget(id)?;
        match budget {
            Some(mut budget) => {
                let events = self.fetch_events(id, budget.last_event)?;
                for ev in events {
                    ev.apply(&mut budget);
                }
                Ok(Some(budget))
            }
            None => {
                let mut budget = Budget {
                    id: *id,
                    ..Default::default()
                };
                let events = self.fetch_events(id, budget.last_event)?;
                for ev in events {
                    ev.apply(&mut budget);
                }
                Ok(Some(budget))
            }
        }
    }

    fn snapshot(&self, agg: &Budget) -> anyhow::Result<()> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn append(&mut self, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev);
        self.db.insert(&stored_event).unwrap();
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}
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
#[cfg(test)]
#[test]
pub fn testy() -> anyhow::Result<()> {
    let mut rt = JoyDbBudgetRuntime::new();
    let budget_id = Uuid::from_str("760365f3-fa77-49a8-aa77-4717748e52ae")?;
    let user_id = Uuid::new_v4();
    // Look how clean this is now! No match, no .into(), no boilerplate!
    // rt.cmd(budget_id, |budget| budget.create_budget("Test Budget".to_string(), user_id, true))?;
    // rt.cmd(budget_id, |budget| budget.add_group(Uuid::new_v4(), "Inkomster".to_string()))?;

    let budget_agg = rt.materialize(&budget_id)?;
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );
    
    for group in budget_agg.budget_groups.values() {
        println!("Group: {}", group.name);
    }

    // audit log
    println!("Events: {:?}", rt.events(&budget_id)?);
    Ok(())
}
