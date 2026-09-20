use std::marker::PhantomData;

use crate::{Graph, GraphBuilder, Marker, Meta, NodeId, Regular, graph_bound::Bound};

pub trait IsSubGraph {}

/// Type-Erased at the entry level to allow composite sub-graphs
/// with completely different boundary shapes to be merged into a single parent!
pub enum GraphEntry<'a> {
    Node(Meta),
    BorrowedGraph(&'a dyn AnyGraph),
    OwnedGraph(Box<dyn AnyGraph + 'a>),
}

/// Helper object-safe interface trait to read graph layout metadata
/// during runtime/compile structural composition passes
pub trait AnyGraph: Send + Sync {
    fn meta(&self) -> &[Meta];
    fn adj(&self) -> &[Vec<usize>];
    fn in_degree(&self) -> &[usize];
    fn sources(&self) -> Vec<NodeId>;
    fn sinks(&self) -> Vec<NodeId>;
    fn clone_flat(&self) -> (Vec<Meta>, Vec<Vec<usize>>, Vec<usize>);
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<I: Bound, O: Bound> AnyGraph for Graph<I, O> {
    fn meta(&self) -> &[Meta] {
        self.meta()
    }

    fn adj(&self) -> &[Vec<usize>] {
        self.adj()
    }

    fn in_degree(&self) -> &[usize] {
        self.in_degree()
    }

    fn sources(&self) -> Vec<NodeId> {
        self.sources().collect()
    }

    fn sinks(&self) -> Vec<NodeId> {
        self.sinks().collect()
    }

    fn clone_flat(&self) -> (Vec<Meta>, Vec<Vec<usize>>, Vec<usize>) {
        (self.meta.clone(), self.adj.clone(), self.in_degree.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub trait AsGraphEntry<'a> {
    fn as_entry(this: Self) -> GraphEntry<'a>;
}

pub trait AsGraphEntryProxy<'a>: Send + Sync {
    fn as_entry_proxy(self) -> GraphEntry<'a>;

    fn into_compiled_graph(self) -> Box<dyn AnyGraph + 'a>
    where
        Self: Sized,
    {
        match self.as_entry_proxy() {
            GraphEntry::Node(meta) => {
                let mut builder = GraphBuilder::new();
                builder.append(GraphEntry::Node(meta));
                Box::new(builder.build::<Regular, Regular>())
            }
            GraphEntry::OwnedGraph(g) => g,
            GraphEntry::BorrowedGraph(g) => {
                let (meta, adj, in_degree) = g.clone_flat();
                let mut graph: Graph<Regular, Regular> = Graph::from_meta(meta);
                graph.adj = adj;
                graph.in_degree = in_degree;

                Box::new(graph)
            }
        }
    }
}

/// Leaf node wrapper to provide marker types with default [AsGraphEntry::as_entry] implementation.
pub struct Tag<T>(pub PhantomData<T>);

impl<'a, T> AsGraphEntry<'a> for Tag<T>
where
    T: Marker,
{
    fn as_entry(_this: Self) -> GraphEntry<'a> {
        GraphEntry::Node(T::meta())
    }
}

impl<'a, I, O> AsGraphEntry<'a> for &'a Graph<I, O>
where
    I: Bound,
    O: Bound,
{
    fn as_entry(this: Self) -> GraphEntry<'a> {
        GraphEntry::BorrowedGraph(this)
    }
}

impl<'a, I, O> AsGraphEntry<'a> for Graph<I, O>
where
    I: Bound,
    O: Bound,
{
    fn as_entry(this: Self) -> GraphEntry<'a> {
        GraphEntry::OwnedGraph(Box::new(this))
    }
}

impl<'a, I, O> AsGraphEntryProxy<'a> for &'a Graph<I, O>
where
    I: Bound,
    O: Bound,
{
    fn as_entry_proxy(self) -> GraphEntry<'a> {
        GraphEntry::BorrowedGraph(self)
    }
}

impl<'a, I, O> AsGraphEntryProxy<'a> for Graph<I, O>
where
    I: Bound,
    O: Bound,
{
    fn as_entry_proxy(self) -> GraphEntry<'a> {
        GraphEntry::OwnedGraph(Box::new(self))
    }
}

/// Helper trait for proc-macro to resolve [Tag] into a [GraphEntry::Node]
pub trait AsNodeEntry<'a> {
    fn resolve(self) -> GraphEntry<'a>;
}

impl<'a, T> AsNodeEntry<'a> for &Tag<T>
where
    T: Marker,
{
    #[inline(always)]
    fn resolve(self) -> GraphEntry<'a> {
        AsGraphEntry::as_entry(Tag::<T>(std::marker::PhantomData))
    }
}

/// Helper trait for proc-macro to resolve [Tag] into a [GraphEntry::OwnedGraph]
pub trait AsSubgraphEntry<'a> {
    fn resolve(self) -> GraphEntry<'a>;
}

impl<'a, T> AsSubgraphEntry<'a> for &&Tag<T>
where
    T: IsSubGraph + AsGraphEntryProxy<'a> + Default,
{
    #[inline(always)]
    fn resolve(self) -> GraphEntry<'a> {
        <T as Default>::default().as_entry_proxy()
    }
}

impl<'a> AsGraphEntryProxy<'a> for &'a dyn AnyGraph {
    #[inline(always)]
    fn as_entry_proxy(self) -> GraphEntry<'a> {
        GraphEntry::BorrowedGraph(self)
    }
}
