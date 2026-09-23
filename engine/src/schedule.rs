use crate::{NodeEvent, NodeStatus, Resolution};
use action_orc_core::*;

/// # Rules
/// ## Resolving
/// Any node that is **not** marked with [UpstreamRole::Selector] allowed issue [ScheduleDirective::Resolve]
///
/// ## Advancing
/// Only node that is marked with [UpstreamRole::Selector] allowed to issue [ScheduleDirective::Advance] with [ScheduleDirective::Advance::target]
///
/// ### Valid
/// `X -> (A ? B)`
/// - X has [UpstreamRole::Selector] due to arrow pointing to a selection group (each node has [DownstreamRole::SelectionMember])
///
/// ### Invalid
/// `X -> Y` nor `X -> (A | B)` nor `X -> (A, B)`
/// - X has no [UpstreamRole::Selector]
///
/// ## Backtracking
/// ### Valid
/// `X -> (A ? B) -> Y`
/// - can backtrack from either A or B to X, as X is a single node
/// - can backtrack from Y to either A or B - selection was already registered in [Schedule::history]
///
/// `(X, Y) -> (A ? B)`
/// - can backtrack from either A or B to Y, as sequence (X, Y) has clear last Y node
///
/// ### Invalid
/// `(X | Y) -> (A ? B)`
/// - X and Y might want to select different paths, possible conflict
///
/// `(A ? B) -> (Z | W)`
/// - Z and W run in parallel; letting either node backtrack independently would corrupt the peer's active execution state
#[derive(Clone, Copy)]
pub enum ScheduleDirective {
    Resolve { pivot: NodeId },
    Advance { pivot: NodeId, target: NodeId },
    Backtrack,
}

#[derive(Debug, PartialEq)]
pub enum SchedulerError {
    AdvanceBypass {
        pivot: NodeDisplay,
    },
    InvalidAdvanceSource {
        source: NodeDisplay,
        target: NodeDisplay,
    },
    InvalidAdvanceTarget {
        source: NodeDisplay,
        target: NodeDisplay,
    },
    NowhereToBacktrack,
    BacktrackToForkMember {
        from: NodeDisplay,
        to: NodeDisplay,
    },
    BacktrackFromForkMember {
        from: NodeDisplay,
        to: NodeDisplay,
    },
}

#[derive(Debug, PartialEq)]
pub struct NodeDisplay {
    id: NodeId,
    name: &'static str,
}

pub(crate) struct Schedule {
    pub(crate) graph: Box<dyn GraphLayout>,
    pub(crate) in_degree: Vec<usize>,
    pub(crate) finished_count: usize,
    pub(crate) history: Vec<NodeId>,
}

impl Schedule {
    pub(crate) fn from(graph: Box<dyn GraphLayout>) -> Self {
        let in_degree = graph.in_degree();

        Self {
            in_degree: in_degree.to_vec(),
            finished_count: in_degree.iter().filter(|deg| **deg == 0).count(),
            history: Vec::new(),
            graph,
        }
    }

    pub(crate) fn start(&mut self) -> Vec<NodeEvent> {
        self.graph
            .sources()
            .into_iter()
            .map(|source| {
                self.history.push(source);
                (source, NodeStatus::Started)
            })
            .collect()
    }

    pub(crate) fn restart(&mut self) -> Vec<NodeEvent> {
        let in_degree = self.graph.in_degree();
        self.in_degree.copy_from_slice(in_degree);
        self.finished_count = in_degree.iter().filter(|deg| **deg == 0).count();
        self.history.clear();
        self.start()
    }

    fn validate(&self, directive: ScheduleDirective) -> Result<(), SchedulerError> {
        match directive {
            ScheduleDirective::Resolve { pivot } => {
                let meta = &self.graph.meta()[pivot];
                if meta.role_us() == UpstreamRole::Selector {
                    Err(SchedulerError::AdvanceBypass {
                        pivot: NodeDisplay::from(pivot, meta),
                    })
                } else {
                    Ok(())
                }
            }
            ScheduleDirective::Advance { pivot, target } => {
                let source_meta = &self.graph.meta()[pivot];
                let target_meta = &self.graph.meta()[target];

                if source_meta.role_us() != UpstreamRole::Selector {
                    return Err(SchedulerError::InvalidAdvanceSource {
                        source: NodeDisplay::from(pivot, source_meta),
                        target: NodeDisplay::from(target, target_meta),
                    });
                }

                if target_meta.role_ds() != DownstreamRole::SelectionMember {
                    return Err(SchedulerError::InvalidAdvanceTarget {
                        source: NodeDisplay::from(pivot, source_meta),
                        target: NodeDisplay::from(target, target_meta),
                    });
                }

                if !self.graph.adj()[pivot].contains(&target) {
                    return Err(SchedulerError::InvalidAdvanceTarget {
                        source: NodeDisplay::from(pivot, source_meta),
                        target: NodeDisplay::from(target, target_meta),
                    });
                }

                Ok(())
            }
            ScheduleDirective::Backtrack => {
                if self.history.len() < 2 {
                    return Err(SchedulerError::NowhereToBacktrack);
                }

                let current = *self.history.last().unwrap();
                let current_meta = &self.graph.meta()[current];

                let parent = self.history[self.history.len() - 2];
                let parent_meta = &self.graph.meta()[parent];

                if current_meta.role_ds() == DownstreamRole::ForkMember {
                    return Err(SchedulerError::BacktrackFromForkMember {
                        from: NodeDisplay::from(current, current_meta),
                        to: NodeDisplay::from(parent, parent_meta),
                    });
                }

                if parent_meta.role_us() == UpstreamRole::Selector
                    && current_meta.role_ds() == DownstreamRole::SelectionMember
                {
                    return Ok(());
                }

                if parent_meta.role_ds() == DownstreamRole::ForkMember {
                    return Err(SchedulerError::BacktrackToForkMember {
                        from: NodeDisplay::from(current, current_meta),
                        to: NodeDisplay::from(parent, parent_meta),
                    });
                }

                Ok(())
            }
        }
    }

