use crate::{NodeStatus, Resolution};
use action_orc_core::*;

#[derive(Default)]
pub(crate) struct Schedule {
    pub(crate) in_degree: Vec<usize>,
    pub(crate) finished_count: usize,
}

impl Schedule {
    pub(crate) fn from(graph: &Graph) -> Self {
        let in_degree = graph.in_degree();

        Self {
            in_degree: in_degree.to_vec(),
            finished_count: in_degree.iter().filter(|deg| **deg == 0).count(),
        }
    }

    pub(crate) fn start(&self, graph: &Graph) -> Vec<(NodeId, NodeStatus)> {
        graph
            .sources()
            .map(|source| (source, NodeStatus::Started))
            .collect()
    }

    pub(crate) fn restart(&mut self, graph: &Graph) -> Vec<(NodeId, NodeStatus)> {
        let in_degree = graph.in_degree();
        self.in_degree.copy_from_slice(in_degree);
        self.finished_count = in_degree.iter().filter(|deg| **deg == 0).count();
        self.start(graph)
    }

    pub(crate) fn advance(
        &mut self,
        graph: &Graph,
        target: NodeId,
        resolution: Resolution,
    ) -> (bool, Vec<(NodeId, NodeStatus)>) {
        let mut queue = vec![(target, NodeStatus::Resolved(resolution))];

        let is_complete = self.in_degree.len() == self.finished_count;
        if is_complete {
            return (true, queue);
        }

        // Resolve downstream nodes
        for &ds in &graph.adj()[target] {
            self.in_degree[ds] = self.in_degree[ds].saturating_sub(1);
            if self.in_degree[ds] == 0 {
                self.finished_count += 1;
                queue.push((ds, NodeStatus::Started));
            }
        }

        (false, queue)
    }
}
