use crate::{NodeEvent, NodeStatus, Resolution};
use action_orc_core::*;

/// # Select/Backtrack rules
/// ## Valid
/// `X -> (A : B) -> Y`
/// - can backtrack from either A or B to X, as X is a single node
/// - can backtrack from Y to either A or B - selection was already registered in [Schedule::history]
///
/// `(X, Y) -> (A : B) -> Z`
/// - can backtrack from either A or B to Y, as sequence (X, Y) has clear last Y node
///
/// ## Invalid
/// `(X | Y) -> (A : B)`
/// - X and Y might want to select different paths, possible conflict
///
/// `(A : B) -> (Z | W)`
/// - Z and W run in parallel; letting either node backtrack independently would corrupt the peer's active execution state
pub enum ScheduleDirective {
    Advance { pivot: NodeId },
    SelectiveAdvance { pivot: NodeId, target: NodeId },
    Backtrack,
}

#[derive(Default)]
pub(crate) struct Schedule {
    pub(crate) graph: Graph,
    pub(crate) in_degree: Vec<usize>,
    pub(crate) finished_count: usize,
    pub(crate) history: Vec<NodeId>,
}

impl Schedule {
    pub(crate) fn from(graph: Graph) -> Self {
        let in_degree = graph.in_degree();

        Self {
            in_degree: in_degree.to_vec(),
            finished_count: in_degree.iter().filter(|deg| **deg == 0).count(),
            history: Vec::new(),
            graph,
        }
    }

    pub(crate) fn start(&self) -> Vec<NodeEvent> {
        self.graph
            .sources()
            .map(|source| (source, NodeStatus::Started))
            .collect()
    }

    pub(crate) fn restart(&mut self) -> Vec<NodeEvent> {
        let in_degree = self.graph.in_degree();
        self.in_degree.copy_from_slice(in_degree);
        self.finished_count = in_degree.iter().filter(|deg| **deg == 0).count();
        self.history.clear();
        self.start()
    }

    pub(crate) fn process(&mut self, directive: ScheduleDirective) -> (bool, Vec<NodeEvent>) {
        let mut queue = Vec::new();

        match directive {
            ScheduleDirective::Advance { pivot } => {
                queue.push((pivot, NodeStatus::Resolved(Resolution::Finished)));
                self.history.push(pivot);

                // Resolve downstream nodes
                for &ds in &self.graph.adj()[pivot] {
                    self.in_degree[ds] = self.in_degree[ds].saturating_sub(1);
                    if self.in_degree[ds] == 0 {
                        self.finished_count += 1;
                        queue.push((ds, NodeStatus::Started));
                    }
                }
            }
            ScheduleDirective::SelectiveAdvance { pivot, target } => {
                queue.push((pivot, NodeStatus::Resolved(Resolution::Finished)));
                self.history.push(pivot);

                // Topology invariant: select group in-degree is always one due to single pivot
                // hence setting straight to 0 on pivot resolution
                self.in_degree[target] = 0;
                self.finished_count += 1;
                queue.push((target, NodeStatus::Started));

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
                if let Some(current) = self.history.pop() {
                    queue.push((current, NodeStatus::Resolved(Resolution::Reset)));

                    if let Some(&parent) = self.history.last() {
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

                        queue.push((parent, NodeStatus::Started));
                    } else {
                        queue.push((current, NodeStatus::Started));
                    }
                }
            }
        }

        let is_complete = self.in_degree.len() == self.finished_count;

        (is_complete, queue)
    }
}
