// ===========================
// Framework (Generic Core)
// ===========================

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use joydb::Model;
use uuid::Uuid;

/// Aggregate: domain state that evolves by applying events.
pub trait Aggregate: Sized + Debug {
    /// Identifier type for this aggregate.
    type Id: Eq + Hash + Clone + Debug;

    /// Create a blank/new instance for a given id.
    fn new(id: Self::Id) -> Self;
}

#[derive(Debug, Clone)]
pub struct StoredEvent<A: Aggregate, E: DomainEvent<A>> {
    pub aggregate_id: A::Id,
    pub event_id: Uuid,
    pub ts: u128,
    pub data: E,
}

impl<A: Aggregate, E: DomainEvent<A>> StoredEvent<A, E> {
    pub fn new(data: E) -> Self {
        let aggregate_id = data.aggregate_id();
        let event_id = Uuid::new_v4();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap()
            .as_millis();
        Self { event_id, aggregate_id, ts, data }
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

/// Very small in-memory event store + runtime.
#[derive(Debug, Default)]
pub struct Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    // Append-only log per aggregate id.
    streams: HashMap<A::Id, Vec<StoredEvent<A, E>>>,
}

impl<A, E> Runtime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    pub fn new() -> Self {
        Self { streams: HashMap::new() }
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

    /// Append one event to the store.
    fn append(&mut self, ev: E) {
        let id = ev.aggregate_id();
        let stored_event = StoredEvent::new(ev);
        self.streams.entry(id).or_default().push(stored_event);
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
