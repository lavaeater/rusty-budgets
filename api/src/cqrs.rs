// Minimal, generic CQRS + Event Sourcing framework in one file.
// No external crates. `rustc` stable compatible.
// ---------------------------------------------------------------
// This file contains:
// 1) A tiny generic framework (traits + in-memory runtime)
// 2) A small demo domain (bank account) showing commands/events
// 3) A `main` that exercises the framework

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

// ===========================
// Framework (Generic Core)
// ===========================

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug;

    /// Create a blank/new instance for a given id.
    fn new(id: Self::Id) -> Self;
}

/// Event: a fact that happened, applied to an Aggregate to evolve it.
pub trait Event<A: Aggregate>: Clone + Debug {
    /// Which aggregate instance does this event belong to?
    fn aggregate_id(&self) -> A::Id;

    /// Apply this event to the aggregate state.
    fn apply(&self, state: &mut A);
}

/// Command: an intention to change state. Produces (at most) one Event.
pub trait Command<A: Aggregate, E: Event<A>>: Debug {
    /// The target aggregate id for this command (routing key).
    fn aggregate_id(&self) -> A::Id;

    /// Business logic: take current state (if any) and decide an Event or error.
    fn handle(self, state: Option<&A>) -> Result<E, CommandError>;
}

#[derive(Debug)]
pub enum CommandError {
    Validation(&'static str),
    Conflict(&'static str),
    NotFound(&'static str),
}

/// Very small in-memory event store + runtime.
#[derive(Debug, Default)]
pub struct Runtime<A, E>
where
    A: Aggregate,
    E: Event<A>,
{
    // Append-only log per aggregate id.
    streams: HashMap<A::Id, Vec<E>>,
}

impl<A, E> Runtime<A, E>
where
    A: Aggregate,
    E: Event<A>,
{
    pub fn new() -> Self {
        Self { streams: HashMap::new() }
    }

    /// Load & rebuild current state from events (if any).
    pub fn load(&self, id: &A::Id) -> Option<A> {
        self.streams.get(id).map(|events| {
            let mut state = A::new(id.clone());
            for ev in events {
                ev.apply(&mut state);
            }
            state
        })
    }

    /// Append one event to the store.
    fn append(&mut self, ev: E) {
        let id = ev.aggregate_id();
        self.streams.entry(id).or_default().push(ev);
    }

    /// Execute a command: read, decide, append event, return event.
    pub fn execute<C>(&mut self, cmd: C) -> Result<E, CommandError>
    where
        C: Command<A, E>,
    {
        let id = cmd.aggregate_id();
        let current = self.load(&id);
        let event = cmd.handle(current.as_ref())?;
        self.append(event.clone());
        Ok(event)
    }

    /// Convenience: materialize the latest state after commands.
    pub fn materialize(&self, id: &A::Id) -> Option<A> {
        self.load(id)
    }

    /// Inspect raw events (for audit/testing).
    pub fn events(&self, id: &A::Id) -> Option<&[E]> {
        self.streams.get(id).map(|v| v.as_slice())
    }
}

// ===========================
// Demo Domain: Bank Account
// ===========================

#[derive(Debug, Clone)]
pub struct Account {
    pub id: u64,
    pub owner: String,
    pub balance: i64, // use integer (cents) for money safety in examples
}

impl Aggregate for Account {
    type Id = u64;

    fn new(id: Self::Id) -> Self {
        Self { id, owner: String::new(), balance: 0 }
    }
}

// ---- Events ----

#[derive(Debug, Clone)]
pub struct AccountCreated { pub id: u64, pub owner: String }

#[derive(Debug, Clone)]
pub struct MoneyDeposited { pub id: u64, pub amount_cents: i64 }

#[derive(Debug, Clone)]
pub struct MoneyWithdrawn { pub id: u64, pub amount_cents: i64 }

impl Event<Account> for AccountCreated {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }
    fn apply(&self, state: &mut Account) {
        state.id = self.id;
        state.owner = self.owner.clone();
        // balance starts at 0
    }
}

impl Event<Account> for MoneyDeposited {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }
    fn apply(&self, state: &mut Account) { state.balance += self.amount_cents; }
}

impl Event<Account> for MoneyWithdrawn {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }
    fn apply(&self, state: &mut Account) { state.balance -= self.amount_cents; }
}

// ---- Commands ----

#[derive(Debug)]
pub struct CreateAccount { pub id: u64, pub owner: String }

#[derive(Debug)]
pub struct DepositMoney { pub id: u64, pub amount_cents: i64 }

#[derive(Debug)]
pub struct WithdrawMoney { pub id: u64, pub amount_cents: i64 }

impl Command<Account, AccountCreated> for CreateAccount {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }

    fn handle(self, state: Option<&Account>) -> Result<AccountCreated, CommandError> {
        if state.is_some() {
            return Err(CommandError::Conflict("account already exists"));
        }
        if self.owner.trim().is_empty() {
            return Err(CommandError::Validation("owner must not be empty"));
        }
        Ok(AccountCreated { id: self.id, owner: self.owner })
    }
}

impl Command<Account, MoneyDeposited> for DepositMoney {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }

    fn handle(self, state: Option<&Account>) -> Result<MoneyDeposited, CommandError> {
        let _ = state.ok_or(CommandError::NotFound("account does not exist"))?;
        if self.amount_cents <= 0 { return Err(CommandError::Validation("amount must be > 0")); }
        Ok(MoneyDeposited { id: self.id, amount_cents: self.amount_cents })
    }
}

