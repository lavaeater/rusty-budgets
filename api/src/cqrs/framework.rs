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
    fn handle(self, state: Option<&A>) -> Result<E, CommandError>;
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
    fn load(&self, id: &A::Id) -> Option<A>;

    /// Hydrate runtime with known event stream.
    fn hydrate(&mut self, id: A::Id, events: Vec<StoredEvent<A, E>>);

    /// Append one new event to the stream.
    fn append(&mut self, ev: E);

    /// Execute a command: decide → append → return event.
    fn execute<C>(&mut self, cmd: C) -> Result<E, CommandError>
    where
        C: Command<A, E>,
    {
        let id = cmd.aggregate_id();
        let current = self.load(&id);
        let event = cmd.handle(current.as_ref())?;
        self.append(event.clone());
        Ok(event)
    }

    /// Materialize latest state after commands.
    fn materialize(&self, id: &A::Id) -> Option<A> {
        self.load(id)
    }

    /// Inspect raw events (for audit/testing).
    fn events(&self, id: &A::Id) -> Option<Vec<StoredEvent<A, E>>>;
}


/// Very small in-memory event store + runtime.
pub struct Borgtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    // Append-only log per aggregate id.
    on_event: Option<Box<dyn FnMut(&A::Id, StoredEvent<A, E>)>>,
    streams: HashMap<A::Id, Vec<StoredEvent<A, E>>>,
}

impl<A, E> Borgtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    pub fn new() -> Self {
        Self { streams: HashMap::new(), on_event: None }
    }
    
    pub fn with_storage_function(on_event: Box<dyn FnMut(&A::Id, StoredEvent<A, E>)>) -> Self {
        Self { streams: HashMap::new(), on_event: Some(on_event) }
    }

    /// Load & rebuild current state from events (if any).
    pub fn load(&self, id: &A::Id) -> Option<A> {
        self.streams.get(id).map(|events: &Vec<StoredEvent<A, E>>| {
            let mut state = A::new(id.clone());
            for ev in events {
                ev.data.apply(&mut state);
            }
            state
        })
    }
    
    pub fn hydrate(&mut self, id: A::Id, events: Vec<StoredEvent<A, E>>) {
        self.streams.insert(id, events);
    }
    pub fn to_storage(&self) -> HashMap<A::Id, Vec<StoredEvent<A, E>>> {
        self.streams.clone()
    }

    /// Append one event to the store.
    fn append(&mut self, ev: E) {
        let aggregate_id = ev.aggregate_id();
        let stored_event = StoredEvent::new(ev);
        if let Some(ref mut on_event) = self.on_event {
            on_event(&aggregate_id, stored_event.clone());
        }
        self.streams.entry(aggregate_id).or_default().push(stored_event);
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
    pub fn events(&self, id: &A::Id) -> Option<&[StoredEvent<A, E>]> {
        self.streams.get(id).map(|v| v.as_slice())
    }
}
