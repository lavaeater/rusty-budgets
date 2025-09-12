// ===========================
// Framework (Generic Core)
// ===========================

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug + Clone {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug;

    /// Create a blank/new instance for a given id.
    fn _new(id: Self::Id) -> Self;
    
    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>);
    fn _version(&self) -> u64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent<A, E> where A: Aggregate, E: DomainEvent<A> {
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
        let timestamp = n
            .timestamp_nanos_opt()
            .unwrap();
        
        let created_at = n;
        
        Self { id: event_id, aggregate_id, timestamp, created_at, data, user_id }
    }
    
    pub fn apply(&self, state:  &mut A) {
        self.data.apply(state);
        state.update_timestamp(self.timestamp, self.created_at);
    }
}

/// Event: a fact that happened, applied to an Aggregate to evolve it.
pub trait DomainEvent<A: Aggregate>: Clone + Debug + Sized {
    /// Which aggregate instance does this event belong to?
    fn aggregate_id(&self) -> A::Id;

    /// Apply this event to the aggregate state.
    fn apply(&self, state: &mut A);
}

#[derive(Debug)]
pub enum CommandError {
    Validation(&'static str),
    Conflict(&'static str),
    NotFound(&'static str),
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Validation(msg) => write!(f, "Validation error: {}", msg),
            CommandError::Conflict(msg) => write!(f, "Conflict error: {}", msg),
            CommandError::NotFound(msg) => write!(f, "Not found error: {}", msg),
        }
    }
}

impl Error for CommandError {}

pub trait Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    /// Load and rebuild current state from stored events.
    fn load(&self, id: &A::Id) -> anyhow::Result<Option<A>>;
    
    fn snapshot(&self, agg: &A) -> anyhow::Result<()>;

    /// Append one new event to the stream.
    fn append(&self, user_id: &Uuid, ev: E);

    /// Execute a command: decide → append → return event.
    fn execute<F>(&self, user_id: &Uuid, id: &A::Id, command: F) -> anyhow::Result<A>
    where
        F: FnOnce(&A) -> Result<E, CommandError>,
    {
        let mut current = self.load(id)?.unwrap_or_else(|| A::_new(id.clone()));

        let ev = command(&current)?;
        ev.apply(&mut current);
        self.append(user_id, ev.clone());
        Ok(current)
    }
    
    /// Materialize latest state after commands.
    fn materialize(&self, id: &A::Id) -> anyhow::Result<A> {
        let state = self.load(id)?;
        if let Some(state) = state {
            Ok(state)
        } else {
            Err(anyhow::anyhow!("Aggregate not found"))
        }
    }

    /// Inspect raw events (for audit/testing).
    fn events(&self, id: &A::Id) -> anyhow::Result<Vec<StoredEvent<A, E>>>;
}