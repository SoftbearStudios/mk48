use crate::hash::CompatHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{BitXorAssign, Deref, DerefMut};

/// A client or server (depending on [Role]) consisting of state and one or more passes of behavior.
pub struct World<S: State, P: Pass, R: Role> {
    /// On the server, this is the full state of the world (all partitions).
    /// On the client, this may be a partial state of the world (a subset of the partitions).
    state: S,
    /// The last pass of behavior, which recursively contains previous passes.
    pass: P,
    /// [Client] or [Server]; may contain role-specific state.
    role: R,
}

impl<S: State + Default, P: Pass + Default, R: Role + Default> Default for World<S, P, R> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S: State, P: Pass + Default, R: Role + Default> World<S, P, R> {
    pub fn new(state: S) -> Self {
        Self {
            state,
            pass: P::default(),
            role: R::default(),
        }
    }
}

impl<S: State, OP: Pass<State = S>, R: Role> World<S, OP, R> {
    /// Adds a new pass to the end of the behavior.
    pub fn add_pass<P: PassDef<State = S>>(self) -> World<S, PassContext<S, P, OP>, R> {
        World {
            state: self.state,
            pass: PassContext {
                prev: self.pass,
                pending: Vec::new(),
                _spooky: PhantomData,
            },
            role: self.role,
        }
    }
}

impl<S: State, P: Pass, R: Role> Deref for World<S, P, R> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.state
    }
}

/// Per-tick server to client update.
#[derive(Serialize, Deserialize)]
pub struct Update<S: State, P: Pass> {
    /// Partitions no longer visible to client.
    deletes: Vec<S::PartitionId>,
    /// Update for visible partitions.
    update: P::Update,
    /// Dispatched events.
    events: Vec<S::Event>,
    /// Newly visible partitions.
    completes: Vec<(S::PartitionId, S::Partition)>,
    /// Checksum after all the above is applied.
    checksum: S::Checksum,
}

impl<S: State, P: Pass> Debug for Update<S, P>
where
    S::PartitionId: Debug,
    S::Event: Debug,
    P::Update: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Update{{checksum: {:?} @ ", self.checksum,)?;
        for (i, event) in self
            .deletes
            .iter()
            .map(|partition_id| format!("{partition_id:?}: D"))
            .chain(Some(format!("{:?}", self.update)))
            .chain((!self.events.is_empty()).then(|| format!("{:?}", self.events)))
            .chain(
                self.completes
                    .iter()
                    .map(|(partition_id, _)| format!("{partition_id:?}: A")),
            )
            .enumerate()
        {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(&event)?;
        }
        write!(f, "]}}")
    }
}

/// Data stored on the server, per client.
#[derive(Default)]
pub struct ClientData<S: State> {
    /// Current knowledge of partitions.
    known: HashMap<S::PartitionId, PartitionKnowledge>,
}

/// A client's knowledge of a particular partition.
pub struct PartitionKnowledge {
    /// Starts at 0, counts up each tick.
    since: u8,
    /// Starts at and is refreshed to the expiry amount, counts down each tick.
    until: u8,
}

pub trait PassDef {
    /// The state the events are applied to.
    type State: State;
    /// The event produced and consumed by this pass.
    type Event: Clone + Serialize + DeserializeOwned;
    /// The relative priority of each event.
    type Priority: Ord;
    /// Iterator returned by `source_partition_ids`.
    type SourcePartitionIds: Iterator<Item = <Self::State as State>::PartitionId> + 'static =
        std::iter::Once<<Self::State as State>::PartitionId>;

    /// Return the priority of a given event.
    fn prioritize(event: &Self::Event) -> Self::Priority;

    /// Return whether a given event type should be collapsed to one after sorting. Should only
    /// consider event type, not value.
    fn collapse(event: &Self::Event) -> bool {
        let _ = event;
        false
    }

    /// Return which partition an event originated from. Clients with all these partitions are able
    /// to predict such an event.
    fn source_partition_ids(event: &Self::Event) -> Self::SourcePartitionIds;

    /// Return which partition an event affects.
    fn destination_partition_id(event: &Self::Event) -> <Self::State as State>::PartitionId;

    /// Apply an event to the state.
    fn apply(
        state: &mut Self::State,
        event: Self::Event,
        on_info: impl FnMut(<Self::State as State>::Info),
    );

    /// Some behavior that may produce events and/or info.
    fn tick(
        state: &mut Self::State,
        on_event: impl FnMut(Self::Event),
        on_info: impl FnMut(<Self::State as State>::Info),
    );

