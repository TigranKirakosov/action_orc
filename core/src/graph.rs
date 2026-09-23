use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
};

use crate::{
    bound::Bound,
    meta::{Marker, Meta},
};

pub type NodeId = usize;

#[derive(Debug, PartialEq)]
pub enum GraphError {
    CycleDetected,
}

pub struct Graph<I: Bound, O: Bound> {
    pub(crate) in_degree: Vec<usize>,
    pub(crate) adj: Vec<Vec<NodeId>>,
    pub(crate) meta: Vec<Meta>,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

#[derive(Clone)]
pub struct GraphBounds {
    pub sources: Vec<NodeId>,
    pub sinks: Vec<NodeId>,
}

impl<I: Bound, O: Bound> Graph<I, O> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_meta(meta: Vec<Meta>) -> Self {
        Self {
            adj: vec![vec![]; meta.len()],
            in_degree: vec![0; meta.len()],
            meta,
            _i: PhantomData,
            _o: PhantomData,
        }
    }

    pub fn add_node<T: Marker>(&mut self) -> NodeId {
        self.meta.push(Meta::of::<T>());
        self.adj.push(vec![]);
        self.in_degree.push(0);

        self.adj.len() - 1
    }

    /// Adds a dependency `a -> b` and increments `b`'s dependants count.\
    /// Skips adding duplicate edges.
    pub fn add_edge(&mut self, lhs: NodeId, rhs: NodeId) {
        if self.adj[lhs].contains(&rhs) {
            return;
        }

        self.adj[lhs].push(rhs);
        self.in_degree[rhs] += 1;
    }

    /// Merges sub-[Graph] into this [Graph] returning shifted sub [GraphBounds]
    ///
    /// ### Single entry
    /// Merging sub-graph **H** (`x -> y`) into graph **G** (`a -> b`) at **G**(`a`):
    /// ```text
    /// [a] ──> [x] ──> [y] ──> [b]
    /// ```
    ///
    /// ### Multiple entries
    /// Merging sub-graph **H** (`x`) into graph **G** (`a | b -> c`) at **G**(`a | b`):
    /// ```text
    /// [a] ──┐
    ///       ├──> [x] ──> [c]
    /// [b] ──┘
    /// ```
    pub fn merge<SubI: Bound, SubO: Bound>(
        &mut self,
        sub: &Graph<SubI, SubO>,
        at: Vec<NodeId>,
    ) -> GraphBounds {
        // Collect unique downstream neighbours of each node of `at` list
        // while counting broken edges
        let mut at_downstream = HashMap::new();
        for &node in &at {
            let neighbours = std::mem::take(&mut self.adj[node]);
            for nbr in neighbours {
                *at_downstream.entry(nbr).or_insert(0) += 1;
            }
        }

        // Decrease in-degree for each collected neighbour by broken edges count
        for (&nbr, &count) in &at_downstream {
            self.in_degree[nbr] = self.in_degree[nbr].saturating_sub(count);
        }

        // Offset source and sink indices of sub
        // so they stand right after last node of this graph
        let offset = self.adj.len();
        let sub_sources: Vec<NodeId> = sub.sources().map(|id| id + offset).collect();
        let sub_sinks: Vec<NodeId> = sub.sinks().map(|id| id + offset).collect();

        // Extend with sub vectors
        self.meta.extend(sub.meta.clone());
        self.in_degree.extend(sub.in_degree.clone());

        for downstream in &sub.adj {
            // Offset every downstream node index aswell
            let mut shifted_downstream = downstream.clone();
            for node in &mut shifted_downstream {
                *node += offset;
            }

            self.adj.push(shifted_downstream);
        }

        // Stitch at nodes with sub's source nodes
        for &node in &at {
            for &s_source in &sub_sources {
                self.adj[node].push(s_source);
                self.in_degree[s_source] += 1;
            }
        }

        // Stitch sub's sink nodes with downstream neighbors of at nodes
        // while restoring their in-degrees
        for &s_sink in &sub_sinks {
            for &nbr in at_downstream.keys() {
                self.adj[s_sink].push(nbr);
                self.in_degree[nbr] += 1;
            }
        }

        GraphBounds {
            sources: sub_sources,
            sinks: sub_sinks,
        }
    }

    /// Kahn's topological sort
    pub fn sort_ordered(&self) -> Result<Vec<NodeId>, GraphError> {
        let mut order = Vec::new();
        let mut in_deg = self.in_degree.clone();
        let mut q = VecDeque::<NodeId>::from(self.sources().collect::<Vec<_>>());

        while let Some(id) = q.pop_front() {
            order.push(id);
            for &nbr in &self.adj[id] {
                in_deg[nbr] = in_deg[nbr].saturating_sub(1);
                if in_deg[nbr] == 0 {
                    q.push_back(nbr);
                }
            }
        }

        if order.len() != in_deg.len() {
            return Err(GraphError::CycleDetected);
        }

        Ok(order)
    }

    pub fn in_degree(&self) -> &[usize] {
        self.in_degree.as_slice()
    }

    pub fn adj(&self) -> &[Vec<NodeId>] {
        self.adj.as_slice()
    }

    pub fn meta(&self) -> &[Meta] {
        self.meta.as_slice()
    }

    pub fn meta_mut(&mut self) -> &mut [Meta] {
        &mut self.meta
    }

    pub fn sources(&self) -> impl Iterator<Item = NodeId> {
        let len = self.adj.len();
        (0..len).filter(|&id| self.in_degree[id] == 0)
    }

    pub fn sinks(&self) -> impl Iterator<Item = NodeId> {
        let len = self.adj.len();
        (0..len).filter(|&id| self.adj[id].is_empty())
    }
}

impl<I: Bound, O: Bound> Default for Graph<I, O> {
    fn default() -> Self {
        Self {
            in_degree: vec![],
            adj: vec![],
            meta: vec![],
            _i: PhantomData,
            _o: PhantomData,
        }
    }
}

impl<I: Bound, O: Bound> Clone for Graph<I, O> {
    fn clone(&self) -> Self {
        Self {
            in_degree: self.in_degree.clone(),
            adj: self.adj.clone(),
            meta: self.meta.clone(),
            _i: PhantomData,
            _o: PhantomData,
        }
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph Error: ")?;
        match self {
            GraphError::CycleDetected => write!(f, "Detected cycle."),
        }
    }
}

impl std::error::Error for GraphError {}
