// ===========================
// Framework (Generic Core)
// ===========================

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use joydb::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug + Clone {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug;

    /// Create a blank/new instance for a given id.
    fn new(id: Self::Id) -> Self;
    
    fn update_timestamp(&mut self, timestamp: u128);
    fn version(&self) -> u64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent<A, E> where A: Aggregate, E: DomainEvent<A> {
    pub id: Uuid,
    pub aggregate_id: A::Id,
    pub timestamp: u128,
    pub data: E,
}

impl<A: Aggregate, E: DomainEvent<A>> StoredEvent<A, E> {
    pub fn new(data: E) -> Self {
        let aggregate_id = data.aggregate_id();
        let event_id = Uuid::new_v4();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap()
            .as_nanos();
        Self { id: event_id, aggregate_id, timestamp: ts, data }
    }
    
    pub fn apply(&self, state:  &mut A) {
        self.data.apply(state);
        state.update_timestamp(self.timestamp);
    }
}

/// Event: a fact that happened, applied to an Aggregate to evolve it.
pub trait DomainEvent<A: Aggregate>: Clone + Debug {
    /// Which aggregate instance does this event belong to?
    fn aggregate_id(&self) -> A::Id;

    /// Apply this event to the aggregate state.
    fn apply(&self, state: &mut A);
}

/// Command: an intention to change state. Produces (at most) one Event.
pub trait Command<A: Aggregate, E: DomainEvent<A>>: Debug {
    /// The target aggregate id for this command (routing key).
    fn aggregate_id(&self) -> A::Id;

    /// Business logic: take current state (if any) and decide an Event or error.
    fn handle(self, state: Option<&A>) -> anyhow::Result<E>;
}

#[derive(Debug)]
pub enum CommandError {
    Validation(&'static str),
    Conflict(&'static str),
    NotFound(&'static str),
}

pub trait Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    /// Load and rebuild current state from stored events.
    fn load(&self, id: &A::Id) -> anyhow::Result<Option<A>>;
    
    fn snapshot(&self, agg: &A) -> anyhow::Result<()>;

    /// Hydrate runtime with known event stream.
    fn hydrate(&mut self, id: A::Id, events: Vec<StoredEvent<A, E>>);

    /// Append one new event to the stream.
    fn append(&mut self, ev: E);

    /// Execute a command: decide → append → return event.
    fn execute<C>(&mut self, cmd: C) -> anyhow::Result<E>
    where
        C: Command<A, E>,
    {
        let id = cmd.aggregate_id();
        let current = self.load(&id)?;
        let event = cmd.handle(current.as_ref())?;
        self.append(event.clone());
        Ok(event)
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