    /// Sorts the events to ensure determinism.
    fn sort(events: &mut [Self::Event]) {
        events.sort_unstable_by(|a, b| {
            Self::prioritize(a)
                .cmp(&Self::prioritize(b))
                .then_with(|| Self::source_partition_ids(a).cmp(Self::source_partition_ids(b)))
        });
    }

    /// Applies the events from the iterator (must be sorted), collapsing them when necessary.
    fn apply_all(
        state: &mut Self::State,
        events: impl IntoIterator<Item = Self::Event>,
        mut on_info: impl FnMut(<Self::State as State>::Info),
    ) {
        let mut events = events.into_iter().peekable();
        while let Some(event) = events.next() {
            if Self::collapse(&event) {
                if let Some(next) = events.peek() {
                    if Self::destination_partition_id(&event)
                        == Self::destination_partition_id(&next)
                        && mem::discriminant(&event) == mem::discriminant(next)
                    {
                        continue;
                    }
                }
            }
            Self::apply(state, event, &mut on_info);
        }
    }
}

/// Behavior + event storage if needed.
pub trait Pass {
    /// The corresponding state.
    type State: State;
    /// Update for pass this and, recursively, all previous passes.
    type Update: Serialize + DeserializeOwned;

    /// If server_update is `Some`, regarded as a client tick. Otherwise regarded a server tick.
    /// on_hash is called during the most appropriate time to hash the state from a client's perspective.
    fn tick(
        &mut self,
        state: &mut Self::State,
        update: Option<Self::Update>,
        on_info: impl FnMut(<Self::State as State>::Info),
    );

    /// Called on the server to get update to pass to clients.
    fn update(&self, client_data: &ClientData<Self::State>) -> Self::Update;
}

/// Implements [`Pass`] for [`PassDef`].
pub struct PassContext<S: State, PD: PassDef<State = S>, P: Pass<State = S> = PhantomData<S>> {
    /// The previous pass and, recursively, all previous passes.
    prev: P,
    /// On the server, this contains events that were immediately applied but possibly need echoing.
    /// On the client, this is a scratch allocation, used during ticks.
    pending: Vec<PD::Event>,
    _spooky: PhantomData<P>,
}

impl<S: State, PD: PassDef<State = S>, P: Pass<State = S> + Default> Default
    for PassContext<S, PD, P>
{
    fn default() -> Self {
        Self {
            prev: P::default(),
            pending: Vec::new(),
            _spooky: PhantomData,
        }
    }
}

impl<S: State, PD: PassDef<State = S>, P: Pass<State = S>> Deref for PassContext<S, PD, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.prev
    }
}

impl<S: State, PD: PassDef<State = S>, P: Pass<State = S>> DerefMut for PassContext<S, PD, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.prev
    }
}

impl<S: State, PD: PassDef<State = S>, P: Pass<State = S>> Pass for PassContext<S, PD, P> {
    type State = S;
    type Update = (P::Update, Vec<PD::Event>);

    fn tick(
        &mut self,
        state: &mut Self::State,
        update: Option<Self::Update>,
        mut on_info: impl FnMut(S::Info),
    ) {
        let pending = &mut self.pending;
        let server_update = if let Some((prev_update, events)) = update {
            pending.extend(events);
            Some(prev_update)
        } else {
            // Make room for new events.
            pending.clear();
            None
        };

        let server = server_update.is_none();
        self.prev.tick(state, server_update, &mut on_info);

        PD::tick(state, |event| pending.push(event), &mut on_info);

        PD::sort(pending);
        if server {
            PD::apply_all(state, pending.iter().cloned(), on_info);
        } else {
            PD::apply_all(state, pending.drain(..), on_info);
        }
    }

