use action_orc_core::*;
use std::fmt;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

use crate::NodeCommand;
use crate::{NodeStatus, schedule::Schedule};

#[derive(Debug)]
pub enum ReactorError {
    MissingListener,
    UnknownTypeId,
}

impl fmt::Display for ReactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReactorError::MissingListener => {
                write!(f, "Reactor error: Missing listener registration.")
            }
            ReactorError::UnknownTypeId => write!(
                f,
                "Reactor error: Attempted to process an unknown TypeId layout."
            ),
        }
    }
}

impl std::error::Error for ReactorError {}

#[derive(Default)]
pub struct Reactor {
    pub(crate) schedule: Schedule,
    pub(crate) listeners: HashMap<TypeId, Vec<Box<dyn Listener>>>,
}

impl Reactor {
    /// Build a reactor based on a configured graph.
    ///
    /// Once given a [Graph], reactor is tied to it
    /// and will treat it as a static blueprint for underlying [Schedule].
    pub fn from<'a, G: AsGraphEntryProxy<'a>>(layout: G) -> Self {
        let graph = layout.into_compiled_graph();
        let schedule = Schedule::from(graph);

        Self {
            schedule,
            listeners: HashMap::new(),
        }
    }

    /// Start underlying [Schedule] and notify [Listener]s which nodes
    /// have started, that is, received control over schedule advancement.
    pub fn start(&mut self) -> Result<(), ReactorError> {
        for (node, status) in self.schedule.start() {
            self.notify(node, status)?;
        }

        Ok(())
    }

    /// Resets underlying [Schedule] to defaults and does the sames as [Self::init]
    pub fn restart(&mut self) -> Result<(), ReactorError> {
        for (node, status) in self.schedule.restart() {
            self.notify(node, status)?;
        }

        Ok(())
    }

    /// Register [Listener] for [TypeId] lifecycle statuses.
    ///
    /// Will return [ReactorError::UnknownTypeId] on attempt to listen for non-present type within underlying [Graph].
    pub fn listen_for(
        &mut self,
        type_id: TypeId,
        listener: impl Listener,
    ) -> Result<(), ReactorError> {
        let type_exists = self
            .schedule
            .graph
            .meta()
            .iter()
            .any(|m| *m.type_id() == type_id);
        if !type_exists {
            return Err(ReactorError::UnknownTypeId);
        }

        self.listeners
            .entry(type_id)
            .or_default()
            .push(Box::new(listener));
        Ok(())
    }

    /// Evaluates node command.
    ///
    /// Will error [ReactorError::MissingListener] if some node does not have registered [Listener]
    /// to receive control over schedule advancement.
    pub fn resolve(&mut self, command: NodeCommand) -> Result<bool, ReactorError> {
        let (state, node_payload) = self.schedule.process(command.schedule_directive);

        for (id, event) in node_payload {
            self.notify(id, event)?;
        }

        Ok(state)
    }

    /// An ordered mapping of underlying graph [NodeId]s to respective [Meta]
    pub fn node_meta(&self) -> Vec<(NodeId, &Meta)> {
        self.schedule.graph.meta().iter().enumerate().collect()
    }

    fn notify(&self, id: NodeId, event: NodeStatus) -> Result<(), ReactorError> {
        let meta = &self.schedule.graph.meta()[id];

        let typed_observers = self
            .listeners
            .get(meta.type_id())
            .ok_or(ReactorError::MissingListener)?;

        for obs in typed_observers {
            obs.notify(id, event);
        }

        Ok(())
    }
}

pub trait Listener: Send + Sync + 'static {
    fn notify(&self, id: NodeId, event: NodeStatus);
}

impl<F> Listener for F
where
    F: Fn(NodeId, NodeStatus) + Send + Sync + 'static,
{
    fn notify(&self, id: NodeId, event: NodeStatus) {
        self(id, event);
    }
}

impl<O> Listener for Arc<O>
where
    O: Listener + ?Sized,
{
    fn notify(&self, id: NodeId, event: NodeStatus) {
        (**self).notify(id, event);
    }
}
