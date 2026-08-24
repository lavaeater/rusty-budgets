// ===========================
// Framework (Generic Core)
// ===========================

use crate::api_error::RustyError;
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use joydb::JoydbError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::prelude::DbState;
#[cfg(feature = "server")]
use crate::cqrs::runtime::StoredBudgetEvent;
#[cfg(feature = "server")]
use crate::models::{Budget, BudgetEvent};
#[cfg(feature = "server")]
use crate::pg_models::{PgBudget, PgStoredBudgetEvent};

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug + Clone {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug + Default;

    /// Create a blank/new instance for a given id.
    fn _new(id: Self::Id) -> Self;
    fn _default() -> Self;

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>);
    fn version(&self) -> i64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    pub id: Uuid,
    pub aggregate_id: A::Id,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
    pub data: E,
    pub user_id: Uuid,
}

#[cfg(feature = "server")]
impl From<DbState<PgStoredBudgetEvent>> for StoredEvent<Budget, BudgetEvent> {
    fn from(value: DbState<PgStoredBudgetEvent>) -> Self {
        Self::new(serde_json::from_value(value.data.clone()).unwrap(), value.user_id)
    }
}

#[cfg(feature = "server")]
impl From<DbState<PgBudget>> for Budget {
    fn from(value: DbState<PgBudget>) -> Self {
        serde_json::from_value(value.data.clone()).expect("failed to deserialize Budget")
    }
}

impl<A: Aggregate, E: DomainEvent<A>> StoredEvent<A, E> {
    /// # Panics
    /// Panics if the current time cannot be represented as nanoseconds since the Unix epoch.
    pub fn new(data: E, user_id: Uuid) -> Self {
        let aggregate_id = data.aggregate_id();
        let event_id = Uuid::new_v4();
        let n = Utc::now();
        let timestamp = n.timestamp_nanos_opt().unwrap();

        let created_at = n;

        Self {
            id: event_id,
            aggregate_id,
            timestamp,
            created_at,
            data,
            user_id,
        }
    }

    pub fn apply(&self, state: &mut A) {
        state.update_timestamp(self.timestamp, self.created_at);
        self.data.apply(state);
    }
}

/// Event: a fact that happened, applied to an Aggregate to evolve it.
pub trait DomainEvent<A: Aggregate>: Clone + Debug + Sized {
    /// Which aggregate instance does this event belong to?
    fn aggregate_id(&self) -> A::Id;

    /// Apply this event to the aggregate state.
    fn apply(&self, state: &mut A) -> Uuid;
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Conflict error: {0}")]
    Conflict(String),
    #[error("Not found error: {0}")]
    NotFound(String),
}

pub trait Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    /// Load and rebuild current state from stored events.
    fn load(&self, id: A::Id) -> Result<A, RustyError>;

    fn snapshot(&self, agg: &A) -> Result<(), RustyError>;

    /// Append one new event to the stream.
    fn append(&self, user_id: Uuid, ev: E) -> Result<(), RustyError>;

    /// Execute a command: decide → append → return event.
    fn execute<F>(&self, user_id: Uuid, id: A::Id, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&A) -> Result<E, CommandError>,
    {
        let d: A::Id = Default::default();
        if id == d {
            tracing::info!("This is ugly trick for creating new aggregates");
            let mut current = A::_default();
            let ev = command(&current)?;
            let latest_id = ev.apply(&mut current);
            self.append(user_id, ev.clone())?;
            Ok(latest_id)
        } else {
            let mut current = self.load(id)?;

            let ev = command(&current)?;

            let latest_id = ev.apply(&mut current);

            self.append(user_id, ev.clone())?;
            Ok(latest_id)
        }
    }
    fn fetch_events(
        &self,
        id: Uuid,
        last_timestamp: i64,
    ) -> Result<Vec<StoredEvent<A, E>>, RustyError>;
    fn get_budget(&self, id: Uuid) -> Result<Option<A>, RustyError>;
    fn undo_last(&self, budget_id: Uuid) -> Result<bool, RustyError>;

    /// Inspect raw events (for audit/testing).
    fn events(&self, id: A::Id) -> Result<Vec<StoredEvent<A, E>>, RustyError>;

    /// Append many events in one go. Default just loops `append`; a runtime
    /// backed by a real database can override this with a genuine bulk insert.
    fn append_many(&self, user_id: Uuid, events: Vec<E>) -> Result<(), RustyError> {
        for ev in events {
            self.append(user_id, ev)?;
        }
        Ok(())
    }

    /// Run many commands against a *single* loaded copy of the aggregate,
    /// applying each successful one in memory before moving to the next, then
    /// append every resulting event and snapshot once at the end.
    ///
    /// This exists for bulk workloads (e.g. importing thousands of
    /// transactions) where going through `execute` per command would reload
    /// and replay the whole event stream on every single call — turning an
    /// O(rows) amount of work into O(rows²). Failed commands (e.g. a
    /// duplicate transaction) are reported per-index rather than aborting the
    /// whole batch.
    #[allow(clippy::type_complexity)]
    fn bulk_execute(
        &self,
        user_id: Uuid,
        id: A::Id,
        commands: Vec<Box<dyn FnOnce(&A) -> Result<E, CommandError> + Send>>,
    ) -> Result<Vec<Result<Uuid, CommandError>>, RustyError> {
        let mut current = self.load(id)?;
        let mut events = Vec::with_capacity(commands.len());
        let mut results = Vec::with_capacity(commands.len());
        for command in commands {
            match command(&current) {
                Ok(ev) => {
                    let latest_id = ev.apply(&mut current);
                    results.push(Ok(latest_id));
                    events.push(ev);
                }
                Err(e) => results.push(Err(e)),
            }
        }
        if !events.is_empty() {
            self.append_many(user_id, events)?;
            self.snapshot(&current)?;
        }
        Ok(results)
    }
}

