use std::collections::HashMap;

use crate::{Graph, GraphBounds, GraphEntry, Meta, NodeId, Role};

pub struct GraphBuilder {
    pub(crate) meta: Vec<Meta>,
    pub(crate) edges: Vec<(NodeId, NodeId)>,
    pub(crate) graph_cache: HashMap<*const Graph, GraphBounds>,
}

impl<'a> GraphBuilder {
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            edges: Vec::new(),
            graph_cache: HashMap::new(),
        }
    }

    /// Normalizes [GraphEntry] into unified [GraphBounds]
    pub fn append(&mut self, entry: GraphEntry<'a>) -> GraphBounds {
        match entry {
            GraphEntry::Node(meta) => {
                let node_id = self.meta.len();
                self.meta.push(meta);

                GraphBounds {
                    sources: vec![node_id],
                    sinks: vec![node_id],
                }
            }
            GraphEntry::OwnedGraph(sub_graph) => self.merge_layout(&sub_graph),
            GraphEntry::BorrowedGraph(sub_graph) => {
                let ptr = sub_graph as *const Graph;
                if let Some(cached_bounds) = self.graph_cache.get(&ptr) {
                    return cached_bounds.clone();
                }

                let bounds = self.merge_layout(sub_graph);
                self.graph_cache.insert(ptr, bounds.clone());

                bounds
            }
        }
    }

    fn merge_layout(&mut self, sub_graph: &Graph) -> GraphBounds {
        let offset = self.meta.len();

        for meta in sub_graph.meta() {
            self.meta.push(meta.clone());
        }

        for (from_local, neighbours) in sub_graph.adj().iter().enumerate() {
            for &to_local in neighbours {
                self.edges.push((from_local + offset, to_local + offset));
            }
        }

        let sources = sub_graph.sources().map(|id| id + offset).collect();
        let sinks = sub_graph.sinks().map(|id| id + offset).collect();

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

    pub fn set_topology_role(&mut self, node_id: &NodeId, role: Role) {
        self.meta[*node_id].role = role;
    }

    pub fn build(self) -> Graph {
        let mut graph = Graph::from_meta(self.meta);

        for (from, to) in self.edges {
            graph.add_edge(from, to);
        }

        graph
    }
}
