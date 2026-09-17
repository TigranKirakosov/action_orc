#[cfg(feature = "orchestrator")]
mod orchestrator;
mod reactor;
mod schedule;

use action_orc_core::NodeId;
#[cfg(feature = "orchestrator")]
pub use orchestrator::*;
pub use reactor::*;
pub use schedule::ScheduleDirective;

#[cfg(test)]
mod tests;

type NodeEvent = (NodeId, NodeStatus);

#[derive(Clone, Copy)]
pub struct NodeCommand {
    pub schedule_directive: ScheduleDirective,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    /// Once a node got this status, it **obliged** to eventually call [Reactor::resolve]
    /// in order drive schedule advancement. Otherwise, whole **engine will stall**.
    Started,
    Resolved(Resolution),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    Finished,
    Reset,
}
