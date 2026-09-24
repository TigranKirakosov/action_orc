use action_orc_core::*;

struct SyntheticMarker;

pub struct CompileGraph {
    pub(crate) graph: Graph<Join, Join>,
}

impl Default for CompileGraph {
    fn default() -> Self {
        Self {
            graph: Graph::new(),
        }
    }
}

impl CompileGraph {
    pub fn add_synthetic_node(&mut self) -> usize {
        self.graph.add_node::<SyntheticMarker>()
    }

    pub fn add_edge(&mut self, lhs: usize, rhs: usize) {
        self.graph.add_edge(lhs, rhs);
    }

    pub fn sort_ordered(&mut self) -> Result<Vec<NodeId>, GraphError> {
        self.graph.sort_ordered()
    }

    pub fn in_degree(&mut self) -> &[NodeId] {
        self.graph.in_degree()
    }

    pub fn meta(&mut self) -> &[Meta] {
        self.graph.meta()
    }

    pub fn meta_mut(&mut self) -> &mut [Meta] {
        self.graph.meta_mut()
    }
}
