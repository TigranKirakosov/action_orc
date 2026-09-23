use super::*;

#[derive(Default)]
pub struct GraphBuilder {
    pub meta: Vec<Meta>,
    pub edges: Vec<(NodeId, NodeId)>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Appends a typed graph, preserving I/O bounds for compile-time assertions
    pub fn append<I: Bound, O: Bound>(&mut self, g: &Graph<I, O>) -> GraphBounds {
        self.merge_layout(g)
    }

    /// Appends an already-erased layout (e.g., from [IntoGraphLayout::into_graph_layout]).
    ///
    /// Intended for complex expressions like
    /// ```ignore
    /// @Race { a: x, b: y }`
    /// ```
    /// whose type is not [Graph<I, O>]
    pub fn append_graph_layout(&mut self, g: &dyn GraphLayout) -> GraphBounds {
        self.merge_layout(g)
    }

    pub fn append_node<T: Marker>(&mut self) -> GraphBounds {
        let node_id = self.meta.len();
        self.meta.push(Meta::of::<T>());
        GraphBounds {
            sources: vec![node_id],
            sinks: vec![node_id],
        }
    }

    fn merge_layout(&mut self, sub_graph: &dyn GraphLayout) -> GraphBounds {
        let offset = self.meta.len();

        for meta in sub_graph.meta() {
            self.meta.push(meta.clone());
        }

        for (from_local, neighbours) in sub_graph.adj().iter().enumerate() {
            for &to_local in neighbours {
                self.edges.push((from_local + offset, to_local + offset));
            }
        }

        let sources = sub_graph
            .sources()
            .into_iter()
            .map(|id| id + offset)
            .collect();

        let sinks = sub_graph
            .sinks()
            .into_iter()
            .map(|id| id + offset)
            .collect();

        GraphBounds { sources, sinks }
    }

    /// Connects exit points of an upstream to the entry points of a downstream
    pub fn connect(&mut self, upstream: &GraphBounds, downstream: &GraphBounds) {
        for &from in &upstream.sinks {
            for &to in &downstream.sources {
                self.edges.push((from, to));
            }
        }
    }

    pub fn set_upstream_role(&mut self, node_id: &NodeId, role: UpstreamRole) {
        // TODO: move from node role tracking to edge role tracking
        // i.e., store directed roles between connected nodes
        // X -> (A ? B -> (C ? Y)): X::Selector for A, B and B::Selector for C, Y
        let current_us = self.meta[*node_id].role_us;
        if current_us == UpstreamRole::Selector {
            return;
        }
        self.meta[*node_id].set_role_us(role);
    }

    pub fn set_downstream_role(&mut self, node_id: &NodeId, role: DownstreamRole) {
        // TODO: research on how to properly finilize downstream role of a node
        let current_ds = self.meta[*node_id].role_ds;
        if current_ds == DownstreamRole::ForkMember || current_ds == DownstreamRole::SelectionMember
        {
            return;
        }
        self.meta[*node_id].set_role_ds(role);
    }

    /// Generates the finalized topology, binding it to the specified
    /// compiled type-state parameters at the boundary
    pub fn build<I: Bound, O: Bound>(self) -> Graph<I, O> {
        let mut graph = Graph::from_meta(self.meta);

        for (from, to) in self.edges {
            graph.add_edge(from, to);
        }

        graph
    }
}