    fn update(&self, client_data: &ClientData<Self::State>) -> Self::Update {
        let prev = self.prev.update(client_data);
        let local = self
            .pending
            .iter()
            .filter(|event| {
                client_data
                    .known
                    .get(&PD::destination_partition_id(event))
                    .map(|k| k.since > 0)
                    .unwrap_or(false)
                    && PD::source_partition_ids(event).any(|partition_id| {
                        client_data
                            .known
                            .get(&partition_id)
                            .map(|k| k.since == 0 || k.until == 0)
                            .unwrap_or(true)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        (prev, local)
    }
}

// The no-op [`Pass`] at the end of the recursive passes.
impl<S: State> Pass for PhantomData<S> {
    type State = S;
    type Update = ();

    fn tick(&mut self, _: &mut Self::State, _: Option<Self::Update>, _: impl FnMut(S::Info)) {}

    fn update(&self, _: &ClientData<Self::State>) -> Self::Update {
        ()
    }
}

/// State in need of network synchronization.
pub trait State: Sized {
    /// Identifies disjoint subsets of the state.
    type PartitionId: Copy + Ord + Hash + Serialize + DeserializeOwned + 'static;
    /// A disjoint subset of the state.
    type Partition: Serialize + DeserializeOwned;
    /// An informational event produced but never sent over the network.
    type Info;
    /// A state-affecting event produced and, if needed, sent over the network.
    type Event: Clone + Serialize + DeserializeOwned;
    /// A checksum to verify synchronization. May be:
    ///  - [()] (no checksum)
    ///  - [u32] (hash checksum)
    ///  - [BTreeMap<Self::PartitionId, Self::Partition>] (complete checksum)
    ///  - Custom [Checksum] implementation.
    type Checksum: Checksum<Self> = ();

    /// How many ticks to send updates after a partition is no longer visible.
    const PARTITION_KEEPALIVE: u8 = 5;
    /// How many completes may be sent per tick.
    const COMPLETE_QUOTA: usize = usize::MAX;

    /// Return the affected partition.
    fn destination_partition_id(event: &Self::Event) -> Self::PartitionId;

    /// Visit all [PartitionId]s present in the state (not all theoretical [PartitionId]s)
    // TODO: Convert to `Iterator` when RPITIT is ready.
    fn visit_partition_ids(&self, visitor: impl FnMut(Self::PartitionId));

    /// Lookup the contents of a partition.
    fn get_partition(&self, partition_id: Self::PartitionId) -> Option<Self::Partition>;

    /// Hash the contents of a partition. Do this more efficiently if possible.
    fn hash_partition<H: Hasher>(&self, partition_id: Self::PartitionId, state: &mut H)
    where
        Self::Checksum: BitXorAssign;

    /// Set the contents of a partition, returning the old value.
    fn insert_partition(
        &mut self,
        partition_id: Self::PartitionId,
        partition: Self::Partition,
    ) -> Option<Self::Partition>;

    fn remove_partition(&mut self, partition_id: Self::PartitionId) -> Option<Self::Partition>;

    /// Applies an event to the state.
    fn apply(&mut self, event: Self::Event, on_info: impl FnMut(Self::Info));
}

impl<S: State, P: Pass<State = S>> World<S, P, Server<S>> {
    pub fn dispatch(&mut self, event: S::Event, on_info: impl FnMut(S::Info)) {
        self.role.pending.push(event.clone());
        self.state.apply(event, on_info);
    }

    pub fn update(
        &self,
        client_data: &mut ClientData<S>,
        visibility: impl IntoIterator<Item = S::PartitionId>,
    ) -> Update<S, P> {
        let mut completes = Vec::<(S::PartitionId, S::Partition)>::new();

        for partition_id in visibility {
            match client_data.known.entry(partition_id) {
                Entry::Occupied(mut occupied) => {
                    occupied.insert(PartitionKnowledge {
                        since: occupied.get().since,
                        until: S::PARTITION_KEEPALIVE,
                    });
                }
                Entry::Vacant(vacant) => {
                    if completes.len() >= S::COMPLETE_QUOTA {
                        continue;
                    }
                    vacant.insert(PartitionKnowledge {
                        since: 0,
                        until: S::PARTITION_KEEPALIVE,
                    });
                    completes.push((
                        partition_id,
                        self.state
                            .get_partition(partition_id)
                            .expect("missing visible partition"),
                    ));
                }
            }
        }

        let update = self.pass.update(client_data);

        let events = self
            .role
            .pending
            .iter()
            .filter(|event| {
                client_data
                    .known
                    .get(&S::destination_partition_id(event))
                    .map(|k| k.until > 0 && k.since > 0)
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut deletes = Vec::<S::PartitionId>::new();
        let mut checksum = S::Checksum::default();
        client_data.known.retain(|&partition_id, keepalive| {
            if let Some(new) = keepalive.until.checked_sub(1) {
                *keepalive = PartitionKnowledge {
                    since: keepalive.since.saturating_add(1),
                    until: new,
                };
                checksum.accumulate(partition_id, &self.state);
                true
            } else {
                deletes.push(partition_id);
                false
            }
        });

        Update {
            deletes,
            update,
            events,
            completes,
            checksum,
        }
    }

    pub fn tick(&mut self, on_info: impl FnMut(S::Info)) {
        self.role.pending.clear();
        self.pass.tick(&mut self.state, None, on_info);
    }
}

impl<S: State, P: Pass<State = S>> World<S, P, Client> {
    pub fn tick(&mut self, update: Update<S, P>, mut on_info: impl FnMut(S::Info)) {
        let Update {
            deletes,
            update,
            events,
            completes,
            checksum: expected_checksum,
        } = update;

        for partition_id in deletes {
            self.state
                .remove_partition(partition_id)
                .expect("missing removed partition");
        }

        self.pass.tick(&mut self.state, Some(update), &mut on_info);

        for event in events {
            self.state.apply(event, &mut on_info);
        }

        for (partition_id, complete) in completes {
            assert!(
                self.state
                    .insert_partition(partition_id, complete)
                    .is_none(),
                "complete replaced existing chunk"
            );
        }

        let mut checksum = S::Checksum::default();
        self.state.visit_partition_ids(|partition_id| {
            checksum.accumulate(partition_id, &self.state);
        });
        assert_eq!(checksum, expected_checksum, "desync");
    }
}

/// [`Client`] or [`Server`]; may contain role-specific data.
pub trait Role {}

struct Server<S: State> {
    /// Dispatched events that must be echoed to clients.
    pending: Vec<S::Event>,
}

impl<S: State> Default for Server<S> {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
        }
    }
}

impl<S: State> Role for Server<S> {}

#[derive(Default)]
struct Client;

impl Role for Client {}

/// Helps verify the synchronization is working.
pub trait Checksum<S: State>: Debug + Eq + Default + Serialize + DeserializeOwned {
    /// Add a partition ot the checksum (or no-op for `()` checksum).
    /// Order should not matter.
    fn accumulate(&mut self, partition_id: S::PartitionId, state: &S);
}

impl<S: State> Checksum<S> for () {
    fn accumulate(&mut self, _: S::PartitionId, _: &S) {
        // No-op
    }
}

impl<S: State> Checksum<S> for u32
where
    S::Checksum: BitXorAssign,
{
    fn accumulate(&mut self, partition_id: S::PartitionId, state: &S) {
        let mut hasher = CompatHasher::default();
        partition_id.hash(&mut hasher);
        state.hash_partition(partition_id, &mut hasher);
        *self ^= hasher.finish() as u32
    }
}

impl<S: State> Checksum<S> for BTreeMap<S::PartitionId, S::Partition>
where
    S::PartitionId: Debug,
    S::Partition: Eq + Debug,
{
    fn accumulate(&mut self, partition_id: S::PartitionId, state: &S) {
        self.insert(
            partition_id,
            state
                .get_partition(partition_id)
                .expect("missing partition in checksum"),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::{Client, ClientData, PassContext, PassDef, Server, State, World};
    use rand::prelude::IteratorRandom;
    use rand::{thread_rng, Rng};
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, HashMap};
    use std::marker::PhantomData;

    #[test]
    fn fuzz() {
        #[derive(Default)]
        struct SimpleState {
            partitions: HashMap<u8, String>,
        }

        #[derive(Default)]
        struct SimplePass;

        impl PassDef for SimplePass {
            type State = SimpleState;
            type Event = SimplePassEvent;
            type Priority = usize;

            fn prioritize(event: &Self::Event) -> Self::Priority {
                match event {
                    SimplePassEvent::PushChar { .. } => 0,
                    SimplePassEvent::PopChar { .. } => 0,
                    SimplePassEvent::Overwrite { string, .. } => string.len().saturating_add(1),
                }
            }

            fn source_partition_ids(event: &Self::Event) -> Self::SourcePartitionIds {
                std::iter::once(match event {
                    SimplePassEvent::PushChar {
                        source_partition_id,
                        ..
                    } => *source_partition_id,
                    SimplePassEvent::PopChar { partition_id, .. }
                    | SimplePassEvent::Overwrite { partition_id, .. } => *partition_id,
                })
            }

            fn destination_partition_id(
                event: &Self::Event,
            ) -> <SimpleState as State>::PartitionId {
                match event {
                    SimplePassEvent::PushChar {
                        destination_partition_id,
                        ..
                    } => *destination_partition_id,
                    SimplePassEvent::PopChar { partition_id, .. }
                    | SimplePassEvent::Overwrite { partition_id, .. } => *partition_id,
                }
            }

            fn collapse(event: &Self::Event) -> bool {
                matches!(event, SimplePassEvent::Overwrite { .. })
            }

            fn apply(
                state: &mut SimpleState,
                event: Self::Event,
                mut on_info: impl FnMut(<SimpleState as State>::Info),
            ) {
                match event {
                    SimplePassEvent::PushChar {
                        destination_partition_id,
                        c,
                        ..
                    } => {
                        let Some(partition) = state.partitions.get_mut(&destination_partition_id) else {
                            return;
                        };
                        partition.push(c);
                        on_info(SimpleInfo::CharPushed {
                            partition_id: destination_partition_id,
                            c,
                            new: partition.clone(),
                        })
                    }
                    SimplePassEvent::PopChar { partition_id } => {
                        let Some(partition) = state.partitions.get_mut(&partition_id) else {
                            return;
                        };
                        on_info(SimpleInfo::CharPopped {
                            partition_id,
                            c: partition.pop(),
                            new: partition.clone(),
                        });
                    }
                    SimplePassEvent::Overwrite {
                        partition_id,
                        string,
                    } => {
                        let Some(partition) = state.partitions.get_mut(&partition_id) else {
                            return;
                        };
                        *partition = string;
                        on_info(SimpleInfo::Overwritten {
                            partition_id,
                            new: partition.clone(),
                        });
                    }
                }
            }

            fn tick(
                state: &mut Self::State,
                mut on_event: impl FnMut(Self::Event),
                mut on_info: impl FnMut(<SimpleState as State>::Info),
            ) {
                for (&partition_id, string) in &mut state.partitions {
                    if string.len() % 3 == 0 {
                        string.push('m');
                        on_info(SimpleInfo::CharPushed {
                            partition_id,
                            c: 'm',
                            new: string.clone(),
                        });
                    } else {
                        on_info(SimpleInfo::CharPopped {
                            partition_id,
                            c: string.pop(),
                            new: string.clone(),
                        });
                    }

                    if partition_id % 4 == 0 {
                        on_event(SimplePassEvent::Overwrite {
                            partition_id,
                            string: String::from("ABCDE"),
                        });
                    }
                    if partition_id % 8 == 0 {
                        on_event(SimplePassEvent::Overwrite {
                            partition_id,
                            string: String::from("________"),
                        });
                    }
                    if partition_id % 3 == 0 {
                        on_event(SimplePassEvent::PushChar {
                            source_partition_id: partition_id,
                            destination_partition_id: partition_id,
                            c: 'a',
                        });
                    } else if string.len() % 7 == 0 {
                        on_event(SimplePassEvent::Overwrite {
                            partition_id,
                            string: String::from("abcd"),
                        });
                    } else {
                        on_event(SimplePassEvent::PopChar { partition_id });
                        on_event(SimplePassEvent::PushChar {
                            source_partition_id: partition_id,
                            destination_partition_id: partition_id.saturating_sub(1),
                            c: 'b',
                        });
                    }
                }
            }
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        enum SimpleStateEvent {
            AddString { partition_id: u8, string: String },
            PushChar { partition_id: u8, c: char },
        }

        #[derive(Clone, Debug, Serialize, Deserialize)]
        enum SimplePassEvent {
            PushChar {
                source_partition_id: u8,
                destination_partition_id: u8,
                c: char,
            },
            PopChar {
                partition_id: u8,
            },
            Overwrite {
                partition_id: u8,
                string: String,
            },
        }

        #[derive(Debug)]
        #[allow(unused)]
        enum SimpleInfo {
            CharPushed {
                partition_id: u8,
                c: char,
                new: String,
            },
            CharPopped {
                partition_id: u8,
                c: Option<char>,
                new: String,
            },
            Overwritten {
                partition_id: u8,
                new: String,
            },
        }

        impl State for SimpleState {
            type PartitionId = u8;
            type Partition = String;
            type Info = SimpleInfo;
            type Event = SimpleStateEvent;
            type Checksum = BTreeMap<Self::PartitionId, Self::Partition>;

            fn destination_partition_id(event: &Self::Event) -> Self::PartitionId {
                match event {
                    SimpleStateEvent::AddString { partition_id, .. }
                    | SimpleStateEvent::PushChar { partition_id, .. } => *partition_id,
                }
            }

            fn visit_partition_ids(&self, visitor: impl FnMut(Self::PartitionId)) {
                self.partitions.keys().copied().for_each(visitor);
            }

            fn get_partition(&self, partition_id: Self::PartitionId) -> Option<Self::Partition> {
                self.partitions.get(&partition_id).cloned()
            }

            fn insert_partition(
                &mut self,
                partition_id: Self::PartitionId,
                partition: Self::Partition,
            ) -> Option<Self::Partition> {
                self.partitions.insert(partition_id, partition)
            }

            fn remove_partition(
                &mut self,
                partition_id: Self::PartitionId,
            ) -> Option<Self::Partition> {
                self.partitions.remove(&partition_id)
            }

            fn apply(&mut self, event: Self::Event, mut on_info: impl FnMut(Self::Info)) {
                match event {
                    SimpleStateEvent::AddString {
                        partition_id,
                        string,
                    } => {
                        self.partitions.insert(partition_id, string);
                    }
                    SimpleStateEvent::PushChar { partition_id, c } => {
                        let partition = self.partitions.get_mut(&partition_id).unwrap();
                        partition.push(c);
                        on_info(SimpleInfo::CharPushed {
                            partition_id,
                            c,
                            new: partition.clone(),
                        });
                    }
                }
            }
        }

        /*
        type SimplePasses =
            PassContext<SimpleState, SimplePass>;
        */
        type SimplePasses =
            PassContext<SimpleState, SimplePass, PassContext<SimpleState, SimplePass>>;

        struct MockClient {
            world: World<SimpleState, SimplePasses, Client>,
            data: ClientData<SimpleState>,
        }

        impl Default for MockClient {
            fn default() -> Self {
                Self {
                    world: World::<SimpleState, PhantomData<SimpleState>, Client>::default()
                        .add_pass::<SimplePass>()
                        .add_pass::<SimplePass>(),
                    data: ClientData::default(),
                }
            }
        }

        fn print_info(info: SimpleInfo) {
            println!("Info: {:?}", info);
        }

        let update_clients =
            |server: &mut World<SimpleState, SimplePasses, Server<SimpleState>>,
             clients: &mut [MockClient]| {
                let n_clients = clients.len();
                let mut rng = thread_rng();
                for (i, client) in clients.iter_mut().enumerate() {
                    let visibility = server
                        .partitions
                        .keys()
                        .copied()
                        .filter(|&n| {
                            rng.gen_bool(if n as usize % n_clients == i {
                                0.9
                            } else {
                                0.1
                            })
                        })
                        .collect::<Vec<_>>();
                    let update = server.update(&mut client.data, visibility);
                    let has = &client.world.partitions;
                    println!("{i} has {has:?} gets {update:?}");
                    client.world.tick(update, print_info);
                }
            };

        let mut rng = thread_rng();
        let isolate = false;

        for i in 0..512 {
            println!("@@@@@@@@@@@@@@@@@@@@@@@@ FUZZ #{i}");

            let mut server = World::<SimpleState, SimplePasses, Server<SimpleState>>::default();
            let mut clients = std::iter::repeat_with(MockClient::default)
                .take(if isolate { 1 } else { rng.gen_range(0..=32) })
                .collect::<Vec<_>>();

            let mut possible_partitions = if isolate {
                vec![22, 23]
            } else {
                (0..32).collect::<Vec<_>>()
            };

            for j in 0..rng.gen_range(1..=16) {
                println!("@@@@@@@@@@@@@@@ ITERATION #{j}");

                println!("@@@@@@@ DISPATCH");

                for _ in 0..rng.gen_range(0..=4) {
                    if !possible_partitions.is_empty() {
                        let i = rng.gen_range(0..possible_partitions.len());
                        let partition_id = possible_partitions.swap_remove(i);
                        server.dispatch(
                            SimpleStateEvent::AddString {
                                partition_id,
                                string: i.to_string(),
                            },
                            print_info,
                        );
                    }
                }

                println!("@@@@@@@ DISPATCH 2");

                if !server.partitions.is_empty() {
                    for _ in 0..rng.gen_range(0..=if isolate { 3 } else { 25 }) {
                        server.dispatch(
                            SimpleStateEvent::PushChar {
                                partition_id: server
                                    .partitions
                                    .keys()
                                    .choose(&mut rng)
                                    .unwrap()
                                    .clone(),
                                c: rng.gen_range('0'..='9'),
                            },
                            print_info,
                        );
                    }
                }

                println!("@@@@@@@ UPDATE CLIENTS");

                update_clients(&mut server, &mut clients);

                println!("@@@@@@@ TICK: {:?}", server.state.partitions);

                server.tick(print_info);
            }
        }
    }
}
