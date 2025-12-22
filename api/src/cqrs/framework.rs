// ===========================
// Framework (Generic Core)
// ===========================

use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use joydb::JoydbError;
use uuid::Uuid;

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug + Clone {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug + Default;

    /// Create a blank/new instance for a given id.
    fn _new(id: Self::Id) -> Self;
    fn _default() -> Self;

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>);
    fn version(&self) -> u64;
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

impl<A: Aggregate, E: DomainEvent<A>> StoredEvent<A, E> {
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

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Aggregate not found")]
    AggregateNotFound,
    #[error("Command error: {0}")]
    CommandError(CommandError),
    #[error("Database error: {0}")]
    DbError(JoydbError)
}

impl From<CommandError> for RuntimeError {
    fn from(value: CommandError) -> Self {
        RuntimeError::CommandError(value)
    }
}

impl From<JoydbError> for RuntimeError {
    fn from(value: JoydbError) -> Self {
        RuntimeError::DbError(value)
    }
}

pub trait Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    /// Load and rebuild current state from stored events.
    fn load(&self, id: A::Id) -> Result<Option<A>, RuntimeError>;

    fn snapshot(&self, agg: &A) -> Result<(), RuntimeError>;

    /// Append one new event to the stream.
    fn append(&self, user_id: Uuid, ev: E) -> Result<(), RuntimeError>;

    /// Execute a command: decide → append → return event.
    fn execute<F>(&self, user_id: Uuid, id: A::Id, command: F) -> Result<Uuid, RuntimeError>
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
            let mut current = self.load(id)?.unwrap();

            let ev = command(&current)?;

            let latest_id = ev.apply(&mut current);

            self.append(user_id, ev.clone())?;
            Ok(latest_id)
        }
    }

    /// Materialize latest state after commands.
    fn materialize(&self, id: A::Id) -> Result<A, RuntimeError> {
        let state = self.load(id)?;
        if let Some(state) = state {
            Ok(state)
        } else {
            Err(RuntimeError::AggregateNotFound)
        }
    }

    /// Inspect raw events (for audit/testing).
    fn events(&self, id: A::Id) -> Result<Vec<StoredEvent<A, E>>, RuntimeError>;
}
