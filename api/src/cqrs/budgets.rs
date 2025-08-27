use std::cmp::Ordering;
use crate::cqrs::budgets::BudgetEvent::{Created, GroupAddedToBudget};
use crate::cqrs::framework::*;
use chrono::{DateTime, Utc};
use joydb::adapters::JsonAdapter;
use joydb::{Joydb, JoydbError, Model};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, AddAssign, Sub, SubAssign};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    EUR,
    USD,
    SEK,
    // extend as needed
}

#[derive(Debug, Clone, Copy, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    amount: i64,       // stored in minor units (cents/öre)
    currency: Currency,
}

impl PartialOrd for Money {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.currency != other.currency {
            None
        } else {
            Some(self.amount.cmp(&other.amount))
        }
    }
}

impl PartialEq for Money {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.currency == other.currency
    }
}

impl Money {
    pub fn new(amount: i64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new(self.amount + rhs.amount, self.currency)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new(self.amount - rhs.amount, self.currency)
    }
}

// Pretty-printing
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02} {:?}", self.amount / 100, self.amount % 100, self.currency)
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetEvent {
    Created(BudgetCreated),
    GroupAddedToBudget(GroupAdded),
    ItemAdded(ItemAdded),
    TransactionAdded(TransactionAdded),
    TransactionConnected(TransactionConnected),
    FundsReallocated(FundsReallocated),
}

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

joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget],
}

type Db = Joydb<AppState, JsonAdapter>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
}

impl DomainEvent<Budget> for BudgetCreated {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.id = self.budget_id;
        state.name = self.name.clone();
        state.user_id = self.user_id;
        state.default_budget = self.default;
    }
}

