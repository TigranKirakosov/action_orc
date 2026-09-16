use std::{
    any::TypeId,
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
};

use action_orc_core::{AsGraphEntryProxy, Meta, NodeId};

use crate::{Listener, NodeEvent, NodeResolution, NodeStatus, Reactor, ReactorError};

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
    Maintain,
    Break,
}

#[derive(Clone)]
pub struct ScheduleDirectives {
    pub loop_directive: Option<LoopDirective>,
}

pub struct ScheduleConfig {
    pub should_loop: bool,
}

#[derive(PartialEq)]
pub enum ScheduleState {
    Active,
    Ended,
    Restarted,
}

pub struct Orchestrator {
    reactor: Reactor,
    event_queue: EventQueue,
    resolution_tx: Sender<NodeResolution>,
    resolution_rx: Mutex<Receiver<NodeResolution>>,
    schedule_config: ScheduleConfig,
    /// Indicates count of currently processing events by outside world
    in_flight: usize,
}

impl Orchestrator {
    pub fn new<'a, G: AsGraphEntryProxy<'a>>(
        graph: G,
        schedule_config: ScheduleConfig,
    ) -> Result<Self, ReactorError> {
        let (resolution_tx, resolution_rx) = channel();
        let resolution_rx = Mutex::new(resolution_rx);
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
            event_queue,
            resolution_tx,
            resolution_rx,
            schedule_config,
            in_flight: 0,
        })
    }

    pub fn start(&mut self) -> Result<(), ReactorError> {
        self.reactor.start()
    }

    /// Handle for writing node resolutions into [Orchestrator]
    pub fn resolver(&self) -> Sender<NodeResolution> {
        self.resolution_tx.clone()
    }

    pub fn register_listener(
        &mut self,
        type_id: TypeId,
        listener: impl Listener,
    ) -> Result<(), ReactorError> {
        self.reactor.listen_for(type_id, listener)
    }

    /// Must be polled
    pub fn tick(&mut self) -> Result<ScheduleState, ReactorError> {
        let Ok(queue) = self.resolution_rx.lock() else {
            return Ok(ScheduleState::Active);
        };

        for (id, resolution) in queue.try_iter() {
            self.reactor.resolve(id, resolution)?;
            self.in_flight = self.in_flight.saturating_sub(1);
        }

        let queue_is_empty = self
            .event_queue
            .0
            .lock()
            .map(|g| g.is_empty())
            .unwrap_or(false);

        if self.in_flight == 0 && queue_is_empty {
            return if self.schedule_config.should_loop {
                self.reactor.restart()?;
                Ok(ScheduleState::Restarted)
            } else {
                Ok(ScheduleState::Ended)
            };
        }

        Ok(ScheduleState::Active)
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
            let should_loop = match loop_directive {
                LoopDirective::Maintain => true,
                LoopDirective::Break => false,
            };
            self.schedule_config.should_loop = should_loop;
        }
    }

    /// An ordered mapping of underlying graph [NodeId]s to respective [Meta]
    pub fn node_meta(&self) -> Vec<(NodeId, &Meta)> {
        self.reactor.node_meta()
    }
}