    pub(crate) fn process(
        &mut self,
        directive: ScheduleDirective,
    ) -> Result<(bool, Vec<NodeEvent>), SchedulerError> {
        self.validate(directive)?;

        let mut queue = Vec::new();

        match directive {
            ScheduleDirective::Resolve { pivot } => {
                queue.push((pivot, NodeStatus::Resolved(Resolution::Finished)));
                self.history.push(pivot);

                // Resolve downstream nodes
                for &ds in &self.graph.adj()[pivot] {
                    self.in_degree[ds] = self.in_degree[ds].saturating_sub(1);
                    if self.in_degree[ds] == 0 {
                        self.finished_count += 1;

                        queue.push((ds, NodeStatus::Started));
                        self.history.push(ds);
                    }
                }
            }
            ScheduleDirective::Advance { pivot, target } => {
                queue.push((pivot, NodeStatus::Resolved(Resolution::Finished)));

                // Topology invariant: select group in-degree is always one due to single pivot
                // hence setting straight to 0 on pivot resolution
                self.in_degree[target] = 0;
                self.finished_count += 1;
                queue.push((target, NodeStatus::Started));
                self.history.push(target);

                // Eliminate unselected routes
                for &sibling in &self.graph.adj()[pivot] {
                    if sibling != target {
                        self.finished_count += 1;

                        // Sibling is dead, reduce downstream in-degree
                        for &ds in &self.graph.adj()[sibling] {
                            self.in_degree[ds] = self.in_degree[ds].saturating_sub(1);
                        }
                    }
                }
            }
            ScheduleDirective::Backtrack => {
                let current = self.history.pop().expect("Guarded by validator.");
                let &parent = self.history.last().expect("Guarded by validator");
                self.finished_count = self.finished_count.saturating_sub(1);
                self.in_degree[current] = self.graph.in_degree()[current];

                // Restore sibling
                for &sibling in &self.graph.adj()[parent] {
                    if sibling != current {
                        self.finished_count = self.finished_count.saturating_sub(1);

                        // Sibling restored, restore downstream in-degree
                        for &ds in &self.graph.adj()[sibling] {
                            self.in_degree[ds] += 1;
                        }
                    }
                }

                queue.push((current, NodeStatus::Resolved(Resolution::Reset)));
                queue.push((parent, NodeStatus::Started));
                self.history.push(parent);
            }
        }

        let is_complete = self.in_degree.len() == self.finished_count;

        Ok((is_complete, queue))
    }
}

impl std::fmt::Display for NodeDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { id, name } = self;
        write!(f, "[{name} : {id}]")
    }
}

impl NodeDisplay {
    pub(crate) fn from(id: NodeId, meta: &Meta) -> Self {
        Self {
            id,
            name: meta.type_name(),
        }
    }
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scheduler error: ")?;

        match self {
            SchedulerError::AdvanceBypass { pivot } => {
                write!(
                    f,
                    "Attempt to resolve node {pivot} without advancing directive."
                )
            }
            SchedulerError::InvalidAdvanceSource { source, target } => {
                write!(
                    f,
                    "Cannot use advance directive from regular node {source} to {target}. Use Resolve instead."
                )
            }
            SchedulerError::InvalidAdvanceTarget { source, target } => {
                write!(
                    f,
                    "Node {target} is not a valid branch target for selector node {source}."
                )
            }
            SchedulerError::NowhereToBacktrack => {
                write!(f, "Attempt to backtrack to non-existent point in history.")
            }
            SchedulerError::BacktrackToForkMember { from, to } => {
                write!(f, "Attempt to backtrack to Fork member {to} from {from}")
            }
            SchedulerError::BacktrackFromForkMember { from, to } => {
                write!(f, "Attempt to backtrack from Fork member {from} to {to}")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}