impl BudgetCreated {
    pub fn new(name: String, user_id: Uuid, default: bool) -> Self {
        Self {
            budget_id: Uuid::new_v4(),
            name,
            user_id,
            default,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
}

impl DomainEvent<Budget> for GroupAdded {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.budget_groups.insert(
            self.group_id,
            BudgetGroup {
                id: self.group_id,
                name: self.name.clone(),
                items: Vec::default(),
            },
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub item: BudgetItem,
}

impl DomainEvent<Budget> for ItemAdded {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        if let Some(group) = state.budget_groups.get_mut(&self.group_id) {
            group.items.push(self.item.clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAdded {
    budget_id: Uuid,
    tx: BankTransaction,
}

impl DomainEvent<Budget> for TransactionAdded {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.bank_transactions.insert(self.tx.id, self.tx.clone());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConnected {
    budget_id: Uuid,
    tx_id: Uuid,
    item_id: Uuid,
}

impl TransactionConnected {
    pub fn new(budget_id: Uuid, tx_id: Uuid, item_id: Uuid) -> Self {
        Self {
            budget_id,
            tx_id,
            item_id,
        }
    }
}

impl DomainEvent<Budget> for TransactionConnected {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        if let Some(tx) = state
            .bank_transactions.get_mut(&self.tx_id)
        {
            tx.budget_item_id = Some(self.item_id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundsReallocated {
    budget_id: Uuid,
    from_item: Uuid,
    to_item: Uuid,
    amount: Money,
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.amount -= rhs.amount;
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.amount += rhs.amount;
    }
}

impl DomainEvent<Budget> for FundsReallocated {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        if let Some(mut from_item) = state.get_item_mut(&self.from_item)
        {
            from_item.budgeted_amount -= self.amount;
        }
        if let Some(mut to_item) = state.get_item_mut(&self.to_item)
        {
            to_item.budgeted_amount += self.amount;
        }
    }
}

impl DomainEvent<Budget> for BudgetEvent {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        match self {
            Created(e) => e.budget_id,
            GroupAddedToBudget(e) => e.budget_id,
            BudgetEvent::ItemAdded(e) => e.budget_id,
            BudgetEvent::TransactionAdded(e) => e.budget_id,
            BudgetEvent::TransactionConnected(e) => e.budget_id,
            BudgetEvent::FundsReallocated(e) => e.budget_id,
        }
    }

    fn apply(&self, state: &mut Budget) {
        match self {
            BudgetEvent::Created(e) => e.apply(state),
            BudgetEvent::GroupAddedToBudget(e) => e.apply(state),
            BudgetEvent::ItemAdded(e) => e.apply(state),
            BudgetEvent::TransactionAdded(e) => e.apply(state),
            BudgetEvent::TransactionConnected(e) => e.apply(state),
            BudgetEvent::FundsReallocated(e) => e.apply(state),
        }
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_groups: HashMap<Uuid, BudgetGroup>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
}

impl Budget {
    fn get_item_mut(&mut self, item_id: &Uuid) -> Option<&mut BudgetItem> {
        self
            .budget_groups
            .iter_mut()
            .flat_map(move |(_, group)| group.items.iter_mut())
            .find(|item| item.id == *item_id)
    }
    
    fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self
            .budget_groups
            .iter()
            .flat_map(move |(_, group)| group.items.iter())
            .find(|item| item.id == *item_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetGroup {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<BudgetItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: Money,
    pub actual_spent: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetItemType {
    Income,
    Expense,
    Savings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransaction {
    pub id: Uuid,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub budget_item_id: Option<Uuid>,
}

impl BudgetItem {
    pub fn new(
        name: &str,
        item_type: BudgetItemType,
        budgeted_amount: Money,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            item_type,
            budgeted_amount,
            actual_spent: Money::new(0, budgeted_amount.currency),
            notes: None,
            tags: Vec::new(),
        }
    }
}

impl BankTransaction {
    pub fn new(amount: Money, description: &str, date:DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            amount,
            description: description.to_string(),
            date,
            budget_item_id: None,
        }
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn new(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            user_id: Uuid::new_v4(),
            default_budget: false,
            budget_groups: HashMap::new(),
            bank_transactions: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_event: 0,
            version: 0,
        }
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

// --- Commands ---
pub struct CreateBudget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
}

impl CreateBudget {
    pub fn new(name: String, user_id: Uuid, default: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            user_id,
            default,
        }
    }
}

impl Debug for CreateBudget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateBudget")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("user_id", &self.user_id)
            .field("default", &self.default)
            .finish()
    }
}

impl Command<Budget, BudgetEvent> for CreateBudget {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.id
    }

    fn handle(self, _state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        Ok(Created(BudgetCreated {
            budget_id: self.id,
            name: self.name,
            user_id: self.user_id,
            default: self.default,
        }))
    }
}

pub struct AddGroup {
    pub budget_id: Uuid,
    pub name: String,
}

impl AddGroup {
    pub fn new(budget_id: Uuid, name: String) -> Self {
        Self { budget_id, name }
    }
}

impl Debug for AddGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddGroup")
            .field("budget_id", &self.budget_id)
            .field("name", &self.name)
            .finish()
    }
}

impl Command<Budget, BudgetEvent> for AddGroup {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        match state {
            Some(state) => {
                if state.budget_groups.values().find(|g| g.name == self.name).is_some() {
                    Err(anyhow::anyhow!("Duplicate group name"))
                } else {
                    Ok(GroupAddedToBudget(GroupAdded {
                        budget_id: self.budget_id,
                        group_id: Uuid::new_v4(),
                        name: self.name,
                    }))
                }
            }
            None => {
                Err(anyhow::anyhow!("Budget not found"))
            }
        }
    }
}

pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

impl JoyDbBudgetRuntime {
    fn new() -> Self {
        Self {
            db: Db::open("data.json").unwrap(),
        }
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

    fn hydrate(&mut self, id: Uuid, events: Vec<StoredBudgetEvent>) {
        todo!()
    }

    fn append(&mut self, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev);
        self.db.insert(&stored_event).unwrap();
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItem {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: Money,
    pub actual_spent: Money,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl AddItem {
    pub fn new(
        budget_id: Uuid,
        group_id: Uuid,
        name: String,
        item_type: BudgetItemType,
        budgeted_amount: Money,
        actual_spent: Money,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            budget_id,
            group_id,
            name,
            item_type,
            budgeted_amount,
            actual_spent,
            notes,
            tags,
        }
    }
}

impl Command<Budget, BudgetEvent> for AddItem {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, _state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        Ok(BudgetEvent::ItemAdded(ItemAdded {
            budget_id: self.budget_id,
            group_id: self.group_id,
            item: BudgetItem {
                id: Uuid::new_v4(),
                name: self.name,
                item_type: self.item_type,
                budgeted_amount: self.budgeted_amount,
                actual_spent: self.actual_spent,
                notes: self.notes,
                tags: self.tags.unwrap_or_default(),
            }
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTransaction {
    pub budget_id: Uuid,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
}

impl Command<Budget, BudgetEvent> for AddTransaction {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, _state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        Ok(BudgetEvent::TransactionAdded(TransactionAdded {
            budget_id: self.budget_id,
            tx: BankTransaction {
                id: Uuid::new_v4(),
                amount: self.amount,
                description: self.description,
                date: self.date,
                budget_item_id: None,
            },
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectTransaction {
    pub budget_id: Uuid,
    pub tx_id: Uuid,
    pub item_id: Uuid,
}

impl Command<Budget, BudgetEvent> for ConnectTransaction {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        match state {
            None => {
                Err(anyhow::anyhow!("Budget not found"))
            }
            Some(state) => {
                if state.bank_transactions.contains_key(&self.tx_id) && state.get_item(&self.item_id).is_some() {
                    Ok(BudgetEvent::TransactionConnected(TransactionConnected {
                        budget_id: self.budget_id,
                        tx_id: self.tx_id,
                        item_id: self.item_id,
                    }))
                } else {
                    Err(anyhow::anyhow!("Transaction or item not found"))
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReallocateFunds {
    pub budget_id: Uuid,
    pub from_item: Uuid,
    pub to_item: Uuid,
    pub amount: Money,
}
    
impl Command<Budget, BudgetEvent> for ReallocateFunds {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        match state {
            None => {
                Err(anyhow::anyhow!("Budget not found")) 
            }
            Some(state) => {
                match state.get_item(&self.from_item) {
                    None => {
                        Err(anyhow::anyhow!("From item not found")) 
                    }
                    Some(from_item) => {
                        match state.get_item(&self.to_item) {
                            None => {
                                Err(anyhow::anyhow!("To item not found"))
                            }
                            Some(_) => {
                                if from_item.budgeted_amount < self.amount {
                                     Err(anyhow::anyhow!("From item has not enough funds"))
                                } else {
                                    Ok(BudgetEvent::FundsReallocated(FundsReallocated {
                                        budget_id: self.budget_id,
                                        from_item: self.from_item,
                                        to_item: self.to_item,
                                        amount: self.amount,
                                    }))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[test]
pub fn testy() {
    let mut rt = JoyDbBudgetRuntime::new();
    let budget_id = Uuid::new_v4();

    // happy path
    rt.execute(CreateBudget::new(
        budget_id,
        "Family Budget".into(),
        Uuid::new_v4(),
        true,
    ))
        .unwrap();
    rt.execute(AddGroup::new(budget_id, "Salaries".into()))
        .unwrap();
    let budget = rt.materialize(&budget_id).unwrap();
    rt.snapshot(&budget).unwrap();
    rt.execute(AddGroup::new(budget_id, "New group".into()))
        .unwrap();
    // rt.execute(crate::cqrs::framework::Deposit(DepositMoney { id: 100, amount_cents: 50_00 })).unwrap();
    // rt.execute(crate::cqrs::framework::Withdraw(WithdrawMoney { id: 100, amount_cents: 20_00 })).unwrap();

    let budget_agg = rt.materialize(&budget_id).unwrap();
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );

    // audit log
    println!("Events: {:?}", rt.events(&budget_id).unwrap());
}