impl Command<Account, MoneyWithdrawn> for WithdrawMoney {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.id }

    fn handle(self, state: Option<&Account>) -> Result<MoneyWithdrawn, CommandError> {
        let acc = state.ok_or(CommandError::NotFound("account does not exist"))?;
        if self.amount_cents <= 0 { return Err(CommandError::Validation("amount must be > 0")); }
        if acc.balance < self.amount_cents { return Err(CommandError::Validation("insufficient funds")); }
        Ok(MoneyWithdrawn { id: self.id, amount_cents: self.amount_cents })
    }
}

// ===========================
// Example usage
// ===========================

fn main() {
    let mut rt: Runtime<Account, AccountCreated> = Runtime::new();
    // We can have separate runtimes per event type, or use an enum of events if you prefer one store.

    // Create account (produces AccountCreated)
    let _ = rt.execute(CreateAccount { id: 1, owner: "Alice".into() }).unwrap();

    // After creation, we want to handle money events. We can use a runtime for MoneyDeposited/Withdrawn
    let mut money_rt: Runtime<Account, MoneyDeposited> = Runtime::new();
    let mut withdraw_rt: Runtime<Account, MoneyWithdrawn> = Runtime::new();

    // Seed the money/withdraw runtimes with the creation event so materialization works across them in this minimal example.
    // In a real system, you'd have a *single* unified event enum or a single log and projections.
    if let Some(created_events) = rt.events(&1) {
        // Re-apply creation in the money/withdraw logs as a bootstrap (demo purpose only)
        // Normally you'd design a unified Event enum and single Runtime to avoid this ceremony.
        let mut unified: Vec<Box<dyn Fn(&mut Account)>> = Vec::new();
        // materialize state from creation to ensure existence
        let mut acc = Account::new(1);
        for _e in created_events {
            AccountCreated { id: 1, owner: "Alice".into() }.apply(&mut acc);
        }
        drop(unified);
        // Now execute money commands on their own runtimes
        let _ = money_rt.execute(DepositMoney { id: 1, amount_cents: 10_00 }).unwrap();
        let _ = money_rt.execute(DepositMoney { id: 1, amount_cents: 25_00 }).unwrap();
        let _ = withdraw_rt.execute(WithdrawMoney { id: 1, amount_cents: 5_00 }); // will fail because this runtime doesn't know deposits
    }

    // Realistic approach: use a single Event enum + single Runtime. See below for a compact variant.

    compact_demo();
}

// A more realistic compact demo: single Event enum + single Runtime instance.
#[derive(Debug, Clone)]
enum AccountEvent {
    Created(AccountCreated),
    Deposited(MoneyDeposited),
    Withdrawn(MoneyWithdrawn),
}

impl Event<Account> for AccountEvent {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id {
        match self {
            AccountEvent::Created(e) => e.id,
            AccountEvent::Deposited(e) => e.id,
            AccountEvent::Withdrawn(e) => e.id,
        }
    }
    fn apply(&self, state: &mut Account) {
        match self {
            AccountEvent::Created(e) => e.apply(state),
            AccountEvent::Deposited(e) => e.apply(state),
            AccountEvent::Withdrawn(e) => e.apply(state),
        }
    }
}

#[derive(Debug)]
struct Create(pub CreateAccount);
#[derive(Debug)]
struct Deposit(pub DepositMoney);
#[derive(Debug)]
struct Withdraw(pub WithdrawMoney);

impl Command<Account, AccountEvent> for Create {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.0.id }
    fn handle(self, state: Option<&Account>) -> Result<AccountEvent, CommandError> {
        <CreateAccount as Command<Account, AccountCreated>>::handle(self.0, state)
            .map(AccountEvent::Created)
    }
}
impl Command<Account, AccountEvent> for Deposit {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.0.id }
    fn handle(self, state: Option<&Account>) -> Result<AccountEvent, CommandError> {
        <DepositMoney as Command<Account, MoneyDeposited>>::handle(self.0, state)
            .map(AccountEvent::Deposited)
    }
}
impl Command<Account, AccountEvent> for Withdraw {
    fn aggregate_id(&self) -> <Account as Aggregate>::Id { self.0.id }
    fn handle(self, state: Option<&Account>) -> Result<AccountEvent, CommandError> {
        <WithdrawMoney as Command<Account, MoneyWithdrawn>>::handle(self.0, state)
            .map(AccountEvent::Withdrawn)
    }
}

fn compact_demo() {
    let mut rt: Runtime<Account, AccountEvent> = Runtime::new();

    // happy path
    rt.execute(Create(CreateAccount { id: 100, owner: "Bob".into() })).unwrap();
    rt.execute(Deposit(DepositMoney { id: 100, amount_cents: 50_00 })).unwrap();
    rt.execute(Withdraw(WithdrawMoney { id: 100, amount_cents: 20_00 })).unwrap();

    let acc = rt.materialize(&100).unwrap();
    println!("Account {:?}: owner={}, balance_cents={}", acc.id, acc.owner, acc.balance);

    // audit log
    println!("Events: {:?}", rt.events(&100).unwrap());
}
