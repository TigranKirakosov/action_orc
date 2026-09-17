use std::{
    any::TypeId,
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
};

use action_orc_core::{AsGraphEntryProxy, Meta, NodeId};

use crate::{Listener, NodeCommand, NodeEvent, NodeStatus, Reactor, ReactorError};

#[derive(Default, Clone)]
struct EventQueue(Arc<Mutex<VecDeque<NodeEvent>>>);

impl Listener for EventQueue {
    fn notify(&self, id: NodeId, event: NodeStatus) {
        if let Ok(mut guard) = self.0.lock() {
            guard.push_back((id, event));
        }
    }
}

#[derive(Clone)]
pub enum LoopDirective {
    Loop,
    Break,
}

#[derive(Clone)]
pub struct ScheduleDirectives {
    pub loop_directive: Option<LoopDirective>,
}

pub struct Config {
    pub loop_schedule: bool,
}

#[derive(PartialEq)]
pub enum State {
    Active,
    Ended,
    Restarted,
}

struct CommandsChannel {
    tx: Sender<NodeCommand>,
    rx: Mutex<Receiver<NodeCommand>>,
}

pub struct Orchestrator {
    reactor: Reactor,
    event_queue: EventQueue,
    commands_channel: CommandsChannel,
    config: Config,
    /// Indicates count of currently processing events by outside world
    in_flight: usize,
}

impl Orchestrator {
    pub fn new<'a, G: AsGraphEntryProxy<'a>>(
        graph: G,
        config: Config,
    ) -> Result<Self, ReactorError> {
        let commands_channel = CommandsChannel::new();
        let event_queue = EventQueue::default();
        let mut reactor = Reactor::from(graph);

        // Connect event queue to reactor events
        for type_id in reactor
            .node_meta()
            .iter()
            .map(|(_, m)| *m.type_id())
            .collect::<Vec<_>>()
        {
            reactor.listen_for(type_id, event_queue.clone())?;
        }

        Ok(Self {
            reactor,
            commands_channel,
            event_queue,
            in_flight: 0,
            config,
        })
    }

    pub fn start(&mut self) -> Result<(), ReactorError> {
        self.reactor.start()
    }

    /// Handle for writing node commands into [Orchestrator]
    pub fn resolver(&self) -> Sender<NodeCommand> {
        self.commands_channel.tx.clone()
    }

    pub fn register_listener(
        &mut self,
        type_id: TypeId,
        listener: impl Listener,
    ) -> Result<(), ReactorError> {
        self.reactor.listen_for(type_id, listener)
    }

    /// Must be polled
    pub fn tick(&mut self) -> Result<State, ReactorError> {
        let Ok(queue) = self.commands_channel.rx.lock() else {
            return Ok(State::Active);
        };

        for command in queue.try_iter() {
            self.reactor.resolve(command)?;
            self.in_flight = self.in_flight.saturating_sub(1);
        }

        let queue_is_empty = self
            .event_queue
            .0
            .lock()
            .map(|g| g.is_empty())
            .unwrap_or(false);

        if self.in_flight == 0 && queue_is_empty {
            return if self.config.loop_schedule {
                self.reactor.restart()?;
                Ok(State::Restarted)
            } else {
                Ok(State::Ended)
            };
        }

        Ok(State::Active)
    }

    /// Eagerly drains all currently available chronological events.
    pub fn drain_events(&mut self) -> VecDeque<NodeEvent> {
        let mut guard = match self.event_queue.0.lock() {
            Ok(g) => g,
            Err(_) => return VecDeque::new(),
        };

        let elements = std::mem::take(&mut *guard);
        drop(guard);

        for event in &elements {
            if let NodeStatus::Started = event.1 {
                self.in_flight += 1;
            }
        }

        elements
    }

    pub fn config_schedule(&mut self, directives: &ScheduleDirectives) {
        if let Some(loop_directive) = &directives.loop_directive {
            let loop_schedule = match loop_directive {
                LoopDirective::Loop => true,
                LoopDirective::Break => false,
            };
            self.config.loop_schedule = loop_schedule;
        }
    }

    /// An ordered mapping of underlying graph [NodeId]s to respective [Meta]
    pub fn node_meta(&self) -> Vec<(NodeId, &Meta)> {
        self.reactor.node_meta()
    }
}

impl CommandsChannel {
    fn new() -> Self {
        let (tx, rx) = channel();
        let rx = Mutex::new(rx);
        Self { tx, rx }
    }
}