#[allow(async_fn_in_trait)]
pub trait AsyncRuntime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    /// Load and rebuild current state from stored events.
    async fn load(&self, id: A::Id) -> Result<A, RustyError>;

    async fn snapshot(&self, agg: &A) -> Result<(), RustyError>;

    /// Append one new event to the stream.
    async fn append(&self, user_id: Uuid, ev: E) -> Result<(), RustyError>;

    /// Execute a command: decide → append → return event.
    async fn execute<F>(&self, user_id: Uuid, id: A::Id, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&A) -> Result<E, CommandError>,
    {
        let d: A::Id = Default::default();
        if id == d {
            tracing::info!("This is ugly trick for creating new aggregates");
            let mut current = A::_default();
            let ev = command(&current)?;
            let latest_id = ev.apply(&mut current);
            self.append(user_id, ev.clone()).await?;
            Ok(latest_id)
        } else {
            let mut current = self.load(id).await?;

            let ev = command(&current)?;

            let latest_id = ev.apply(&mut current);

            self.append(user_id, ev.clone()).await?;
            Ok(latest_id)
        }
    }
    
    async fn fetch_events(
        &self,
        id: Uuid,
        last_timestamp: i64,
    ) -> Result<Vec<StoredEvent<A, E>>, RustyError>;
    async fn get_budget(&self, id: Uuid) -> Result<Option<A>, RustyError>;
    async fn undo_last(&self, budget_id: Uuid) -> Result<bool, RustyError>;

    /// Inspect raw events (for audit/testing).
    async fn events(&self, id: A::Id) -> Result<Vec<StoredEvent<A, E>>, RustyError>;

    /// Append many events in one go. Default just loops `append`; a runtime
    /// backed by a real database can override this with a genuine bulk insert.
    async fn append_many(&self, user_id: Uuid, events: Vec<E>) -> Result<(), RustyError> {
        for ev in events {
            self.append(user_id, ev).await?;
        }
        Ok(())
    }

    /// Run many commands against a *single* loaded copy of the aggregate,
    /// applying each successful one in memory before moving to the next, then
    /// append every resulting event and snapshot once at the end.
    ///
    /// This exists for bulk workloads (e.g. importing thousands of
    /// transactions) where going through `execute` per command would reload
    /// and replay the whole event stream on every single call — turning an
    /// O(rows) amount of work into O(rows²). Failed commands (e.g. a
    /// duplicate transaction) are reported per-index rather than aborting the
    /// whole batch.
    #[allow(clippy::type_complexity)]
    async fn bulk_execute(
        &self,
        user_id: Uuid,
        id: A::Id,
        commands: Vec<Box<dyn FnOnce(&A) -> Result<E, CommandError> + Send>>,
    ) -> Result<Vec<Result<Uuid, CommandError>>, RustyError> {
        let mut current = self.load(id).await?;
        let mut events = Vec::with_capacity(commands.len());
        let mut results = Vec::with_capacity(commands.len());
        for command in commands {
            match command(&current) {
                Ok(ev) => {
                    let latest_id = ev.apply(&mut current);
                    results.push(Ok(latest_id));
                    events.push(ev);
                }
                Err(e) => results.push(Err(e)),
            }
        }
        if !events.is_empty() {
            self.append_many(user_id, events).await?;
            self.snapshot(&current).await?;
        }
        Ok(results)
    }
}
