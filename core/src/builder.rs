use crate::{
    DownstreamRole, Graph, GraphBounds, GraphEntry, Meta, NodeId, UpstreamRole, graph_bound::Bound,
    graph_entry::AnyGraph,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct GraphBuilder {
    pub meta: Vec<Meta>,
    pub edges: Vec<(NodeId, NodeId)>,
    // Use an erased *const () pointer to provide robust caching keys for trait objects
    pub(crate) graph_cache: HashMap<*const (), GraphBounds>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            edges: Vec::new(),
            graph_cache: HashMap::new(),
        }
    }

    /// Normalizes [GraphEntry] into unified [GraphBounds]
    pub fn append<'a>(&mut self, entry: GraphEntry<'a>) -> GraphBounds {
        match entry {
            GraphEntry::Node(meta) => {
                let node_id = self.meta.len();
                self.meta.push(meta);

                GraphBounds {
                    sources: vec![node_id],
                    sinks: vec![node_id],
                }
            }
            GraphEntry::OwnedGraph(sub_graph) => self.merge_layout(sub_graph.as_ref()),
            GraphEntry::BorrowedGraph(sub_graph) => {
                let ptr = sub_graph.as_any() as *const _ as *const ();
                if let Some(cached_bounds) = self.graph_cache.get(&ptr) {
                    return cached_bounds.clone();
                }

                let bounds = self.merge_layout(sub_graph);
                self.graph_cache.insert(ptr, bounds.clone());

                bounds
            }
        }
    }

    fn merge_layout(&mut self, sub_graph: &dyn AnyGraph) -> GraphBounds {
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
        if current_ds == DownstreamRole::ParallelBranch
            || current_ds == DownstreamRole::SelectionBranch
        {
            return;
        }
        self.meta[*node_id].set_role_ds(role);
    }

    /// Generates the finalized topology, binding it to the specified
    /// compiled type-state parameters at the boundary entrypoint
    pub fn build<I: Bound, O: Bound>(self) -> Graph<I, O> {
        let mut graph = Graph::from_meta(self.meta);

        for (from, to) in self.edges {
            graph.add_edge(from, to);
        }

        graph
    }
}
